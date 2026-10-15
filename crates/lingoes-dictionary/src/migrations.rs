use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OptionalExtension};

use crate::DictionaryError;

pub const CURRENT_SCHEMA_VERSION: u32 = 5;

pub fn schema_version(connection: &Connection) -> Result<u32, DictionaryError> {
    let migrations_table_exists: bool = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|count| count > 0)?;

    if migrations_table_exists {
        let version: Option<u32> = connection
            .query_row("SELECT max(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .optional()?;
        return Ok(version.unwrap_or(0));
    }

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

    if !entries_exists && !dictionaries_exists {
        return Ok(0);
    }

    if entries_exists && !dictionaries_exists {
        return Ok(1);
    }

    // dictionaries exists: check which columns exist
    let mut stmt = connection.prepare("PRAGMA table_info(dictionaries)")?;
    let columns = stmt.query_map([], |r| r.get::<_, String>(1))?;
    let mut has_sort_order = false;
    let mut has_title = false;
    for col in columns {
        let c = col?;
        if c == "sort_order" {
            has_sort_order = true;
        } else if c == "title" {
            has_title = true;
        }
    }

    if has_title {
        Ok(4)
    } else if has_sort_order {
        Ok(3)
    } else {
        Ok(2)
    }
}

pub fn backup_database(
    connection: &Connection,
    destination_path: &Path,
) -> Result<(), DictionaryError> {
    if let Some(parent) = destination_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut destination = Connection::open(destination_path)?;
    let backup = rusqlite::backup::Backup::new(connection, &mut destination)?;
    backup.run_to_completion(100, std::time::Duration::from_millis(1), None)?;
    Ok(())
}

pub fn restore_database(
    source_path: &Path,
    destination: &mut Connection,
) -> Result<(), DictionaryError> {
    let source = Connection::open(source_path)?;
    let backup = rusqlite::backup::Backup::new(&source, destination)?;
    backup.run_to_completion(100, std::time::Duration::from_millis(1), None)?;
    Ok(())
}

