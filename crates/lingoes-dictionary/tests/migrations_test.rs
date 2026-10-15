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

#[test]
fn test_unaccented_and_multiword_vietnamese_lookup() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory db");
    dict.upsert(&DictionaryEntry::new("chào", "to greet, salute").unwrap())
        .unwrap();
    dict.upsert(&DictionaryEntry::new("chào cờ", "to salute the flag").unwrap())
        .unwrap();
    dict.upsert(&DictionaryEntry::new("chào buổi sáng", "good morning").unwrap())
        .unwrap();
    dict.upsert(&DictionaryEntry::new("anh chàng keo kiệt", "miserly person").unwrap())
        .unwrap();
    dict.upsert(&DictionaryEntry::new("keo kiệt", "stingy").unwrap())
        .unwrap();
    dict.upsert(
        &DictionaryEntry::new(
            "cộng hòa xã hội chủ nghĩa việt nam",
            "socialist republic of vietnam",
        )
        .unwrap(),
    )
    .unwrap();

    // 1. Single word unaccented prefix
    let q = LookupQuery::new("chao").unwrap();
    let res = dict.lookup_prefix(&q, 10).unwrap();
    let words: Vec<&str> = res.iter().map(|e| e.headword()).collect();
    assert!(words.contains(&"chào"));
    assert!(words.contains(&"chào cờ"));
    assert!(words.contains(&"chào buổi sáng"));

    // 2. Multi-word unaccented prefix (2 words)
    let q = LookupQuery::new("chao co").unwrap();
    let res = dict.lookup_prefix(&q, 10).unwrap();
    let words: Vec<&str> = res.iter().map(|e| e.headword()).collect();
    assert_eq!(words, vec!["chào cờ"]);

    // 3. Multi-word phrase anywhere in headword (e.g. 'keo kiet' inside 'anh chàng keo kiệt')
    let q = LookupQuery::new("keo kiet").unwrap();
    let res = dict.lookup_smart(&q, 10).unwrap();
    let words: Vec<&str> = res.iter().map(|e| e.headword()).collect();
    assert!(words.contains(&"keo kiệt"));
    assert!(words.contains(&"anh chàng keo kiệt"));

    // 4. 4 words unaccented
    let q = LookupQuery::new("cong hoa xa hoi").unwrap();
    let res = dict.lookup_smart(&q, 10).unwrap();
    let words: Vec<&str> = res.iter().map(|e| e.headword()).collect();
    assert!(words.contains(&"cộng hòa xã hội chủ nghĩa việt nam"));

    // 5. Advanced search finding keywords in definition
    let q = LookupQuery::new("salute").unwrap();
    let res = dict.lookup_advanced(&q, 10).unwrap();
    let words: Vec<&str> = res.iter().map(|e| e.headword()).collect();
    assert!(words.contains(&"chào"));
    assert!(words.contains(&"chào cờ"));
}
