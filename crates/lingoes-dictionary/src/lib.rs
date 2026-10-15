use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use lingoes_core::{DictionaryEntry, DictionaryMetadata, LookupQuery};
use opendict::{Dictionary, mdict::MdictDictionary, stardict::StarDictDictionary};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use thiserror::Error;

pub mod ld2;
pub mod ldb;
pub mod ldp;
pub mod migrations;
pub use ld2::{Ld2Dictionary, Ld2Error, clean_lingoes_xml, decode_text};
pub use ldb::LdbArchive;
pub use ldp::LdpPackage;
pub use migrations::CURRENT_SCHEMA_VERSION;

pub struct SqliteDictionary {
    connection: Connection,
    path: Option<PathBuf>,
}

/// Public database API name used by metadata/details consumers.
pub type DictionaryDatabase = SqliteDictionary;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictionaryGroup {
    Query,
    Index,
    Capture,
}

impl DictionaryGroup {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Query => "query",
            Self::Index => "index",
            Self::Capture => "capture",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    Search,
    Translation,
}

impl HistoryKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Translation => "translation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: i64,
    pub kind: HistoryKind,
    pub query: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FavoriteEntry {
    pub id: i64,
    pub headword: String,
    pub dictionary_id: Option<i64>,
    pub dictionary_name: Option<String>,
    pub snapshot_definition: String,
    pub folder: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub created_at: String,
    pub updated_at: String,
    pub source_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportJobMeta {
    pub id: i64,
    pub dictionary_id: Option<i64>,
    pub format: String,
    pub source_path: Option<String>,
    pub imported_entries: usize,
    pub inserted_entries: usize,
    pub updated_entries: usize,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: String,
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
    pub sort_order: i32,
    pub in_query_group: bool,
    pub in_index_group: bool,
    pub in_capture_group: bool,
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
    Ld2,
    Ldp,
    Ldb,
}

impl DictionaryFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tsv => "tsv",
            Self::StarDict => "stardict",
            Self::Mdict => "mdict",
            Self::Ld2 => "ld2",
            Self::Ldp => "ldp",
            Self::Ldb => "ldb",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportProgress {
    pub processed: usize,
    pub total: Option<usize>,
}

fn parse_tsv_metadata(content: &str) -> DictionaryMetadata {
    let mut metadata = DictionaryMetadata::default();
    for raw_line in content.lines() {
        let trimmed = raw_line.trim();
        if !trimmed.starts_with('#') {
            if !trimmed.is_empty() {
                break;
            }
            continue;
        }
        let comment = trimmed.trim_start_matches('#').trim();
        let Some((key, value)) = comment.split_once(':') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase().replace(['_', '-', ' '], "");
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.as_str() {
            "title" | "bookname" | "name" => metadata.title = value.to_string(),
            "author" | "creator" => metadata.author = Some(value.to_string()),
            "publisher" => metadata.publisher = Some(value.to_string()),
            "website" | "url" => metadata.website = Some(value.to_string()),
            "copyright" => metadata.copyright = Some(value.to_string()),
            "description" | "summary" => metadata.description = Some(value.to_string()),
            "sourcelang" | "langfrom" | "from" => metadata.source_lang = Some(value.to_string()),
            "targetlang" | "langto" | "to" => metadata.target_lang = Some(value.to_string()),
            _ => {}
        }
    }
    metadata
}

fn parse_ifo_metadata(path: &Path, fallback: &str) -> Result<DictionaryMetadata, ImportError> {
    let text = std::fs::read_to_string(path)?;
    let mut metadata = DictionaryMetadata::new(fallback);
    metadata.format = Some("stardict".to_string());
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "bookname" => metadata.title = value.to_string(),
            "author" => metadata.author = Some(value.to_string()),
            "publisher" => metadata.publisher = Some(value.to_string()),
            "website" => metadata.website = Some(value.to_string()),
            "copyright" => metadata.copyright = Some(value.to_string()),
            "description" => metadata.description = Some(value.to_string()),
            "sourcelang" | "langfrom" => metadata.source_lang = Some(value.to_string()),
            "targetlang" | "langto" => metadata.target_lang = Some(value.to_string()),
            "wordcount" => metadata.entry_count = value.parse().ok(),
            _ => {}
        }
    }
    Ok(metadata)
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
    } else if file_name.ends_with(".ld2") || file_name.ends_with(".ldx") {
        Ok(DictionaryFormat::Ld2)
    } else if file_name.ends_with(".ldp") {
        Ok(DictionaryFormat::Ldp)
    } else if file_name.ends_with(".ldb") {
        Ok(DictionaryFormat::Ldb)
    } else {
        if let Ok(mut file) = std::fs::File::open(path) {
            let mut magic = [0u8; 4];
            if file.read_exact(&mut magic).is_ok() && (&magic == b"?LD2" || &magic == b"?LDX") {
                return Ok(DictionaryFormat::Ld2);
            }
        }
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
        let mut connection = Connection::open_in_memory()?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::migrate_or_init(&mut connection, None)?;
        Ok(Self {
            connection,
            path: None,
        })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        let path = path.as_ref();
        let mut connection = Connection::open(path)?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrations::migrate_or_init(&mut connection, Some(path))?;
        Ok(Self {
            connection,
            path: Some(path.to_path_buf()),
        })
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn schema_version(&self) -> Result<u32, DictionaryError> {
        migrations::schema_version(&self.connection)
    }

    pub fn backup_to(&self, destination: impl AsRef<Path>) -> Result<(), DictionaryError> {
        migrations::backup_database(&self.connection, destination.as_ref())
    }

    pub fn restore_from(&mut self, source: impl AsRef<Path>) -> Result<(), DictionaryError> {
        migrations::restore_database(source.as_ref(), &mut self.connection)
    }

    pub fn list_dictionaries(&self) -> Result<Vec<DictionaryMeta>, DictionaryError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, format, source_path, enabled, entry_count, created_at, sort_order, in_query_group, in_index_group, in_capture_group
             FROM dictionaries
             ORDER BY sort_order ASC, id ASC",
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
                sort_order: row.get(7)?,
                in_query_group: row.get(8)?,
                in_index_group: row.get(9)?,
                in_capture_group: row.get(10)?,
            })
        })?;
        let mut dictionaries = Vec::new();
        for row in rows {
            dictionaries.push(row?);
        }
        Ok(dictionaries)
    }

    pub fn list_dictionaries_in_group(
        &self,
        group: DictionaryGroup,
    ) -> Result<Vec<DictionaryMeta>, DictionaryError> {
        let group_col = match group {
            DictionaryGroup::Query => "in_query_group",
            DictionaryGroup::Index => "in_index_group",
            DictionaryGroup::Capture => "in_capture_group",
        };
        let sql = format!(
            "SELECT id, name, format, source_path, enabled, entry_count, created_at, sort_order, in_query_group, in_index_group, in_capture_group
             FROM dictionaries
             WHERE enabled = 1 AND {group_col} = 1
             ORDER BY sort_order ASC, id ASC"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map([], |row| {
            Ok(DictionaryMeta {
                id: row.get(0)?,
                name: row.get(1)?,
                format: row.get(2)?,
                source_path: row.get(3)?,
                enabled: row.get(4)?,
                entry_count: row.get::<_, i64>(5)? as usize,
                created_at: row.get(6)?,
                sort_order: row.get(7)?,
                in_query_group: row.get(8)?,
                in_index_group: row.get(9)?,
                in_capture_group: row.get(10)?,
            })
        })?;
        let mut dictionaries = Vec::new();
        for row in rows {
            dictionaries.push(row?);
        }
        Ok(dictionaries)
    }

    pub fn get_dictionary_metadata(
        &self,
        id: i64,
    ) -> Result<Option<DictionaryMetadata>, DictionaryError> {
        self.connection
            .query_row(
                "SELECT id, COALESCE(NULLIF(title, ''), name), author, publisher, website, copyright,
                        description, source_lang, target_lang, format, entry_count
                 FROM dictionaries WHERE id = ?1",
                params![id],
                |row| {
                    Ok(DictionaryMetadata {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        author: row.get(2)?,
                        publisher: row.get(3)?,
                        website: row.get(4)?,
                        copyright: row.get(5)?,
                        description: row.get(6)?,
                        source_lang: row.get(7)?,
                        target_lang: row.get(8)?,
                        format: row.get(9)?,
                        entry_count: row.get::<_, Option<i64>>(10)?.map(|n| n as usize),
                    })
                },
            )
            .optional()
            .map_err(DictionaryError::from)
    }

    pub fn update_dictionary_metadata(
        &mut self,
        id: i64,
        metadata: &DictionaryMetadata,
    ) -> Result<(), DictionaryError> {
        let rows = self.connection.execute(
            "UPDATE dictionaries
             SET name = ?1, title = ?1, author = ?2, publisher = ?3, website = ?4,
                 copyright = ?5, description = ?6, source_lang = ?7, target_lang = ?8
             WHERE id = ?9",
            params![
                metadata.title,
                metadata.author,
                metadata.publisher,
                metadata.website,
                metadata.copyright,
                metadata.description,
                metadata.source_lang,
                metadata.target_lang,
                id
            ],
        )?;
        if rows == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        Ok(())
    }

    pub fn set_dictionary_order(
        &mut self,
        id: i64,
        sort_order: i32,
    ) -> Result<(), DictionaryError> {
        let rows_affected = self.connection.execute(
            "UPDATE dictionaries SET sort_order = ?1 WHERE id = ?2",
            params![sort_order, id],
        )?;
        if rows_affected == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        Ok(())
    }

    pub fn move_dictionary_to_top(&mut self, id: i64) -> Result<(), DictionaryError> {
        let min_order: i32 = self.connection.query_row(
            "SELECT COALESCE(MIN(sort_order), 0) FROM dictionaries",
            [],
            |r| r.get(0),
        )?;
        let new_order = min_order.saturating_sub(1);
        let rows_affected = self.connection.execute(
            "UPDATE dictionaries SET sort_order = ?1 WHERE id = ?2",
            params![new_order, id],
        )?;
        if rows_affected == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        Ok(())
    }

    pub fn set_dictionary_groups(
        &mut self,
        id: i64,
        in_query: bool,
        in_index: bool,
        in_capture: bool,
    ) -> Result<(), DictionaryError> {
        let rows_affected = self.connection.execute(
            "UPDATE dictionaries SET in_query_group = ?1, in_index_group = ?2, in_capture_group = ?3 WHERE id = ?4",
            params![in_query, in_index, in_capture, id],
        )?;
        if rows_affected == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        Ok(())
    }

    pub fn set_dictionary_group(
        &mut self,
        id: i64,
        group: DictionaryGroup,
        included: bool,
    ) -> Result<(), DictionaryError> {
        let group_col = match group {
            DictionaryGroup::Query => "in_query_group",
            DictionaryGroup::Index => "in_index_group",
            DictionaryGroup::Capture => "in_capture_group",
        };
        let sql = format!("UPDATE dictionaries SET {group_col} = ?1 WHERE id = ?2");
        let rows_affected = self.connection.execute(&sql, params![included, id])?;
        if rows_affected == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        Ok(())
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
        let mut content = String::new();
        let mut reader = reader;
        reader.read_to_string(&mut content)?;
        let mut metadata = parse_tsv_metadata(&content);

        let transaction = self
            .connection
            .transaction()
            .map_err(DictionaryError::from)?;
        let effective_name = name
            .filter(|value| !value.trim().is_empty())
            .map(str::trim)
            .or_else(|| (!metadata.title.is_empty()).then_some(metadata.title.as_str()));
        let dictionary_id = match effective_name {
            Some(dict_name) if !dict_name.trim().is_empty() => {
                transaction.execute(
                    "INSERT INTO dictionaries (name, format, source_path, enabled, entry_count, created_at, sort_order, in_query_group, in_index_group, in_capture_group)
                     VALUES (?1, 'tsv', NULL, 1, 0, datetime('now'), 0, 1, 1, 1)",
                    params![dict_name.trim()],
                )?;
                transaction.last_insert_rowid()
            }
            _ => ensure_default_dictionary(&transaction)?,
        };
        let mut report = ImportReport::default();
        for (index, line) in BufReader::new(content.as_bytes()).lines().enumerate() {
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
        metadata.id = Some(dictionary_id);
        metadata.format = Some("tsv".to_string());
        metadata.entry_count = Some(report.inserted + report.updated);
        if metadata.title.is_empty() {
            metadata.title = effective_name.unwrap_or("Imported TSV").to_string();
        }
        transaction.execute(
            "UPDATE dictionaries
             SET entry_count = (SELECT count(*) FROM entries WHERE dictionary_id = ?1),
                 title = ?2, author = ?3, publisher = ?4, website = ?5, copyright = ?6,
                 description = ?7, source_lang = ?8, target_lang = ?9
             WHERE id = ?1",
            params![
                dictionary_id,
                metadata.title,
                metadata.author,
                metadata.publisher,
                metadata.website,
                metadata.copyright,
                metadata.description,
                metadata.source_lang,
                metadata.target_lang
            ],
        )?;
        transaction.commit().map_err(DictionaryError::from)?;

        let _ = self.connection.execute(
            "INSERT INTO import_jobs (dictionary_id, format, source_path, imported_entries, inserted_entries, updated_entries, status, error_message, created_at)
             VALUES (?1, 'tsv', NULL, ?2, ?3, ?4, 'completed', NULL, datetime('now'))",
            params![dictionary_id, report.imported as i64, report.inserted as i64, report.updated as i64],
        );

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

        if format == DictionaryFormat::Ldb {
            let dicts = ldb::LdbArchive::extract_and_parse(path)?;
            let mut total_report = ImportReport::default();
            for (idx, sub_dict) in dicts.into_iter().enumerate() {
                let sub_name = if idx == 0 {
                    dict_name.clone()
                } else {
                    format!("{dict_name} ({})", sub_dict.info().name)
                };
                let report = self.import_boxed_dictionary(
                    Box::new(sub_dict),
                    &sub_name,
                    DictionaryFormat::Ldb,
                    &source_str,
                    None,
                    &mut on_progress,
                )?;
                total_report.imported += report.imported;
                total_report.inserted += report.inserted;
                total_report.updated += report.updated;
            }
            return Ok(total_report);
        }

        let mut extracted_metadata: Option<DictionaryMetadata> = None;
        let dictionary: Box<dyn Dictionary> = match format {
            DictionaryFormat::StarDict => {
                extracted_metadata = Some(parse_ifo_metadata(path, &dict_name)?);
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
            DictionaryFormat::Ld2 => {
                let ld2_dictionary = ld2::Ld2Dictionary::open(path)
                    .map_err(|error| ImportError::Format(error.to_string()))?;
                extracted_metadata = Some(ld2_dictionary.metadata().clone());
                Box::new(ld2_dictionary)
            }
            DictionaryFormat::Ldp => Box::new(ldp::LdpPackage::extract_and_parse(path)?),
            DictionaryFormat::Ldb | DictionaryFormat::Tsv => unreachable!(),
        };

        self.import_boxed_dictionary(
            dictionary,
            &dict_name,
            format,
            &source_str,
            extracted_metadata,
            &mut on_progress,
        )
    }

    fn import_boxed_dictionary(
        &mut self,
        dictionary: Box<dyn Dictionary>,
        dict_name: &str,
        format: DictionaryFormat,
        source_str: &str,
        metadata: Option<DictionaryMetadata>,
        on_progress: &mut dyn FnMut(ImportProgress),
    ) -> Result<ImportReport, ImportError> {
        const MAX_ENTRIES: usize = 5_000_000;
        const MAX_ENTRY_BYTES: usize = 16 * 1024 * 1024;

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
            "INSERT INTO dictionaries (name, format, source_path, enabled, entry_count, created_at, sort_order, in_query_group, in_index_group, in_capture_group)
             VALUES (?1, ?2, ?3, 1, 0, datetime('now'), 0, 1, 1, 1)",
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
        if let Some(mut meta) = metadata {
            meta.id = Some(dictionary_id);
            meta.format = Some(format.as_str().to_string());
            meta.entry_count = Some(report.inserted + report.updated);
            if meta.title.is_empty() {
                meta.title = dict_name.to_string();
            }
            transaction.execute(
                "UPDATE dictionaries SET name = ?2, title = ?2, author = ?3, publisher = ?4,
                        website = ?5, copyright = ?6, description = ?7, source_lang = ?8,
                        target_lang = ?9,
                        entry_count = (SELECT count(*) FROM entries WHERE dictionary_id = ?1)
                 WHERE id = ?1",
                params![
                    dictionary_id,
                    meta.title,
                    meta.author,
                    meta.publisher,
                    meta.website,
                    meta.copyright,
                    meta.description,
                    meta.source_lang,
                    meta.target_lang
                ],
            )?;
        } else {
            transaction.execute(
                "UPDATE dictionaries SET title = name,
                        entry_count = (SELECT count(*) FROM entries WHERE dictionary_id = ?1)
                 WHERE id = ?1",
                params![dictionary_id],
            )?;
        }
        transaction.commit().map_err(DictionaryError::from)?;

        let _ = self.connection.execute(
            "INSERT INTO import_jobs (dictionary_id, format, source_path, imported_entries, inserted_entries, updated_entries, status, error_message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'completed', NULL, datetime('now'))",
            params![dictionary_id, format.as_str(), source_str, report.imported as i64, report.inserted as i64, report.updated as i64],
        );

        Ok(report)
    }

    pub fn lookup_exact(
        &self,
        query: &LookupQuery,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        self.lookup_exact_in_group(query, DictionaryGroup::Query)
    }

    pub fn lookup_exact_in_group(
        &self,
        query: &LookupQuery,
        group: DictionaryGroup,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        let group_col = match group {
            DictionaryGroup::Query => "d.in_query_group",
            DictionaryGroup::Index => "d.in_index_group",
            DictionaryGroup::Capture => "d.in_capture_group",
        };
        let sql = format!(
            "SELECT e.headword, e.definition, e.dictionary_id, d.name
             FROM entries e
             JOIN dictionaries d ON d.id = e.dictionary_id
             WHERE (e.normalized_headword = ?1 OR e.unaccented_headword = ?2) AND d.enabled = 1 AND {group_col} = 1
             ORDER BY CASE WHEN e.normalized_headword = ?1 THEN 0 ELSE 1 END,
                      d.sort_order ASC, d.id ASC, e.headword ASC"
        );
        self.query_entries(&sql, params![query.as_str(), query.unaccented()])
    }

    pub fn lookup_prefix(
        &self,
        query: &LookupQuery,
        limit: usize,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        self.lookup_prefix_in_group(query, limit, DictionaryGroup::Query)
    }

    pub fn lookup_prefix_in_group(
        &self,
        query: &LookupQuery,
        limit: usize,
        group: DictionaryGroup,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let group_col = match group {
            DictionaryGroup::Query => "d.in_query_group",
            DictionaryGroup::Index => "d.in_index_group",
            DictionaryGroup::Capture => "d.in_capture_group",
        };
        let norm_upper = format!("{}{}", query.as_str(), char::MAX);
        let unacc_upper = format!("{}{}", query.unaccented(), char::MAX);
        let sql = format!(
            "SELECT e.headword, e.definition, e.dictionary_id, d.name
             FROM entries e
             JOIN dictionaries d ON d.id = e.dictionary_id
             WHERE ((e.normalized_headword >= ?1 AND e.normalized_headword < ?2)
                 OR (e.unaccented_headword >= ?3 AND e.unaccented_headword < ?4))
               AND d.enabled = 1 AND {group_col} = 1
             ORDER BY CASE 
                        WHEN e.normalized_headword = ?1 THEN 0 
                        WHEN e.unaccented_headword = ?3 THEN 1 
                        WHEN e.normalized_headword >= ?1 AND e.normalized_headword < ?2 THEN 2
                        ELSE 3 
                      END,
                      d.sort_order ASC,
                      e.normalized_headword ASC,
                      d.id ASC
             LIMIT ?5"
        );
        self.query_entries(
            &sql,
            params![
                query.as_str(),
                norm_upper,
                query.unaccented(),
                unacc_upper,
                limit
            ],
        )
    }

    pub fn lookup_smart(
        &self,
        query: &LookupQuery,
        limit: usize,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        self.lookup_smart_in_group(query, limit, DictionaryGroup::Query)
    }

    pub fn lookup_smart_in_group(
        &self,
        query: &LookupQuery,
        limit: usize,
        group: DictionaryGroup,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut results = self.lookup_prefix_in_group(query, limit, group)?;

        let raw_query = query.as_str().trim();
        let words: Vec<&str> = raw_query.split_whitespace().collect();

        // If we already have full results and it's a single word, return directly
        if results.len() >= limit && words.len() <= 1 {
            return Ok(results);
        }

        // For multi-word queries or sparse prefix results, supplement with FTS phrase search
        let mut seen: std::collections::HashSet<(Option<i64>, String)> = results
            .iter()
            .map(|e| (e.dictionary_id(), e.headword().to_string()))
            .collect();

        let clean = raw_query.replace(['"', '*'], "").trim().to_string();
        if !clean.is_empty() {
            let fts_query = format!("headword: \"{clean}\"*");
            let remaining = limit.saturating_sub(results.len()).max(10);
            if let Ok(fts_matches) = self.query_fts_headword_in_group(&fts_query, remaining, group)
            {
                for entry in fts_matches {
                    let key = (entry.dictionary_id(), entry.headword().to_string());
                    if !seen.contains(&key) {
                        seen.insert(key);
                        results.push(entry);
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    pub fn lookup_advanced(
        &self,
        query: &LookupQuery,
        limit: usize,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        self.lookup_advanced_in_group(query, limit, DictionaryGroup::Query)
    }

    pub fn lookup_advanced_in_group(
        &self,
        query: &LookupQuery,
        limit: usize,
        group: DictionaryGroup,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let mut results = self.lookup_smart_in_group(query, limit, group)?;
        if results.len() < limit {
            let mut seen: std::collections::HashSet<(Option<i64>, String)> = results
                .iter()
                .map(|e| (e.dictionary_id(), e.headword().to_string()))
                .collect();
            let remaining = limit - results.len();
            if let Ok(full_text_results) = self.lookup_full_text_in_group(query, remaining, group) {
                for entry in full_text_results {
                    let key = (entry.dictionary_id(), entry.headword().to_string());
                    if !seen.contains(&key) {
                        seen.insert(key);
                        results.push(entry);
                    }
                }
            }
        }
        Ok(results)
    }

    fn query_fts_headword_in_group(
        &self,
        fts_query: &str,
        limit: usize,
        group: DictionaryGroup,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        let group_col = match group {
            DictionaryGroup::Query => "d.in_query_group",
            DictionaryGroup::Index => "d.in_index_group",
            DictionaryGroup::Capture => "d.in_capture_group",
        };
        let sql = format!(
            "SELECT e.headword, e.definition, e.dictionary_id, d.name
             FROM entries_fts f
             JOIN entries e ON e.id = f.rowid
             JOIN dictionaries d ON d.id = e.dictionary_id
             WHERE entries_fts MATCH ?1 AND d.enabled = 1 AND {group_col} = 1
             ORDER BY d.sort_order ASC, bm25(entries_fts), e.normalized_headword ASC, d.id ASC
             LIMIT ?2"
        );
        match self.query_entries(&sql, params![fts_query, limit]) {
            Ok(entries) => Ok(entries),
            Err(DictionaryError::Database(err)) => {
                let msg = err.to_string();
                if msg.contains("syntax error") || msg.contains("fts5") {
                    Ok(Vec::new())
                } else {
                    Err(DictionaryError::Database(err))
                }
            }
            Err(other) => Err(other),
        }
    }

    pub fn lookup_full_text(
        &self,
        query: &LookupQuery,
        limit: usize,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        self.lookup_full_text_in_group(query, limit, DictionaryGroup::Query)
    }

    pub fn lookup_full_text_in_group(
        &self,
        query: &LookupQuery,
        limit: usize,
        group: DictionaryGroup,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let group_col = match group {
            DictionaryGroup::Query => "d.in_query_group",
            DictionaryGroup::Index => "d.in_index_group",
            DictionaryGroup::Capture => "d.in_capture_group",
        };
        let sanitized = query.as_str().replace('\0', " ").replace('"', "\"\"");
        let trimmed = sanitized.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        let fts_query = format!("\"{trimmed}\"");
        let sql = format!(
            "SELECT e.headword, e.definition, e.dictionary_id, d.name
             FROM entries_fts f
             JOIN entries e ON e.id = f.rowid
             JOIN dictionaries d ON d.id = e.dictionary_id
             WHERE entries_fts MATCH ?1 AND d.enabled = 1 AND {group_col} = 1
             ORDER BY d.sort_order ASC, bm25(entries_fts), e.normalized_headword ASC, d.id ASC
             LIMIT ?2"
        );
        match self.query_entries(&sql, params![fts_query, limit]) {
            Ok(entries) => Ok(entries),
            Err(DictionaryError::Database(err)) => {
                let msg = err.to_string();
                if msg.contains("syntax error") || msg.contains("fts5") {
                    Ok(Vec::new())
                } else {
                    Err(DictionaryError::Database(err))
                }
            }
            Err(e) => Err(e),
        }
    }

    // --- Search & Translation History ---
    pub fn add_history_entry(
        &mut self,
        kind: HistoryKind,
        query: &str,
        source_lang: Option<&str>,
        target_lang: Option<&str>,
        capacity: usize,
    ) -> Result<(), DictionaryError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(());
        }
        let capacity = capacity.max(1);
        let tx = self.connection.transaction()?;

        tx.execute(
            "DELETE FROM search_history
             WHERE kind = ?1 AND query = ?2
               AND COALESCE(source_lang, '') = COALESCE(?3, '')
               AND COALESCE(target_lang, '') = COALESCE(?4, '')",
            params![kind.as_str(), query, source_lang, target_lang],
        )?;

        tx.execute(
            "INSERT INTO search_history (kind, query, source_lang, target_lang, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![kind.as_str(), query, source_lang, target_lang],
        )?;

        tx.execute(
            "DELETE FROM search_history
             WHERE kind = ?1 AND id NOT IN (
                 SELECT id FROM search_history WHERE kind = ?1 ORDER BY created_at DESC, id DESC LIMIT ?2
             )",
            params![kind.as_str(), capacity],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn list_history(
        &self,
        kind: HistoryKind,
        limit: Option<usize>,
    ) -> Result<Vec<HistoryEntry>, DictionaryError> {
        let limit_val = limit.unwrap_or(100);
        let mut statement = self.connection.prepare(
            "SELECT id, kind, query, source_lang, target_lang, created_at
             FROM search_history
             WHERE kind = ?1
             ORDER BY created_at DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![kind.as_str(), limit_val], |row| {
            let kind_str: String = row.get(1)?;
            let kind = if kind_str == "translation" {
                HistoryKind::Translation
            } else {
                HistoryKind::Search
            };
            Ok(HistoryEntry {
                id: row.get(0)?,
                kind,
                query: row.get(2)?,
                source_lang: row.get(3)?,
                target_lang: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }
        Ok(entries)
    }

    pub fn clear_history(&mut self, kind: Option<HistoryKind>) -> Result<(), DictionaryError> {
        match kind {
            Some(k) => {
                self.connection.execute(
                    "DELETE FROM search_history WHERE kind = ?1",
                    params![k.as_str()],
                )?;
            }
            None => {
                self.connection.execute("DELETE FROM search_history", [])?;
            }
        }
        Ok(())
    }

    // --- Favorites / Notebook ---
    pub fn add_favorite(
        &mut self,
        headword: &str,
        dictionary_id: Option<i64>,
        snapshot_definition: &str,
        folder: Option<&str>,
        tags: &[&str],
        notes: Option<&str>,
    ) -> Result<i64, DictionaryError> {
        let headword = headword.trim();
        let snapshot_definition = snapshot_definition.trim();
        let folder = folder
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or("Default");
        let tags_str = tags
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(",");
        let notes = notes.unwrap_or("").trim();

        let dictionary_name: Option<String> = if let Some(id) = dictionary_id {
            self.connection
                .query_row(
                    "SELECT name FROM dictionaries WHERE id = ?1",
                    params![id],
                    |r| r.get(0),
                )
                .optional()?
        } else {
            None
        };

        self.connection.execute(
            "INSERT INTO favorites (headword, dictionary_id, dictionary_name, snapshot_definition, folder, tags, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), datetime('now'))",
            params![
                headword,
                dictionary_id,
                dictionary_name,
                snapshot_definition,
                folder,
                tags_str,
                notes
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn update_favorite(
        &mut self,
        id: i64,
        folder: Option<&str>,
        tags: Option<&[&str]>,
        notes: Option<&str>,
    ) -> Result<(), DictionaryError> {
        let existing = self.get_favorite(id)?;
        let Some(existing) = existing else {
            return Err(DictionaryError::NotFound(id));
        };

        let new_folder = folder
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .unwrap_or(&existing.folder);
        let new_tags = tags
            .map(|t| {
                t.iter()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| existing.tags.join(","));
        let new_notes = notes.unwrap_or(&existing.notes);

        self.connection.execute(
            "UPDATE favorites
             SET folder = ?1, tags = ?2, notes = ?3, updated_at = datetime('now')
             WHERE id = ?4",
            params![new_folder, new_tags, new_notes, id],
        )?;
        Ok(())
    }

    pub fn remove_favorite(&mut self, id: i64) -> Result<(), DictionaryError> {
        let rows_affected = self
            .connection
            .execute("DELETE FROM favorites WHERE id = ?1", params![id])?;
        if rows_affected == 0 {
            return Err(DictionaryError::NotFound(id));
        }
        Ok(())
    }

    pub fn get_favorite(&self, id: i64) -> Result<Option<FavoriteEntry>, DictionaryError> {
        let mut statement = self.connection.prepare(
            "SELECT f.id, f.headword, f.dictionary_id, f.dictionary_name, f.snapshot_definition, f.folder, f.tags, f.notes, f.created_at, f.updated_at,
                    (d.id IS NOT NULL AND d.enabled = 1) AS source_available
             FROM favorites f
             LEFT JOIN dictionaries d ON d.id = f.dictionary_id
             WHERE f.id = ?1",
        )?;
        statement
            .query_row(params![id], map_favorite_row)
            .optional()
            .map_err(DictionaryError::from)
    }

    pub fn list_favorites(
        &self,
        folder: Option<&str>,
    ) -> Result<Vec<FavoriteEntry>, DictionaryError> {
        match folder {
            Some(f) if !f.trim().is_empty() => {
                let mut statement = self.connection.prepare(
                    "SELECT f.id, f.headword, f.dictionary_id, f.dictionary_name, f.snapshot_definition, f.folder, f.tags, f.notes, f.created_at, f.updated_at,
                            (d.id IS NOT NULL AND d.enabled = 1) AS source_available
                     FROM favorites f
                     LEFT JOIN dictionaries d ON d.id = f.dictionary_id
                     WHERE f.folder = ?1
                     ORDER BY f.updated_at DESC, f.id DESC",
                )?;
                let rows = statement.query_map(params![f.trim()], map_favorite_row)?;
                let mut list = Vec::new();
                for row in rows {
                    list.push(row?);
                }
                Ok(list)
            }
            _ => {
                let mut statement = self.connection.prepare(
                    "SELECT f.id, f.headword, f.dictionary_id, f.dictionary_name, f.snapshot_definition, f.folder, f.tags, f.notes, f.created_at, f.updated_at,
                            (d.id IS NOT NULL AND d.enabled = 1) AS source_available
                     FROM favorites f
                     LEFT JOIN dictionaries d ON d.id = f.dictionary_id
                     ORDER BY f.updated_at DESC, f.id DESC",
                )?;
                let rows = statement.query_map([], map_favorite_row)?;
                let mut list = Vec::new();
                for row in rows {
                    list.push(row?);
                }
                Ok(list)
            }
        }
    }

    pub fn list_favorite_folders(&self) -> Result<Vec<String>, DictionaryError> {
        let mut statement = self
            .connection
            .prepare("SELECT DISTINCT folder FROM favorites ORDER BY folder ASC")?;
        let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
        let mut folders = Vec::new();
        for row in rows {
            folders.push(row?);
        }
        Ok(folders)
    }

    // --- Online Cache ---
    pub fn get_cached_online_result(
        &self,
        query: &str,
        engine: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<Option<String>, DictionaryError> {
        let mut statement = self.connection.prepare(
            "SELECT result FROM online_cache
             WHERE query = ?1 AND engine = ?2 AND source_lang = ?3 AND target_lang = ?4",
        )?;
        statement
            .query_row(
                params![
                    query.trim(),
                    engine.trim(),
                    source_lang.trim(),
                    target_lang.trim()
                ],
                |r| r.get(0),
            )
            .optional()
            .map_err(DictionaryError::from)
    }

    pub fn cache_online_result(
        &mut self,
        query: &str,
        engine: &str,
        source_lang: &str,
        target_lang: &str,
        result: &str,
    ) -> Result<(), DictionaryError> {
        self.connection.execute(
            "INSERT INTO online_cache (query, engine, source_lang, target_lang, result, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             ON CONFLICT(query, engine, source_lang, target_lang) DO UPDATE SET
                result = excluded.result,
                created_at = excluded.created_at",
            params![
                query.trim(),
                engine.trim(),
                source_lang.trim(),
                target_lang.trim(),
                result.trim()
            ],
        )?;
        Ok(())
    }

    pub fn clear_online_cache(&mut self) -> Result<(), DictionaryError> {
        self.connection.execute("DELETE FROM online_cache", [])?;
        Ok(())
    }

    // --- Import Jobs Audit ---
    pub fn list_import_jobs(&self) -> Result<Vec<ImportJobMeta>, DictionaryError> {
        let mut statement = self.connection.prepare(
            "SELECT id, dictionary_id, format, source_path, imported_entries, inserted_entries, updated_entries, status, error_message, created_at
             FROM import_jobs
             ORDER BY id DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(ImportJobMeta {
                id: row.get(0)?,
                dictionary_id: row.get(1)?,
                format: row.get(2)?,
                source_path: row.get(3)?,
                imported_entries: row.get::<_, i64>(4)? as usize,
                inserted_entries: row.get::<_, i64>(5)? as usize,
                updated_entries: row.get::<_, i64>(6)? as usize,
                status: row.get(7)?,
                error_message: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        let mut jobs = Vec::new();
        for row in rows {
            jobs.push(row?);
        }
        Ok(jobs)
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

fn map_favorite_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FavoriteEntry> {
    let tags_raw: String = row.get(6)?;
    let tags = tags_raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let source_avail_num: i64 = row.get(10)?;
    Ok(FavoriteEntry {
        id: row.get(0)?,
        headword: row.get(1)?,
        dictionary_id: row.get(2)?,
        dictionary_name: row.get(3)?,
        snapshot_definition: row.get(4)?,
        folder: row.get(5)?,
        tags,
        notes: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        source_available: source_avail_num > 0,
    })
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
            "INSERT INTO dictionaries (name, format, source_path, enabled, entry_count, created_at, sort_order, in_query_group, in_index_group, in_capture_group)
             VALUES ('Default Dictionary', 'tsv', NULL, 1, 0, datetime('now'), 0, 1, 1, 1)",
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
        "INSERT INTO entries (dictionary_id, normalized_headword, unaccented_headword, headword, definition)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(dictionary_id, normalized_headword) DO UPDATE SET
            unaccented_headword = excluded.unaccented_headword,
            headword = excluded.headword,
            definition = excluded.definition",
        params![
            dictionary_id,
            entry.normalized_headword(),
            entry.unaccented_headword(),
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
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].headword(), "rust");
    }

    #[test]
    fn zero_limit_returns_no_prefix_results() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("rust", "A systems language").unwrap())
            .unwrap();
        let results = dictionary
            .lookup_prefix(&LookupQuery::new("rust").unwrap(), 0)
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn unicode_normalized_lookup_matches_vietnamese_decomposed_query() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let composed = "ti\u{1ebf}ng vi\u{1ec7}t";
        let decomposed = "tie\u{0302}\u{0301}ng vie\u{0323}\u{0302}t";

        dictionary
            .upsert(&DictionaryEntry::new(composed, "Vietnamese language").unwrap())
            .unwrap();

        let results = dictionary
            .lookup_exact(&LookupQuery::new(decomposed).unwrap())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].headword(), composed);
    }

    #[test]
    fn upsert_replaces_definition_for_same_normalized_headword() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("Rust", "First definition").unwrap())
            .unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("rust", "Updated definition").unwrap())
            .unwrap();

        let results = dictionary
            .lookup_exact(&LookupQuery::new("rust").unwrap())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].definition(), "Updated definition");
    }

    #[test]
    fn full_text_lookup_finds_definition_terms() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("Rust", "A systems language").unwrap())
            .unwrap();
        dictionary
            .upsert(&DictionaryEntry::new("Slint", "A GUI toolkit").unwrap())
            .unwrap();

        let results = dictionary
            .lookup_full_text(&LookupQuery::new("toolkit").unwrap(), 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].headword(), "Slint");
    }

    #[test]
    fn failed_tsv_import_is_atomic() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let tsv_content = "word1\tdef1\nword2\tdef2\nbad_line_no_tab\nword4\tdef4\n";

        let error = dictionary.import_tsv(tsv_content.as_bytes()).unwrap_err();
        assert!(matches!(error, ImportError::MalformedLine { line: 3 }));

        assert!(
            dictionary
                .lookup_exact(&LookupQuery::new("word1").unwrap())
                .unwrap()
                .is_empty()
        );
        assert!(
            dictionary
                .lookup_exact(&LookupQuery::new("word2").unwrap())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn tsv_import_skips_comments_and_blank_lines() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let tsv_content = "# header comment\n\nword1\tdef1\n   \n# another comment\nword2\tdef2\n";

        let report = dictionary.import_tsv(tsv_content.as_bytes()).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(report.inserted, 2);
    }

    #[test]
    fn tsv_import_reports_malformed_line_number() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let tsv_content = "# valid comment\nword1\tdef1\nmalformed_line\n";

        let error = dictionary.import_tsv(tsv_content.as_bytes()).unwrap_err();
        assert!(matches!(error, ImportError::MalformedLine { line: 3 }));
    }

    #[test]
    fn import_report_counts_insertions_and_duplicates() {
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let tsv_content = "word1\tdef1\nword2\tdef2\nword1\tdef1_updated\n";

        let report = dictionary.import_tsv(tsv_content.as_bytes()).unwrap();
        assert_eq!(report.imported, 3);
        assert_eq!(report.inserted, 2);
        assert_eq!(report.updated, 1);
    }

    fn temporary_directory(name: &str) -> std::path::PathBuf {
        let unique = format!(
            "lingoes-tests-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn write_stardict_fixture(
        directory: &Path,
        stem: &str,
        entries: &[(&str, &str)],
    ) -> std::path::PathBuf {
        let ifo_path = directory.join(format!("{stem}.ifo"));
        let idx_path = directory.join(format!("{stem}.idx"));
        let dict_path = directory.join(format!("{stem}.dict"));

        let ifo_content = format!(
            "StarDict's dict ifo file\nversion=2.4.2\nbookname={stem}\nwordcount={}\nidxfilesize={}\nsametypesequence=m\n",
            entries.len(),
            entries.iter().map(|(w, _)| w.len() + 1 + 8).sum::<usize>()
        );
        std::fs::write(&ifo_path, ifo_content).unwrap();

        let mut idx_bytes = Vec::new();
        let mut dict_bytes = Vec::new();

        for (word, definition) in entries {
            let offset = dict_bytes.len() as u32;
            let size = definition.len() as u32;
            dict_bytes.extend_from_slice(definition.as_bytes());

            idx_bytes.extend_from_slice(word.as_bytes());
            idx_bytes.push(0);
            idx_bytes.extend_from_slice(&offset.to_be_bytes());
            idx_bytes.extend_from_slice(&size.to_be_bytes());
        }

        std::fs::write(&idx_path, idx_bytes).unwrap();
        std::fs::write(&dict_path, dict_bytes).unwrap();
        ifo_path
    }

    #[test]
    fn auto_detects_and_imports_stardict_files() {
        let directory = temporary_directory("stardict");
        let ifo_path = write_stardict_fixture(
            &directory,
            "stardict-sample",
            &[
                ("computer", "An electronic device for processing data"),
                ("rust", "A systems programming language"),
            ],
        );

        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let report = dictionary.import_path(&ifo_path).unwrap();
        assert_eq!(report.imported, 2);
        assert_eq!(report.inserted, 2);

        let results = dictionary
            .lookup_exact(&LookupQuery::new("computer").unwrap())
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].headword(), "computer");

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn malformed_stardict_is_rejected_without_partial_import() {
        let directory = temporary_directory("bad-stardict");
        let ifo_path = directory.join("corrupt.ifo");
        let idx_path = directory.join("corrupt.idx");
        let dict_path = directory.join("corrupt.dict");

        std::fs::write(
            &ifo_path,
            "StarDict's dict ifo file\nversion=2.4.2\nbookname=bad\nwordcount=1\nidxfilesize=10\nsametypesequence=m\n",
        )
        .unwrap();
        std::fs::write(&idx_path, [1, 2, 3]).unwrap();
        std::fs::write(&dict_path, [4, 5, 6]).unwrap();

        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let error = dictionary.import_path(&ifo_path).unwrap_err();
        assert!(matches!(error, ImportError::Format(_)));

        let results = dictionary
            .lookup_exact(&LookupQuery::new("bad").unwrap())
            .unwrap();
        assert!(results.is_empty());

        std::fs::remove_dir_all(directory).unwrap();
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
        let mdx_path = directory.join("corrupt.mdx");
        std::fs::write(&mdx_path, [0, 0, 0, 5, 1, 2, 3, 4, 5]).unwrap();

        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let error = dictionary.import_path(&mdx_path).unwrap_err();
        assert!(matches!(error, ImportError::Format(_)));

        let results = dictionary
            .lookup_exact(&LookupQuery::new("anything").unwrap())
            .unwrap();
        assert!(results.is_empty());

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unsupported_extension_has_a_clear_error() {
        let path = Path::new("test.xyz");
        let error = detect_format(path).unwrap_err();
        assert!(matches!(error, ImportError::UnsupportedFormat { .. }));
    }

    #[test]
    fn bulk_import_reports_progress() {
        let directory = temporary_directory("progress");
        let tsv_path = directory.join("words.tsv");
        let mut content = String::new();
        for i in 0..50 {
            content.push_str(&format!("word{i}\tdefinition{i}\n"));
        }
        std::fs::write(&tsv_path, content).unwrap();

        let mut progress_count = 0;
        let mut last_processed = 0;
        let mut dictionary = SqliteDictionary::in_memory().unwrap();
        let report = dictionary
            .import_path_with_progress(&tsv_path, |progress| {
                progress_count += 1;
                last_processed = progress.processed;
            })
            .unwrap();

        assert_eq!(report.imported, 50);
        assert_eq!(last_processed, 50);
        assert_eq!(progress_count, 50);

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn imported_entries_and_full_text_index_persist() {
        let directory = temporary_directory("persistence");
        let database_path = directory.join("lingoes.db");
        let tsv_path = directory.join("words.tsv");
        std::fs::write(&tsv_path, "rust\tA systems language\n").unwrap();

        {
            let mut dictionary = SqliteDictionary::open(&database_path).unwrap();
            dictionary.import_path(&tsv_path).unwrap();
        }

        let dictionary = SqliteDictionary::open(&database_path).unwrap();
        let exact = dictionary
            .lookup_exact(&LookupQuery::new("rust").unwrap())
            .unwrap();
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].headword(), "rust");

        let fts = dictionary
            .lookup_full_text(&LookupQuery::new("systems").unwrap(), 10)
            .unwrap();
        assert_eq!(fts.len(), 1);
        assert_eq!(fts[0].headword(), "rust");

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn list_toggle_and_delete_dictionaries() {
        let directory = temporary_directory("multi_dict_mgmt");
        let database_path = directory.join("test.db");
        let dict1_path = directory.join("dict1.tsv");
        let dict2_path = directory.join("dict2.tsv");

        std::fs::write(&dict1_path, "word1\tdef1\nword2\tdef2\n").unwrap();
        std::fs::write(&dict2_path, "term1\tmeaning1\n").unwrap();

        let mut dictionary = SqliteDictionary::open(&database_path).unwrap();
        dictionary
            .import_path_with_name(&dict1_path, Some("Dict One"))
            .unwrap();
        dictionary
            .import_path_with_name(&dict2_path, Some("Dict Two"))
            .unwrap();

        let list = dictionary.list_dictionaries().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Dict One");
        assert_eq!(list[0].entry_count, 2);
        assert!(list[0].enabled);
        assert_eq!(list[1].name, "Dict Two");
        assert_eq!(list[1].entry_count, 1);
        assert!(list[1].enabled);

        let id1 = list[0].id;
        let id2 = list[1].id;

        // Toggle dict1 disabled
        dictionary.toggle_dictionary(id1, false).unwrap();
        let updated = dictionary.list_dictionaries().unwrap();
        assert!(!updated[0].enabled);
        assert!(updated[1].enabled);

        // Lookup exact on word1 should return empty because dict1 is disabled
        let results = dictionary
            .lookup_exact(&LookupQuery::new("word1").unwrap())
            .unwrap();
        assert!(results.is_empty());

        // Lookup on term1 should still work
        let results2 = dictionary
            .lookup_exact(&LookupQuery::new("term1").unwrap())
            .unwrap();
        assert_eq!(results2.len(), 1);

        // Toggle dict1 back enabled
        dictionary.toggle_dictionary(id1, true).unwrap();
        let results3 = dictionary
            .lookup_exact(&LookupQuery::new("word1").unwrap())
            .unwrap();
        assert_eq!(results3.len(), 1);

        // Delete dict2
        dictionary.delete_dictionary(id2).unwrap();
        let remaining = dictionary.list_dictionaries().unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "Dict One");

        // Lookup term1 returns empty
        let results4 = dictionary
            .lookup_exact(&LookupQuery::new("term1").unwrap())
            .unwrap();
        assert!(results4.is_empty());

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
