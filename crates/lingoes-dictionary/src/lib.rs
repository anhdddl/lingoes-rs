use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use lingoes_core::{DictionaryEntry, LookupQuery};
use opendict::{Dictionary, mdict::MdictDictionary, stardict::StarDictDictionary};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use thiserror::Error;

pub struct SqliteDictionary {
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryMeta {
    pub id: i64,
    pub name: String,
    pub format: String,
    pub source_path: Option<String>,
    pub enabled: bool,
    pub entry_count: usize,
    pub created_at: String,
}

#[derive(Debug, Error)]
pub enum DictionaryError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("stored dictionary entry is invalid")]
    InvalidStoredEntry,
    #[error("dictionary with id {0} not found")]
    NotFound(i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ImportReport {
    pub imported: usize,
    pub inserted: usize,
    pub updated: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionaryFormat {
    Tsv,
    StarDict,
    Mdict,
}

impl DictionaryFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tsv => "tsv",
            Self::StarDict => "stardict",
            Self::Mdict => "mdict",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportProgress {
    pub processed: usize,
    pub total: Option<usize>,
}

pub fn detect_format(path: &Path) -> Result<DictionaryFormat, ImportError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if file_name.ends_with(".tsv") || file_name.ends_with(".txt") {
        Ok(DictionaryFormat::Tsv)
    } else if file_name.ends_with(".ifo")
        || file_name.ends_with(".idx")
        || file_name.ends_with(".dict")
        || file_name.ends_with(".dict.dz")
    {
        Ok(DictionaryFormat::StarDict)
    } else if file_name.ends_with(".mdx") {
        Ok(DictionaryFormat::Mdict)
    } else {
        Err(ImportError::UnsupportedFormat {
            path: path.display().to_string(),
        })
    }
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("unable to read dictionary: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported dictionary format: {path}")]
    UnsupportedFormat { path: String },
    #[error("invalid or unsupported dictionary: {0}")]
    Format(String),
    #[error("dictionary contains too many entries (maximum {maximum})")]
    TooManyEntries { maximum: usize },
    #[error("dictionary entry is too large (maximum {maximum} bytes)")]
    EntryTooLarge { maximum: usize },
    #[error("dictionary entry is not valid UTF-8 text")]
    InvalidText,
    #[error("line {line} must contain a tab-separated headword and definition")]
    MalformedLine { line: usize },
    #[error("line {line} contains an invalid entry")]
    InvalidEntry { line: usize },
    #[error(transparent)]
    Dictionary(#[from] DictionaryError),
}

impl From<rusqlite::Error> for ImportError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Dictionary(DictionaryError::Database(error))
    }
}

