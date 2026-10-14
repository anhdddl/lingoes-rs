use lingoes_core::LookupQuery;
use lingoes_dictionary::{CURRENT_SCHEMA_VERSION, DictionaryDatabase, SqliteDictionary};
use rusqlite::Connection;
use std::fs;
use tempfile::tempdir;

#[test]
fn tsv_metadata_extraction_and_persistence() {
    let tsv_content = r#"# Title: Comprehensive English-Vietnamese Dictionary
# Author: Prof. Tran Van
# Publisher: World Publishing House
# Website: https://lingoes-rs.org/dict
# Copyright: (c) 2026 Open Language Project
# Description: Over 100,000 common words with IPA phonetics.
# SourceLang: en
# TargetLang: vi
hello	xin chào
world	thế giới
"#;

    let mut dict = SqliteDictionary::in_memory().expect("in-memory db");
    let report = dict.import_tsv(tsv_content.as_bytes()).expect("import");
    assert_eq!(report.inserted, 2);

    let list = dict.list_dictionaries().expect("list");
    assert_eq!(list.len(), 1);
    let dict_id = list[0].id;

    // Check metadata retrieval via SqliteDictionary / DictionaryDatabase
    let db: &DictionaryDatabase = &dict;
    let meta = db
        .get_dictionary_metadata(dict_id)
        .expect("query meta")
        .expect("must exist");

    assert_eq!(meta.id, Some(dict_id));
    assert_eq!(meta.title, "Comprehensive English-Vietnamese Dictionary");
    assert_eq!(meta.author.as_deref(), Some("Prof. Tran Van"));
    assert_eq!(meta.publisher.as_deref(), Some("World Publishing House"));
    assert_eq!(meta.website.as_deref(), Some("https://lingoes-rs.org/dict"));
    assert_eq!(
        meta.copyright.as_deref(),
        Some("(c) 2026 Open Language Project")
    );
    assert_eq!(
        meta.description.as_deref(),
        Some("Over 100,000 common words with IPA phonetics.")
    );
    assert_eq!(meta.source_lang.as_deref(), Some("en"));
    assert_eq!(meta.target_lang.as_deref(), Some("vi"));
    assert_eq!(meta.format.as_deref(), Some("tsv"));
    assert_eq!(meta.entry_count, Some(2));
}

#[test]
fn stardict_metadata_extraction_from_ifo() {
    let dir = tempdir().expect("tempdir");
    let ifo_path = dir.path().join("oxford.ifo");
    let ifo_content = r#"StarDict's dict ifo file
version=2.4.2
bookname=Oxford Advanced Learner
author=A. S. Hornby
description=Oxford Advanced Learner's Dictionary 8th Edition
website=https://oxfordlearnersdictionaries.com
sametypesequence=m
wordcount=1
idxfilesize=10
"#;
    fs::write(&ifo_path, ifo_content).expect("write ifo");

    // Write a dummy idx and dict.dz or dict file so StarDict stem is detected
    let idx_path = dir.path().join("oxford.idx");
    // Binary idx: "hello\0" (6 bytes) + offset (4 bytes: 0) + size (4 bytes: 5)
    let mut idx_data = Vec::new();
    idx_data.extend_from_slice(b"hello\0");
    idx_data.extend_from_slice(&0u32.to_be_bytes());
    idx_data.extend_from_slice(&5u32.to_be_bytes());
    fs::write(&idx_path, &idx_data).expect("write idx");

    let dict_path = dir.path().join("oxford.dict");
    fs::write(&dict_path, b"world").expect("write dict");

    let mut dict = SqliteDictionary::in_memory().expect("in-memory db");
    let report = dict.import_path(&ifo_path).expect("import stardict");
    assert_eq!(report.inserted, 1);

    let list = dict.list_dictionaries().expect("list");
    assert_eq!(list.len(), 1);
    let dict_id = list[0].id;

    let meta = dict
        .get_dictionary_metadata(dict_id)
        .expect("get metadata")
        .expect("found");
    assert_eq!(meta.title, "Oxford Advanced Learner");
    assert_eq!(meta.author.as_deref(), Some("A. S. Hornby"));
    assert_eq!(
        meta.description.as_deref(),
        Some("Oxford Advanced Learner's Dictionary 8th Edition")
    );
    assert_eq!(
        meta.website.as_deref(),
        Some("https://oxfordlearnersdictionaries.com")
    );
}

