use lingoes_core::{DictionaryEntry, LookupQuery};
use lingoes_dictionary::{CURRENT_SCHEMA_VERSION, SqliteDictionary};
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn fresh_database_migrates_to_current_schema() {
    let dict = SqliteDictionary::in_memory().expect("in-memory db");
    assert_eq!(
        dict.schema_version().expect("version"),
        CURRENT_SCHEMA_VERSION
    );
    let list = dict.list_dictionaries().expect("list");
    assert_eq!(list.len(), 0);
}

#[test]
fn migration_from_v1_legacy_fixture_is_backup_producing_and_preserves_data() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("v1_legacy.db");

    // Create v1 legacy schema (only entries table without dictionary_id)
    {
        let conn = Connection::open(&db_path).expect("open raw");
        conn.execute_batch(
            "CREATE TABLE entries (
                normalized_headword TEXT PRIMARY KEY,
                headword TEXT NOT NULL,
                definition TEXT NOT NULL
            );
            INSERT INTO entries (normalized_headword, headword, definition)
            VALUES ('rust', 'Rust', 'A systems programming language');",
        )
        .expect("v1 init");
    }

    // Open with SqliteDictionary which runs migrations and produces backup
    let dict = SqliteDictionary::open(&db_path).expect("open and migrate");
    assert_eq!(
        dict.schema_version().expect("version"),
        CURRENT_SCHEMA_VERSION
    );

    // Verify backup file exists
    let backup_files: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".bak"))
        .collect();
    assert!(
        !backup_files.is_empty(),
        "a backup file must be created before migration"
    );

    // Verify data preserved
    let results = dict
        .lookup_exact(&LookupQuery::new("rust").expect("query"))
        .expect("lookup");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].headword(), "Rust");
    assert_eq!(results[0].definition(), "A systems programming language");

    // Verify dictionary metadata has v3 columns
    let dicts = dict.list_dictionaries().expect("dicts");
    assert_eq!(dicts.len(), 1);
    assert_eq!(dicts[0].sort_order, 0);
    assert!(dicts[0].in_query_group);
    assert!(dicts[0].in_index_group);
    assert!(dicts[0].in_capture_group);
}

#[test]
fn migration_from_v2_fixture_is_backup_producing_and_preserves_data() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("v2_fixture.db");

    // Create v2 schema (dictionaries and entries, but no sort_order, groups, history, favorites)
    {
        let conn = Connection::open(&db_path).expect("open raw");
        conn.execute_batch(
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
            VALUES (1, 'V2 Dict', 'tsv', NULL, 1, 1, '2026-01-01');

            CREATE TABLE entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dictionary_id INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
                normalized_headword TEXT NOT NULL,
                headword TEXT NOT NULL,
                definition TEXT NOT NULL,
                UNIQUE(dictionary_id, normalized_headword)
            );
            INSERT INTO entries (id, dictionary_id, normalized_headword, headword, definition)
            VALUES (1, 1, 'hello', 'Hello', 'A greeting');",
        )
        .expect("v2 init");
    }

    // Open with SqliteDictionary
    let dict = SqliteDictionary::open(&db_path).expect("open and migrate");
    assert_eq!(
        dict.schema_version().expect("version"),
        CURRENT_SCHEMA_VERSION
    );

    // Verify backup created
    let backup_files: Vec<_> = std::fs::read_dir(dir.path())
        .expect("read dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains(".bak"))
        .collect();
    assert!(!backup_files.is_empty(), "backup created for v2 migration");

    // Verify entries and dictionary attributes
    let dicts = dict.list_dictionaries().expect("dicts");
    assert_eq!(dicts.len(), 1);
    assert_eq!(dicts[0].name, "V2 Dict");
    assert_eq!(dicts[0].sort_order, 0);
    assert!(dicts[0].in_query_group);
    assert!(dicts[0].in_index_group);
    assert!(dicts[0].in_capture_group);

    let results = dict
        .lookup_exact(&LookupQuery::new("hello").expect("query"))
        .expect("lookup");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].headword(), "Hello");
}

#[test]
fn migration_is_idempotent() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("idempotent.db");

    let dict1 = SqliteDictionary::open(&db_path).expect("first open");
    assert_eq!(
        dict1.schema_version().expect("version"),
        CURRENT_SCHEMA_VERSION
    );
    drop(dict1);

    // Opening second time does not re-apply or fail
    let dict2 = SqliteDictionary::open(&db_path).expect("second open");
    assert_eq!(
        dict2.schema_version().expect("version"),
        CURRENT_SCHEMA_VERSION
    );
}

#[test]
fn backup_and_restore_database() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("source.db");
    let backup_path = dir.path().join("manual.bak");

    let mut dict = SqliteDictionary::open(&db_path).expect("open");
    dict.upsert(&DictionaryEntry::new("apple", "A fruit").unwrap())
        .expect("upsert");

    dict.backup_to(&backup_path).expect("backup");
    assert!(backup_path.is_file());

    // Modify original
    dict.upsert(&DictionaryEntry::new("banana", "Yellow fruit").unwrap())
        .expect("upsert");
    assert_eq!(
        dict.lookup_exact(&LookupQuery::new("banana").unwrap())
            .unwrap()
            .len(),
        1
    );

    // Restore from backup
    dict.restore_from(&backup_path).expect("restore");
    assert_eq!(
        dict.lookup_exact(&LookupQuery::new("banana").unwrap())
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        dict.lookup_exact(&LookupQuery::new("apple").unwrap())
            .unwrap()
            .len(),
        1
    );
}
