use std::io::{BufRead, BufReader, Read};
use std::path::Path;

use lingoes_core::{DictionaryEntry, LookupQuery};
use rusqlite::{Connection, params};
use thiserror::Error;

pub struct SqliteDictionary {
    connection: Connection,
}

#[derive(Debug, Error)]
pub enum DictionaryError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("stored dictionary entry is invalid")]
    InvalidStoredEntry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
}

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("unable to read dictionary: {0}")]
    Io(#[from] std::io::Error),
    #[error("line {line} must contain a tab-separated headword and definition")]
    MalformedLine { line: usize },
    #[error("line {line} contains an invalid entry")]
    InvalidEntry { line: usize },
    #[error(transparent)]
    Dictionary(#[from] DictionaryError),
}

impl SqliteDictionary {
    pub fn in_memory() -> Result<Self, DictionaryError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, DictionaryError> {
        Self::from_connection(Connection::open(path)?)
    }

    fn from_connection(connection: Connection) -> Result<Self, DictionaryError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                normalized_headword TEXT PRIMARY KEY,
                headword TEXT NOT NULL,
                definition TEXT NOT NULL
            );",
        )?;
        Ok(Self { connection })
    }

    pub fn upsert(&mut self, entry: &DictionaryEntry) -> Result<(), DictionaryError> {
        self.connection.execute(
            "INSERT INTO entries (normalized_headword, headword, definition)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(normalized_headword) DO UPDATE SET
                headword = excluded.headword,
                definition = excluded.definition",
            params![
                entry.normalized_headword(),
                entry.headword(),
                entry.definition()
            ],
        )?;
        Ok(())
    }

    pub fn import_tsv(&mut self, reader: impl Read) -> Result<ImportReport, ImportError> {
        let mut imported = 0;
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
            self.upsert(&entry)?;
            imported += 1;
        }
        Ok(ImportReport { imported })
    }

    pub fn lookup_exact(
        &self,
        query: &LookupQuery,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        self.query_entries(
            "SELECT headword, definition FROM entries
             WHERE normalized_headword = ?1
             ORDER BY headword",
            query.as_str(),
            1,
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
        let pattern = format!("{}%", escape_like(query.as_str()));
        self.query_entries(
            "SELECT headword, definition FROM entries
             WHERE normalized_headword LIKE ?1 ESCAPE '\\'
             ORDER BY CASE WHEN normalized_headword = ?2 THEN 0 ELSE 1 END,
                      normalized_headword
             LIMIT ?3",
            &pattern,
            limit,
        )
    }

    fn query_entries(
        &self,
        sql: &str,
        value: &str,
        limit: usize,
    ) -> Result<Vec<DictionaryEntry>, DictionaryError> {
        let mut statement = self.connection.prepare(sql)?;
        let rows = if sql.contains("?2") {
            statement.query_map(
                params![value, value.trim_end_matches('%'), limit],
                map_entry,
            )?
        } else {
            statement.query_map(params![value], map_entry)?
        };
        rows.map(|row| {
            let (headword, definition) = row?;
            DictionaryEntry::new(headword, definition)
                .map_err(|_| DictionaryError::InvalidStoredEntry)
        })
        .collect()
    }
}

fn map_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<(String, String)> {
    Ok((row.get(0)?, row.get(1)?))
}

fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
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
}