#[test]
fn update_dictionary_metadata_works() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory db");
    let report = dict
        .import_tsv(b"test\tdefinition\n".as_slice())
        .expect("import");
    assert_eq!(report.inserted, 1);

    let list = dict.list_dictionaries().expect("list");
    let dict_id = list[0].id;

    let mut meta = dict
        .get_dictionary_metadata(dict_id)
        .expect("query")
        .expect("meta");
    meta.title = "Custom Title".to_string();
    meta.author = Some("Custom Author".to_string());
    meta.publisher = Some("Custom Publisher".to_string());
    meta.website = Some("https://example.com".to_string());
    meta.copyright = Some("2026".to_string());
    meta.description = Some("Updated description".to_string());
    meta.source_lang = Some("en".to_string());
    meta.target_lang = Some("de".to_string());

    dict.update_dictionary_metadata(dict_id, &meta)
        .expect("update");

    let retrieved = dict
        .get_dictionary_metadata(dict_id)
        .expect("query")
        .expect("meta");
    assert_eq!(retrieved.title, "Custom Title");
    assert_eq!(retrieved.author.as_deref(), Some("Custom Author"));
    assert_eq!(retrieved.publisher.as_deref(), Some("Custom Publisher"));
    assert_eq!(retrieved.website.as_deref(), Some("https://example.com"));
    assert_eq!(retrieved.copyright.as_deref(), Some("2026"));
    assert_eq!(
        retrieved.description.as_deref(),
        Some("Updated description")
    );
    assert_eq!(retrieved.source_lang.as_deref(), Some("en"));
    assert_eq!(retrieved.target_lang.as_deref(), Some("de"));
}

#[test]
fn migration_from_v3_to_v4_adds_metadata_columns() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("v3_database.db");

    // Initialize v3 schema
    {
        let conn = Connection::open(&db_path).expect("open raw");
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL,
                description TEXT NOT NULL
            );
            INSERT INTO schema_migrations VALUES (1, '2026-01-01', 'Initial');
            INSERT INTO schema_migrations VALUES (2, '2026-01-01', 'Multi-dict');
            INSERT INTO schema_migrations VALUES (3, '2026-01-01', 'Ordering and groups');

            CREATE TABLE dictionaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                format TEXT NOT NULL,
                source_path TEXT,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                entry_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                in_query_group BOOLEAN NOT NULL DEFAULT 1,
                in_index_group BOOLEAN NOT NULL DEFAULT 1,
                in_capture_group BOOLEAN NOT NULL DEFAULT 1
            );
            INSERT INTO dictionaries (id, name, format, source_path, enabled, entry_count, created_at)
            VALUES (1, 'V3 Legacy Dict', 'tsv', NULL, 1, 1, '2026-01-01');

            CREATE TABLE entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dictionary_id INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
                normalized_headword TEXT NOT NULL,
                headword TEXT NOT NULL,
                definition TEXT NOT NULL,
                UNIQUE(dictionary_id, normalized_headword)
            );
            INSERT INTO entries (id, dictionary_id, normalized_headword, headword, definition)
            VALUES (1, 1, 'apple', 'apple', 'fruit');",
        )
        .expect("v3 setup");
    }

    // Open and migrate to v4
    let dict = SqliteDictionary::open(&db_path).expect("open and migrate");
    assert_eq!(
        dict.schema_version().expect("version"),
        CURRENT_SCHEMA_VERSION
    );
    assert_eq!(CURRENT_SCHEMA_VERSION, 4);

    let meta = dict
        .get_dictionary_metadata(1)
        .expect("query")
        .expect("exists");
    assert_eq!(meta.title, "V3 Legacy Dict");
    assert_eq!(meta.author, None);

    let entries = dict
        .lookup_exact(&LookupQuery::new("apple").unwrap())
        .expect("lookup");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].headword(), "apple");
}