impl SqliteDictionary {
    pub fn in_memory() -> Result<Self, DictionaryError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::from_connection(Connection::open(path)?)
    }

    fn from_connection(connection: Connection) -> Result<Self, DictionaryError> {
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrate_or_init_schema(&connection)?;
        Ok(Self { connection })
    }

    pub fn list_dictionaries(&self) -> Result<Vec<DictionaryMeta>, DictionaryError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, format, source_path, enabled, entry_count, created_at
             FROM dictionaries
             ORDER BY id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(DictionaryMeta {
                id: row.get(0)?,
                name: row.get(1)?,
                format: row.get(2)?,
                source_path: row.get(3)?,
                enabled: row.get(4)?,
                entry_count: row.get::<_, i64>(5)? as usize,
                created_at: row.get(6)?,
            })
        })?;
        let mut dictionaries = Vec::new();
        for row in rows {
            dictionaries.push(row?);
        }
        Ok(dictionaries)
    }

    pub fn toggle_dictionary(&mut self, id: i64, enabled: bool) -> Result<(), DictionaryError> {
        let rows_affected = self.connection.execute(
            "UPDATE dictionaries SET enabled = ?1 WHERE id = ?2",
            params![enabled, id],
        )?;
        if rows_affected == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        Ok(())
    }

    pub fn delete_dictionary(&mut self, id: i64) -> Result<(), DictionaryError> {
        let rows_affected = self
            .connection
            .execute("DELETE FROM dictionaries WHERE id = ?1", params![id])?;
        if rows_affected == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        self.connection
            .execute("DELETE FROM entries WHERE dictionary_id = ?1", params![id])?;
        Ok(())
    }

    pub fn upsert(&mut self, entry: &DictionaryEntry) -> Result<(), DictionaryError> {
        let dict_id = match entry.dictionary_id() {
            Some(id) => id,
            None => ensure_default_dictionary(&self.connection)?,
        };
        upsert_entry_connection(&self.connection, dict_id, entry)?;
        self.connection.execute(
            "UPDATE dictionaries SET entry_count = (SELECT count(*) FROM entries WHERE dictionary_id = ?1) WHERE id = ?1",
            params![dict_id],
        )?;
        Ok(())
    }

    pub fn import_tsv(&mut self, reader: impl Read) -> Result<ImportReport, ImportError> {
        self.import_tsv_with_name(reader, None)
    }

    pub fn import_tsv_with_name(
        &mut self,
        reader: impl Read,
        name: Option<&str>,
    ) -> Result<ImportReport, ImportError> {
        self.import_tsv_with_progress_and_name(reader, name, |_| {})
    }

    pub fn import_tsv_with_progress(
        &mut self,
        reader: impl Read,
        on_progress: impl FnMut(ImportProgress),
    ) -> Result<ImportReport, ImportError> {
        self.import_tsv_with_progress_and_name(reader, None, on_progress)
    }

    pub fn import_tsv_with_progress_and_name(
        &mut self,
        reader: impl Read,
        name: Option<&str>,
        mut on_progress: impl FnMut(ImportProgress),
    ) -> Result<ImportReport, ImportError> {
        let transaction = self
            .connection
            .transaction()
            .map_err(DictionaryError::from)?;
        let dictionary_id = match name {
            Some(dict_name) if !dict_name.trim().is_empty() => {
                transaction.execute(
                    "INSERT INTO dictionaries (name, format, source_path, enabled, entry_count, created_at)
                     VALUES (?1, 'tsv', NULL, 1, 0, datetime('now'))",
                    params![dict_name.trim()],
                )?;
                transaction.last_insert_rowid()
            }
            _ => ensure_default_dictionary(&transaction)?,
        };
        let mut report = ImportReport::default();
        for (index, line) in BufReader::new(reader).lines().enumerate() {
            let line_number = index + 1;
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (headword, definition) = line
                .split_once('\t')
                .ok_or(ImportError::MalformedLine { line: line_number })?;
            let entry = DictionaryEntry::new(headword, definition)
                .map_err(|_| ImportError::InvalidEntry { line: line_number })?;
            add_entry(&transaction, dictionary_id, &entry, &mut report)?;
            on_progress(ImportProgress {
                processed: report.imported,
                total: None,
            });
        }
        transaction.execute(
            "UPDATE dictionaries SET entry_count = (SELECT count(*) FROM entries WHERE dictionary_id = ?1) WHERE id = ?1",
            params![dictionary_id],
        )?;
        transaction.commit().map_err(DictionaryError::from)?;
        Ok(report)
    }

    pub fn import_path(&mut self, path: impl AsRef<Path>) -> Result<ImportReport, ImportError> {
        self.import_path_with_name(path, None)
    }

    pub fn import_path_with_name(
        &mut self,
        source_path: impl AsRef<Path>,
        name: Option<&str>,
    ) -> Result<ImportReport, ImportError> {
        self.import_path_with_progress_and_name(source_path, name, |_| {})
    }

    pub fn import_path_with_progress(
        &mut self,
        path: impl AsRef<Path>,
        on_progress: impl FnMut(ImportProgress),
    ) -> Result<ImportReport, ImportError> {
        self.import_path_with_progress_and_name(path, None, on_progress)
    }

    pub fn import_path_with_progress_and_name(
        &mut self,
        path: impl AsRef<Path>,
        name: Option<&str>,
        mut on_progress: impl FnMut(ImportProgress),
    ) -> Result<ImportReport, ImportError> {
        const MAX_ENTRIES: usize = 5_000_000;
        const MAX_ENTRY_BYTES: usize = 16 * 1024 * 1024;

        let path = path.as_ref();
        let format = detect_format(path)?;
        if format == DictionaryFormat::Tsv {
            return self.import_tsv_with_progress_and_name(
                std::fs::File::open(path)?,
                name,
                on_progress,
            );
        }

        let dict_name = match name {
            Some(n) if !n.trim().is_empty() => n.trim().to_owned(),
            _ => path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Dictionary")
                .to_owned(),
        };
        let source_str = path.display().to_string();

        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let dictionary: Box<dyn Dictionary> = match format {
            DictionaryFormat::StarDict => {
                let stem = stardict_stem(path).ok_or_else(|| ImportError::UnsupportedFormat {
                    path: path.display().to_string(),
                })?;
                Box::new(
                    StarDictDictionary::open(parent, &stem)
                        .map_err(|error| ImportError::Format(error.to_string()))?,
                )
            }
            DictionaryFormat::Mdict => Box::new(
                MdictDictionary::open(parent)
                    .map_err(|error| ImportError::Format(error.to_string()))?,
            ),
            DictionaryFormat::Tsv => unreachable!(),
        };
        let total = dictionary.word_count();
        if total > MAX_ENTRIES {
            return Err(ImportError::TooManyEntries {
                maximum: MAX_ENTRIES,
            });
        }
        on_progress(ImportProgress {
            processed: 0,
            total: Some(total),
        });
        let transaction = self
            .connection
            .transaction()
            .map_err(DictionaryError::from)?;
        transaction.execute(
            "INSERT INTO dictionaries (name, format, source_path, enabled, entry_count, created_at)
             VALUES (?1, ?2, ?3, 1, 0, datetime('now'))",
            params![dict_name, format.as_str(), source_str],
        )?;
        let dictionary_id = transaction.last_insert_rowid();

        let mut report = ImportReport::default();
        for word in dictionary.word_list() {
            let records = dictionary
                .lookup(word)
                .map_err(|error| ImportError::Format(error.to_string()))?
                .unwrap_or_default();
            let mut definition = String::new();
            for record in records {
                if record.data.len() > MAX_ENTRY_BYTES {
                    return Err(ImportError::EntryTooLarge {
                        maximum: MAX_ENTRY_BYTES,
                    });
                }
                let text = std::str::from_utf8(&record.data)
                    .map_err(|_| ImportError::InvalidText)?
                    .trim_matches('\0')
                    .trim();
                if !text.is_empty() {
                    if !definition.is_empty() {
                        definition.push_str("\n\n");
                    }
                    definition.push_str(text);
                }
            }
            let entry = DictionaryEntry::new(word, definition)
                .map_err(|_| ImportError::Format(format!("invalid entry for {word:?}")))?;
            add_entry(&transaction, dictionary_id, &entry, &mut report)?;
            on_progress(ImportProgress {
                processed: report.imported,
                total: Some(total),
            });
        }
        transaction.execute(
            "UPDATE dictionaries SET entry_count = (SELECT count(*) FROM entries WHERE dictionary_id = ?1) WHERE id = ?1",
            params![dictionary_id],
        )?;
        transaction.commit().map_err(DictionaryError::from)?;
        Ok(report)
    }

    pub fn lookup_exact(
        &self,
        query: &LookupQuery,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        self.query_entries(
            "SELECT e.headword, e.definition, e.dictionary_id, d.name
             FROM entries e
             JOIN dictionaries d ON d.id = e.dictionary_id
             WHERE e.normalized_headword = ?1 AND d.enabled = 1
             ORDER BY d.id, e.headword",
            params![query.as_str()],
        )
    }

    pub fn lookup_prefix(
        &self,
        query: &LookupQuery,
        limit: usize,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let upper_bound = format!("{}{}", query.as_str(), char::MAX);
        self.query_entries(
            "SELECT e.headword, e.definition, e.dictionary_id, d.name
             FROM entries e
             JOIN dictionaries d ON d.id = e.dictionary_id
             WHERE e.normalized_headword >= ?1 AND e.normalized_headword < ?2 AND d.enabled = 1
             ORDER BY CASE WHEN e.normalized_headword = ?1 THEN 0 ELSE 1 END,
                      e.normalized_headword,
                      d.id
             LIMIT ?3",
            params![query.as_str(), upper_bound, limit],
        )
    }

    pub fn lookup_full_text(
        &self,
        query: &LookupQuery,
        limit: usize,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let fts_query = format!("\"{}\"", query.as_str().replace('"', "\"\""));
        self.query_entries(
            "SELECT e.headword, e.definition, e.dictionary_id, d.name
             FROM entries_fts f
             JOIN entries e ON e.id = f.rowid
             JOIN dictionaries d ON d.id = e.dictionary_id
             WHERE entries_fts MATCH ?1 AND d.enabled = 1
             ORDER BY bm25(entries_fts), e.normalized_headword, d.id
             LIMIT ?2",
            params![fts_query, limit],
        )
    }

    fn query_entries<P: rusqlite::Params>(
        &self,
        sql: &str,
        parameters: P,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map(parameters, map_entry)?;
        rows.map(|row| {
            let (headword, definition, dict_id, dict_name) = row?;
            DictionaryEntry::with_dictionary(headword, definition, dict_id, dict_name)
                .map_err(|_| DictionaryError::InvalidStoredEntry)
        })
        .collect()
    }
}