pub fn migrate_or_init(
    connection: &mut Connection,
    db_path: Option<&Path>,
) -> Result<usize, DictionaryError> {
    let current = schema_version(connection)?;
    if current >= CURRENT_SCHEMA_VERSION {
        return Ok(0);
    }

    // If an existing on-disk database with legacy schema is being upgraded, produce a backup first
    if let Some(path) = db_path {
        if path.is_file() && current > 0 {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let backup_file_name = format!(
                "{}.bak.v{current}.{timestamp}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("database.db")
            );
            let backup_path = path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(backup_file_name);
            backup_database(connection, &backup_path)?;
        }
    }

    let mut applied = 0;

    // Ensure schema_migrations table exists
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL,
            description TEXT NOT NULL
        );",
    )?;

    if current == 0 {
        // Fresh database: apply all initial tables up to CURRENT_SCHEMA_VERSION
        let tx = connection.transaction()?;
        tx.execute_batch(
            "CREATE TABLE dictionaries (
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
                in_capture_group BOOLEAN NOT NULL DEFAULT 1,
                title TEXT,
                author TEXT,
                publisher TEXT,
                website TEXT,
                copyright TEXT,
                description TEXT,
                source_lang TEXT,
                target_lang TEXT
            );

            CREATE TABLE entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dictionary_id INTEGER NOT NULL REFERENCES dictionaries(id) ON DELETE CASCADE,
                normalized_headword TEXT NOT NULL,
                unaccented_headword TEXT,
                headword TEXT NOT NULL,
                definition TEXT NOT NULL,
                UNIQUE(dictionary_id, normalized_headword)
            );
            CREATE INDEX IF NOT EXISTS entries_search_idx ON entries(normalized_headword, dictionary_id);
            CREATE INDEX IF NOT EXISTS entries_unaccented_idx ON entries(unaccented_headword, dictionary_id);
            CREATE INDEX IF NOT EXISTS entries_dict_idx ON entries(dictionary_id);

            CREATE VIRTUAL TABLE entries_fts USING fts5(
                headword,
                definition,
                tokenize = 'unicode61 remove_diacritics 2'
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

            CREATE TABLE search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                query TEXT NOT NULL,
                source_lang TEXT,
                target_lang TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_kind_created ON search_history(kind, created_at DESC, id DESC);

            CREATE TABLE favorites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                headword TEXT NOT NULL,
                dictionary_id INTEGER REFERENCES dictionaries(id) ON DELETE SET NULL,
                dictionary_name TEXT,
                snapshot_definition TEXT NOT NULL,
                folder TEXT NOT NULL DEFAULT 'Default',
                tags TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_favorites_folder ON favorites(folder);
            CREATE INDEX IF NOT EXISTS idx_favorites_headword ON favorites(headword);

            CREATE TABLE online_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                engine TEXT NOT NULL,
                source_lang TEXT NOT NULL DEFAULT '',
                target_lang TEXT NOT NULL DEFAULT '',
                result TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(query, engine, source_lang, target_lang)
            );
            CREATE INDEX IF NOT EXISTS idx_online_cache_lookup ON online_cache(engine, query);

            CREATE TABLE import_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dictionary_id INTEGER REFERENCES dictionaries(id) ON DELETE CASCADE,
                format TEXT NOT NULL,
                source_path TEXT,
                imported_entries INTEGER NOT NULL,
                inserted_entries INTEGER NOT NULL,
                updated_entries INTEGER NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL
            );

            INSERT INTO schema_migrations (version, applied_at, description)
            VALUES (1, datetime('now'), 'Initial schema'),
                   (2, datetime('now'), 'Dictionaries, multi-dictionary entries, FTS5'),
                   (3, datetime('now'), 'Ordering, groups, history, favorites, online cache, import jobs'),
                   (4, datetime('now'), 'Dictionary metadata columns'),
                   (5, datetime('now'), 'Unaccented headword column and accent-insensitive FTS5');",
        )?;
        tx.commit()?;
        return Ok(5);
    }

    if current < 2 {
        // Upgrade from v1 (legacy entries table without dictionary_id)
        let tx = connection.transaction()?;
        tx.execute_batch(
            "CREATE TABLE dictionaries (
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
            INSERT INTO dictionaries (id, name, format, source_path, enabled, entry_count, created_at, sort_order, in_query_group, in_index_group, in_capture_group)
            VALUES (1, 'Default Dictionary', 'tsv', NULL, 1, (SELECT count(*) FROM entries), datetime('now'), 0, 1, 1, 1);

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

            DROP TABLE IF EXISTS entries_fts;
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
            SELECT id, headword, definition FROM entries;

            INSERT INTO schema_migrations (version, applied_at, description)
            VALUES (2, datetime('now'), 'Dictionaries, multi-dictionary entries, FTS5');",
        )?;
        tx.commit()?;
        applied += 1;
    }

    if current < 3 {
        // Upgrade from v2 to v3: add sort_order and group columns, create search_history, favorites, online_cache, import_jobs
        let tx = connection.transaction()?;

        // Add columns to dictionaries if they do not exist
        let needs_columns = {
            let mut stmt = tx.prepare("PRAGMA table_info(dictionaries)")?;
            let columns = stmt.query_map([], |r| r.get::<_, String>(1))?;
            let existing_cols: Vec<String> = columns.filter_map(|c| c.ok()).collect();
            !existing_cols.iter().any(|c| c == "sort_order")
        };

        if needs_columns {
            tx.execute_batch(
                "ALTER TABLE dictionaries ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE dictionaries ADD COLUMN in_query_group BOOLEAN NOT NULL DEFAULT 1;
                 ALTER TABLE dictionaries ADD COLUMN in_index_group BOOLEAN NOT NULL DEFAULT 1;
                 ALTER TABLE dictionaries ADD COLUMN in_capture_group BOOLEAN NOT NULL DEFAULT 1;",
            )?;
        }

        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS search_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                query TEXT NOT NULL,
                source_lang TEXT,
                target_lang TEXT,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_kind_created ON search_history(kind, created_at DESC, id DESC);

            CREATE TABLE IF NOT EXISTS favorites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                headword TEXT NOT NULL,
                dictionary_id INTEGER REFERENCES dictionaries(id) ON DELETE SET NULL,
                dictionary_name TEXT,
                snapshot_definition TEXT NOT NULL,
                folder TEXT NOT NULL DEFAULT 'Default',
                tags TEXT NOT NULL DEFAULT '',
                notes TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_favorites_folder ON favorites(folder);
            CREATE INDEX IF NOT EXISTS idx_favorites_headword ON favorites(headword);

            CREATE TABLE IF NOT EXISTS online_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query TEXT NOT NULL,
                engine TEXT NOT NULL,
                source_lang TEXT NOT NULL DEFAULT '',
                target_lang TEXT NOT NULL DEFAULT '',
                result TEXT NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(query, engine, source_lang, target_lang)
            );
            CREATE INDEX IF NOT EXISTS idx_online_cache_lookup ON online_cache(engine, query);

            CREATE TABLE IF NOT EXISTS import_jobs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                dictionary_id INTEGER REFERENCES dictionaries(id) ON DELETE CASCADE,
                format TEXT NOT NULL,
                source_path TEXT,
                imported_entries INTEGER NOT NULL,
                inserted_entries INTEGER NOT NULL,
                updated_entries INTEGER NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL
            );

            INSERT INTO schema_migrations (version, applied_at, description)
            VALUES (3, datetime('now'), 'Ordering, groups, history, favorites, online cache, import jobs');",
        )?;
        tx.commit()?;
        applied += 1;
    }

    if current < 4 {
        let tx = connection.transaction()?;
        let needs_meta_columns = {
            let mut stmt = tx.prepare("PRAGMA table_info(dictionaries)")?;
            let columns = stmt.query_map([], |r| r.get::<_, String>(1))?;
            let mut has_title = false;
            for col in columns {
                if col? == "title" {
                    has_title = true;
                    break;
                }
            }
            !has_title
        };

        if needs_meta_columns {
            tx.execute_batch(
                "ALTER TABLE dictionaries ADD COLUMN title TEXT;
                 ALTER TABLE dictionaries ADD COLUMN author TEXT;
                 ALTER TABLE dictionaries ADD COLUMN publisher TEXT;
                 ALTER TABLE dictionaries ADD COLUMN website TEXT;
                 ALTER TABLE dictionaries ADD COLUMN copyright TEXT;
                 ALTER TABLE dictionaries ADD COLUMN description TEXT;
                 ALTER TABLE dictionaries ADD COLUMN source_lang TEXT;
                 ALTER TABLE dictionaries ADD COLUMN target_lang TEXT;",
            )?;
        }

        tx.execute(
            "INSERT OR IGNORE INTO schema_migrations (version, applied_at, description)
             VALUES (4, datetime('now'), 'Dictionary metadata columns');",
            [],
        )?;
        tx.commit()?;
        applied += 1;
    }

    if current < 5 {
        let tx = connection.transaction()?;
        let needs_unaccented = {
            let mut stmt = tx.prepare("PRAGMA table_info(entries)")?;
            let columns = stmt.query_map([], |r| r.get::<_, String>(1))?;
            !columns
                .filter_map(|c| c.ok())
                .any(|c| c == "unaccented_headword")
        };

        if needs_unaccented {
            tx.execute_batch(
                "ALTER TABLE entries ADD COLUMN unaccented_headword TEXT;
                 CREATE INDEX IF NOT EXISTS entries_unaccented_idx ON entries(unaccented_headword, dictionary_id);",
            )?;
        }

        {
            let mut select_stmt =
                tx.prepare("SELECT id, headword FROM entries WHERE unaccented_headword IS NULL")?;
            let rows =
                select_stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
            let mut updates = Vec::new();
            for item in rows {
                let (id, headword) = item?;
                let unacc = lingoes_core::strip_vietnamese_diacritics(&headword);
                updates.push((id, unacc));
            }
            drop(select_stmt);

            let mut update_stmt =
                tx.prepare("UPDATE entries SET unaccented_headword = ?1 WHERE id = ?2")?;
            for (id, unacc) in updates {
                update_stmt.execute(rusqlite::params![unacc, id])?;
            }
        }

        tx.execute_batch(
            "DROP TRIGGER IF EXISTS entries_fts_insert;
             DROP TRIGGER IF EXISTS entries_fts_delete;
             DROP TRIGGER IF EXISTS entries_fts_update;
             DROP TABLE IF EXISTS entries_fts;
             CREATE VIRTUAL TABLE entries_fts USING fts5(
                 headword,
                 definition,
                 tokenize = 'unicode61 remove_diacritics 2'
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
             SELECT id, headword, definition FROM entries;
             INSERT OR IGNORE INTO schema_migrations (version, applied_at, description)
             VALUES (5, datetime('now'), 'Unaccented headword column and accent-insensitive FTS5');",
        )?;
        tx.commit()?;
        applied += 1;
    }

    Ok(applied)
}