fn migrate_or_init_schema(connection: &Connection) -> Result<(), DictionaryError> {
    let entries_exists: bool = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'entries'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;

    let dictionaries_exists: bool = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'dictionaries'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;

    if entries_exists && !dictionaries_exists {
        let mut stmt = connection.prepare("PRAGMA table_info(entries)")?;
        let columns = stmt.query_map([], |r| r.get::<_, String>(1))?;
        let mut has_dict_id = false;
        for col in columns {
            if col? == "dictionary_id" {
                has_dict_id = true;
                break;
            }
        }
        if !has_dict_id {
            connection.execute_batch(
                "CREATE TABLE dictionaries (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    format TEXT NOT NULL,
                    source_path TEXT,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    entry_count INTEGER NOT NULL DEFAULT 0,
                    created_at TEXT NOT NULL
                );
                INSERT INTO dictionaries (id, name, format, source_path, enabled, entry_count, created_at)
                VALUES (1, 'Default Dictionary', 'tsv', NULL, 1, (SELECT count(*) FROM entries), datetime('now'));

                DROP TRIGGER IF EXISTS entries_fts_insert;
                DROP TRIGGER IF EXISTS entries_fts_delete;
                DROP TRIGGER IF EXISTS entries_fts_update;
                DROP TABLE IF EXISTS entries_fts;

                CREATE TABLE entries_v2 (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    dictionary_id INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
                    normalized_headword TEXT NOT NULL,
                    headword TEXT NOT NULL,
                    definition TEXT NOT NULL,
                    UNIQUE(dictionary_id, normalized_headword)
                );
                INSERT INTO entries_v2 (dictionary_id, normalized_headword, headword, definition)
                SELECT 1, normalized_headword, headword, definition FROM entries;

                DROP TABLE entries;
                ALTER TABLE entries_v2 RENAME TO entries;

                CREATE INDEX IF NOT EXISTS entries_search_idx ON entries(normalized_headword, dictionary_id);
                CREATE INDEX IF NOT EXISTS entries_dict_idx ON entries(dictionary_id);

                CREATE VIRTUAL TABLE entries_fts USING fts5(
                    headword,
                    definition,
                    tokenize = 'unicode61 remove_diacritics 0'
                );
                CREATE TRIGGER entries_fts_insert AFTER INSERT ON entries BEGIN
                    INSERT INTO entries_fts(rowid, headword, definition)
                    VALUES (new.id, new.headword, new.definition);
                END;
                CREATE TRIGGER entries_fts_delete AFTER DELETE ON entries BEGIN
                    DELETE FROM entries_fts WHERE rowid = old.id;
                END;
                CREATE TRIGGER entries_fts_update AFTER UPDATE ON entries BEGIN
                    DELETE FROM entries_fts WHERE rowid = old.id;
                    INSERT INTO entries_fts(rowid, headword, definition)
                    VALUES (new.id, new.headword, new.definition);
                END;
                INSERT INTO entries_fts(rowid, headword, definition)
                SELECT id, headword, definition FROM entries;",
            )?;
        }
    } else {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS dictionaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                format TEXT NOT NULL,
                source_path TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                entry_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dictionary_id INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
                normalized_headword TEXT NOT NULL,
                headword TEXT NOT NULL,
                definition TEXT NOT NULL,
                UNIQUE(dictionary_id, normalized_headword)
            );
            CREATE INDEX IF NOT EXISTS entries_search_idx ON entries(normalized_headword, dictionary_id);
            CREATE INDEX IF NOT EXISTS entries_dict_idx ON entries(dictionary_id);
            CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
                headword,
                definition,
                tokenize = 'unicode61 remove_diacritics 0'
            );
            CREATE TRIGGER IF NOT EXISTS entries_fts_insert AFTER INSERT ON entries BEGIN
                INSERT INTO entries_fts(rowid, headword, definition)
                VALUES (new.id, new.headword, new.definition);
            END;
            CREATE TRIGGER IF NOT EXISTS entries_fts_delete AFTER DELETE ON entries BEGIN
                DELETE FROM entries_fts WHERE rowid = old.id;
            END;
            CREATE TRIGGER IF NOT EXISTS entries_fts_update AFTER UPDATE ON entries BEGIN
                DELETE FROM entries_fts WHERE rowid = old.id;
                INSERT INTO entries_fts(rowid, headword, definition)
                VALUES (new.id, new.headword, new.definition);
            END;",
        )?;
    }
    Ok(())
}

fn ensure_default_dictionary(connection: &Connection) -> rusqlite::Result<i64> {
    if let Some(id) = connection
        .query_row(
            "SELECT id FROM dictionaries ORDER BY id ASC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .optional()?
    {
        Ok(id)
    } else {
        connection.execute(
            "INSERT INTO dictionaries (name, format, source_path, enabled, entry_count, created_at)
             VALUES ('Default Dictionary', 'tsv', NULL, 1, 0, datetime('now'))",
            [],
        )?;
        Ok(connection.last_insert_rowid())
    }
}

fn upsert_entry_connection(
    connection: &Connection,
    dictionary_id: i64,
    entry: &DictionaryEntry,
) -> rusqlite::Result<usize> {
    connection.execute(
        "INSERT INTO entries (dictionary_id, normalized_headword, headword, definition)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(dictionary_id, normalized_headword) DO UPDATE SET
            headword = excluded.headword,
            definition = excluded.definition",
        params![
            dictionary_id,
            entry.normalized_headword(),
            entry.headword(),
            entry.definition()
        ],
    )
}

fn add_entry(
    transaction: &Transaction<'_>,
    dictionary_id: i64,
    entry: &DictionaryEntry,
    report: &mut ImportReport,
) -> Result<(), DictionaryError> {
    let exists = transaction
        .query_row(
            "SELECT 1 FROM entries WHERE dictionary_id = ?1 AND normalized_headword = ?2",
            params![dictionary_id, entry.normalized_headword()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    upsert_entry_connection(transaction, dictionary_id, entry)?;
    report.imported += 1;
    if exists {
        report.updated += 1;
    } else {
        report.inserted += 1;
    }
    Ok(())
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, String, i64, String)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn stardict_stem(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    for suffix in [".dict.dz", ".dict", ".idx", ".ifo"] {
        if name.to_ascii_lowercase().ends_with(suffix) {
            return Some(name[..name.len() - suffix.len()].to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use lingoes_core::{DictionaryEntry, LookupQuery};

    #[test]
    fn exact_lookup_is_case_insensitive() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("Rust", "A systems language").unwrap())
            .unwrap();
        let results = dictionary
            .lookup_exact(&LookupQuery::new("RUST").unwrap())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].headword(), "Rust");
    }

    #[test]
    fn prefix_lookup_orders_exact_match_first() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        for (word, definition) in [
            ("rustacean", "A Rust programmer"),
            ("rust", "A systems language"),
            ("rustic", "Simple or rural"),
        ] {
            dictionary
                .upsert(&DictionaryEntry::new(word, definition).unwrap())
                .unwrap();
        }
        let results = dictionary
            .lookup_prefix(&LookupQuery::new("rust").unwrap(), 10)
            .unwrap();
        let words: Vec<_> = results.iter().map(DictionaryEntry::headword).collect();
        assert_eq!(words, ["rust", "rustacean", "rustic"]);
    }

    #[test]
    fn upsert_replaces_definition_for_same_normalized_headword() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("Rust", "old").unwrap())
            .unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("RUST", "new").unwrap())
            .unwrap();
        let results = dictionary
            .lookup_exact(&LookupQuery::new("rust").unwrap())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].headword(), "RUST");
        assert_eq!(results[0].definition(), "new");
    }

    #[test]
    fn zero_limit_returns_no_prefix_results() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("rust", "definition").unwrap())
            .unwrap();
        let results = dictionary
            .lookup_prefix(&LookupQuery::new("r").unwrap(), 0)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn tsv_import_skips_comments_and_blank_lines() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let input = "# English-Vietnamese\n\nhello\txin chào\nrust\tngôn ngữ Rust\n";
        let report = dictionary.import_tsv(input.as_bytes()).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(
            dictionary
                .lookup_exact(&LookupQuery::new("hello").unwrap())
                .unwrap()[0]
                .definition(),
            "xin chào"
        );
    }

    #[test]
    fn tsv_import_reports_malformed_line_number() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let error = dictionary
            .import_tsv("hello\txin chào\nmissing-tab\n".as_bytes())
            .unwrap_err();
        assert!(matches!(error, ImportError::MalformedLine { line: 2 }));
    }

    #[test]
    fn unicode_normalized_lookup_matches_vietnamese_decomposed_query() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("Tiếng Việt", "Vietnamese").unwrap())
            .unwrap();
        let result = dictionary
            .lookup_exact(&LookupQuery::new("tie\u{302}\u{301}ng vie\u{323}\u{302}t").unwrap())
            .unwrap();
        assert_eq!(result[0].headword(), "Tiếng Việt");
    }

    #[test]
    fn full_text_lookup_finds_definition_terms() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("xin chào", "a Vietnamese greeting").unwrap())
            .unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("tạm biệt", "a Vietnamese farewell").unwrap())
            .unwrap();
        let results = dictionary
            .lookup_full_text(&LookupQuery::new("greeting").unwrap(), 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].headword(), "xin chào");
    }

    #[test]
    fn failed_tsv_import_is_atomic() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let error = dictionary
            .import_tsv("valid\tdefinition\nmalformed\n".as_bytes())
            .unwrap_err();
        assert!(matches!(error, ImportError::MalformedLine { line: 2 }));
        assert!(
            dictionary
                .lookup_exact(&LookupQuery::new("valid").unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn import_report_counts_insertions_and_duplicates() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let report = dictionary
            .import_tsv("Rust\told\nRUST\tnew\nFerris\tmascot\n".as_bytes())
            .unwrap();
        assert_eq!(report.imported, 3);
        assert_eq!(report.inserted, 2);
        assert_eq!(report.updated, 1);
    }

    fn temporary_directory(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "lingoes-dictionary-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_stardict_fixture(directory: &Path) -> std::path::PathBuf {
        let definitions = b"a systems languagea Vietnamese greeting";
        let first_size = "a systems language".len() as u32;
        let mut index = Vec::new();
        index.extend_from_slice(b"Rust\0");
        index.extend_from_slice(&0_u32.to_be_bytes());
        index.extend_from_slice(&first_size.to_be_bytes());
        index.extend_from_slice(b"xin chao\0");
        index.extend_from_slice(&first_size.to_be_bytes());
        index.extend_from_slice(&("a Vietnamese greeting".len() as u32).to_be_bytes());
        let ifo = format!(
            "StarDict's dict ifo file\nversion=2.4.2\nwordcount=2\nidxfilesize={}\nbookname=Fixture\nsametypesequence=m\n",
            index.len()
        );
        let ifo_path = directory.join("fixture.ifo");
        std::fs::write(&ifo_path, ifo).unwrap();
        std::fs::write(directory.join("fixture.idx"), index).unwrap();
        std::fs::write(directory.join("fixture.dict"), definitions).unwrap();
        ifo_path
    }

    #[test]
    fn auto_detects_and_imports_stardict_files() {
        let directory = temporary_directory("stardict");
        let source = write_stardict_fixture(&directory);
        let mut dictionary = SqliteDictionary::in_memory().unwrap();

        assert_eq!(detect_format(&source).unwrap(), DictionaryFormat::StarDict);
        let report = dictionary.import_path(&source).unwrap();

        assert_eq!(report.imported, 2);
        assert_eq!(
            dictionary
                .lookup_full_text(&LookupQuery::new("greeting").unwrap(), 10)
                .unwrap()[0]
                .headword(),
            "xin chao"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_stardict_is_rejected_without_partial_import() {
        let directory = temporary_directory("bad-stardict");
        let source = write_stardict_fixture(&directory);
        std::fs::write(directory.join("fixture.idx"), b"truncated").unwrap();
        let mut dictionary = SqliteDictionary::in_memory().unwrap();

        assert!(dictionary.import_path(&source).is_err());
        assert!(
            dictionary
                .lookup_prefix(&LookupQuery::new("r").unwrap(), 10)
                .unwrap()
                .is_empty()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unsupported_extension_has_a_clear_error() {
        let error = detect_format(Path::new("dictionary.zip")).unwrap_err();
        assert!(matches!(error, ImportError::UnsupportedFormat { .. }));
    }

    fn wrap_mdict_block(data: &[u8]) -> Vec<u8> {
        use flate2::{Compression, write::ZlibEncoder};
        use std::io::Write;

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        let compressed = encoder.finish().unwrap();
        let mut block = 2_u32.to_le_bytes().to_vec();
        block.extend_from_slice(&adler2::adler32_slice(data).to_be_bytes());
        block.extend_from_slice(&compressed);
        block
    }

    fn write_mdx_fixture(directory: &Path) -> std::path::PathBuf {
        let entries = [("hello", "xin chào"), ("rust", "systems language")];
        let mut records = Vec::new();
        let mut offsets = Vec::new();
        for (_, definition) in entries {
            offsets.push(records.len() as u64);
            records.extend_from_slice(definition.as_bytes());
            records.push(0);
        }
        let mut keys = Vec::new();
        for ((word, _), offset) in entries.iter().zip(offsets) {
            keys.extend_from_slice(&offset.to_be_bytes());
            keys.extend_from_slice(word.as_bytes());
            keys.push(0);
        }
        let key_block = wrap_mdict_block(&keys);
        let mut key_index = (entries.len() as u64).to_be_bytes().to_vec();
        for word in [entries[0].0, entries[1].0] {
            key_index.extend_from_slice(&(word.len() as u16).to_be_bytes());
            key_index.extend_from_slice(word.as_bytes());
            key_index.push(0);
        }
        key_index.extend_from_slice(&(key_block.len() as u64).to_be_bytes());
        key_index.extend_from_slice(&(keys.len() as u64).to_be_bytes());
        let key_index_block = wrap_mdict_block(&key_index);
        let mut keyword_header = 1_u64.to_be_bytes().to_vec();
        keyword_header.extend_from_slice(&(entries.len() as u64).to_be_bytes());
        keyword_header.extend_from_slice(&(key_index.len() as u64).to_be_bytes());
        keyword_header.extend_from_slice(&(key_index_block.len() as u64).to_be_bytes());
        keyword_header.extend_from_slice(&(key_block.len() as u64).to_be_bytes());
        keyword_header.extend_from_slice(&adler2::adler32_slice(&keyword_header).to_be_bytes());

        let record_block = wrap_mdict_block(&records);
        let mut record_header = 1_u64.to_be_bytes().to_vec();
        record_header.extend_from_slice(&(entries.len() as u64).to_be_bytes());
        record_header.extend_from_slice(&16_u64.to_be_bytes());
        record_header.extend_from_slice(&(record_block.len() as u64).to_be_bytes());
        let mut record_index = (record_block.len() as u64).to_be_bytes().to_vec();
        record_index.extend_from_slice(&(records.len() as u64).to_be_bytes());

        let xml = r#"<Dict GeneratedByEngineVersion="2.0" Encoding="UTF-8" Format="Html" Title="Fixture">"#;
        let xml: Vec<u8> = xml.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut file = (xml.len() as u32).to_be_bytes().to_vec();
        file.extend_from_slice(&xml);
        file.extend_from_slice(&adler2::adler32_slice(&xml).to_be_bytes());
        file.extend_from_slice(&keyword_header);
        file.extend_from_slice(&key_index_block);
        file.extend_from_slice(&key_block);
        file.extend_from_slice(&record_header);
        file.extend_from_slice(&record_index);
        file.extend_from_slice(&record_block);
        let path = directory.join("fixture.mdx");
        std::fs::write(&path, file).unwrap();
        path
    }

    #[test]
    fn imports_fixture_backed_mdict_v2() {
        let directory = temporary_directory("mdict");
        let source = write_mdx_fixture(&directory);
        let mut dictionary = SqliteDictionary::in_memory().unwrap();

        assert_eq!(detect_format(&source).unwrap(), DictionaryFormat::Mdict);
        let report = dictionary.import_path(&source).unwrap();

        assert_eq!(report.imported, 2);
        assert_eq!(
            dictionary
                .lookup_exact(&LookupQuery::new("hello").unwrap())
                .unwrap()[0]
                .definition(),
            "xin chào"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_mdict_is_rejected_without_partial_import() {
        let directory = temporary_directory("bad-mdict");
        let source = directory.join("broken.mdx");
        std::fs::write(&source, b"not an mdict").unwrap();
        let mut dictionary = SqliteDictionary::in_memory().unwrap();

        assert!(dictionary.import_path(&source).is_err());
        assert!(
            dictionary
                .lookup_prefix(&LookupQuery::new("a").unwrap(), 10)
                .unwrap()
                .is_empty()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn bulk_import_reports_progress() {
        let directory = temporary_directory("progress");
        let source = write_stardict_fixture(&directory);
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let mut progress = Vec::new();

        dictionary
            .import_path_with_progress(&source, |update| progress.push(update))
            .unwrap();

        assert_eq!(progress.last().unwrap().processed, 2);
        assert_eq!(progress.last().unwrap().total, Some(2));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_entries_and_full_text_index_persist() {
        let directory = temporary_directory("persistence");
        let database_path = directory.join("dictionary.db");
        {
            let mut dictionary = SqliteDictionary::open(&database_path).unwrap();
            dictionary
                .import_tsv("Tiếng Việt\ta Vietnamese language\n".as_bytes())
                .unwrap();
        }

        let dictionary = SqliteDictionary::open(&database_path).unwrap();
        assert_eq!(
            dictionary
                .lookup_prefix(&LookupQuery::new("tiế").unwrap(), 10)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            dictionary
                .lookup_full_text(&LookupQuery::new("language").unwrap(), 10)
                .unwrap()
                .len(),
            1
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn list_toggle_and_delete_dictionaries() {
        let directory = temporary_directory("multi_dict_mgmt");
        let database_path = directory.join("dictionary.db");
        let dict1_path = directory.join("english_vi.tsv");
        let dict2_path = directory.join("tech_terms.tsv");

        std::fs::write(&dict1_path, "apple\tquả táo\nbanana\tquả chuối\n").unwrap();
        std::fs::write(&dict2_path, "apple\tApple Inc.\nrust\tngôn ngữ Rust\n").unwrap();

        let mut dictionary = SqliteDictionary::open(&database_path).unwrap();
        let rep1 = dictionary
            .import_path_with_name(&dict1_path, Some("English-Vietnamese"))
            .unwrap();
        assert_eq!(rep1.imported, 2);

        let rep2 = dictionary
            .import_path_with_name(&dict2_path, Some("Tech Terms"))
            .unwrap();
        assert_eq!(rep2.imported, 2);

        let list = dictionary.list_dictionaries().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "English-Vietnamese");
        assert_eq!(list[0].entry_count, 2);
        assert!(list[0].enabled);
        assert_eq!(list[1].name, "Tech Terms");
        assert_eq!(list[1].entry_count, 2);
        assert!(list[1].enabled);

        // Lookup "apple" returns 2 entries with attribution
        let apples = dictionary
            .lookup_exact(&LookupQuery::new("apple").unwrap())
            .unwrap();
        assert_eq!(apples.len(), 2);
        assert_eq!(apples[0].dictionary_name(), Some("English-Vietnamese"));
        assert_eq!(apples[0].definition(), "quả táo");
        assert_eq!(apples[1].dictionary_name(), Some("Tech Terms"));
        assert_eq!(apples[1].definition(), "Apple Inc.");

        // Toggle first dictionary to disabled
        dictionary.toggle_dictionary(list[0].id, false).unwrap();
        let disabled_list = dictionary.list_dictionaries().unwrap();
        assert!(!disabled_list[0].enabled);

        let apples_after_disable = dictionary
            .lookup_exact(&LookupQuery::new("apple").unwrap())
            .unwrap();
        assert_eq!(apples_after_disable.len(), 1);
        assert_eq!(
            apples_after_disable[0].dictionary_name(),
            Some("Tech Terms")
        );

        // Re-enable first dictionary
        dictionary.toggle_dictionary(list[0].id, true).unwrap();
        assert_eq!(
            dictionary
                .lookup_exact(&LookupQuery::new("apple").unwrap())
                .unwrap()
                .len(),
            2
        );

        // Delete second dictionary
        dictionary.delete_dictionary(list[1].id).unwrap();
        let remaining_list = dictionary.list_dictionaries().unwrap();
        assert_eq!(remaining_list.len(), 1);
        assert_eq!(remaining_list[0].id, list[0].id);

        let apples_after_delete = dictionary
            .lookup_exact(&LookupQuery::new("apple").unwrap())
            .unwrap();
        assert_eq!(apples_after_delete.len(), 1);
        assert_eq!(
            apples_after_delete[0].dictionary_name(),
            Some("English-Vietnamese")
        );

        // Deleting non-existent returns error
        assert!(dictionary.delete_dictionary(9999).is_err());
        assert!(dictionary.toggle_dictionary(9999, false).is_err());

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn auto_migration_from_legacy_schema() {
        let directory = temporary_directory("legacy_migration");
        let database_path = directory.join("legacy.db");

        // Manually create legacy schema without dictionaries table
        {
            let conn = Connection::open(&database_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE entries (
                    normalized_headword TEXT PRIMARY KEY,
                    headword TEXT NOT NULL,
                    definition TEXT NOT NULL
                );
                INSERT INTO entries (normalized_headword, headword, definition)
                VALUES ('rust', 'Rust', 'A systems programming language');",
            )
            .unwrap();
        }

        // Open with updated SqliteDictionary
        let mut dictionary = SqliteDictionary::open(&database_path).unwrap();
        let list = dictionary.list_dictionaries().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Default Dictionary");
        assert_eq!(list[0].entry_count, 1);
        assert!(list[0].enabled);

        let results = dictionary
            .lookup_exact(&LookupQuery::new("rust").unwrap())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].headword(), "Rust");
        assert_eq!(results[0].definition(), "A systems programming language");
        assert_eq!(results[0].dictionary_id(), Some(1));
        assert_eq!(results[0].dictionary_name(), Some("Default Dictionary"));

        // Full-text search works on migrated entries
        let fts_results = dictionary
            .lookup_full_text(&LookupQuery::new("programming").unwrap(), 10)
            .unwrap();
        assert_eq!(fts_results.len(), 1);
        assert_eq!(fts_results[0].headword(), "Rust");

        // Can import another dictionary into the migrated database
        let second_path = directory.join("extra.tsv");
        std::fs::write(&second_path, "rust\tgỉ sét\n").unwrap();
        dictionary
            .import_path_with_name(&second_path, Some("Extra Dictionary"))
            .unwrap();

        let updated_list = dictionary.list_dictionaries().unwrap();
        assert_eq!(updated_list.len(), 2);

        let multi_rust = dictionary
            .lookup_exact(&LookupQuery::new("rust").unwrap())
            .unwrap();
        assert_eq!(multi_rust.len(), 2);

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn full_text_search_respects_enabled_dictionaries() {
        let directory = temporary_directory("fts_multi_dict");
        let database_path = directory.join("fts.db");
        let dict1_path = directory.join("d1.tsv");
        let dict2_path = directory.join("d2.tsv");

        std::fs::write(&dict1_path, "rust\ta modern systems language\n").unwrap();
        std::fs::write(&dict2_path, "cplusplus\tan old systems language\n").unwrap();

        let mut dictionary = SqliteDictionary::open(&database_path).unwrap();
        dictionary
            .import_path_with_name(&dict1_path, Some("Rust Dict"))
            .unwrap();
        dictionary
            .import_path_with_name(&dict2_path, Some("C++ Dict"))
            .unwrap();

        let list = dictionary.list_dictionaries().unwrap();
        assert_eq!(list.len(), 2);

        // Both enabled: "systems" finds both
        let results = dictionary
            .lookup_full_text(&LookupQuery::new("systems").unwrap(), 10)
            .unwrap();
        assert_eq!(results.len(), 2);

        // Disable C++ dict
        dictionary.toggle_dictionary(list[1].id, false).unwrap();
        let results_filtered = dictionary
            .lookup_full_text(&LookupQuery::new("systems").unwrap(), 10)
            .unwrap();
        assert_eq!(results_filtered.len(), 1);
        assert_eq!(results_filtered[0].headword(), "rust");
        assert_eq!(results_filtered[0].dictionary_name(), Some("Rust Dict"));

        // Delete Rust dict
        dictionary.delete_dictionary(list[0].id).unwrap();
        let results_after_delete = dictionary
            .lookup_full_text(&LookupQuery::new("systems").unwrap(), 10)
            .unwrap();
        assert_eq!(results_after_delete.len(), 0);

        // Re-enable C++ dict
        dictionary.toggle_dictionary(list[1].id, true).unwrap();
        let results_restored = dictionary
            .lookup_full_text(&LookupQuery::new("systems").unwrap(), 10)
            .unwrap();
        assert_eq!(results_restored.len(), 1);
        assert_eq!(results_restored[0].headword(), "cplusplus");
        assert_eq!(results_restored[0].dictionary_name(), Some("C++ Dict"));

        std::fs::remove_dir_all(directory).unwrap();
    }
}
