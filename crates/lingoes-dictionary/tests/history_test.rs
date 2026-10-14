use lingoes_dictionary::{HistoryKind, SqliteDictionary};
use tempfile::tempdir;

#[test]
fn history_bounded_capacity_and_deduplication() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");

    // Add with capacity 3
    dict.add_history_entry(HistoryKind::Search, "alpha", None, None, 3)
        .unwrap();
    dict.add_history_entry(HistoryKind::Search, "beta", None, None, 3)
        .unwrap();
    dict.add_history_entry(HistoryKind::Search, "gamma", None, None, 3)
        .unwrap();

    let list = dict.list_history(HistoryKind::Search, None).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].query, "gamma");
    assert_eq!(list[1].query, "beta");
    assert_eq!(list[2].query, "alpha");

    // Add fourth entry: "alpha" evicted
    dict.add_history_entry(HistoryKind::Search, "delta", None, None, 3)
        .unwrap();
    let list = dict.list_history(HistoryKind::Search, None).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].query, "delta");
    assert_eq!(list[1].query, "gamma");
    assert_eq!(list[2].query, "beta");

    // Deduplication: re-add "gamma", should move to top without growing beyond 3
    dict.add_history_entry(HistoryKind::Search, "gamma", None, None, 3)
        .unwrap();
    let list = dict.list_history(HistoryKind::Search, None).unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].query, "gamma");
    assert_eq!(list[1].query, "delta");
    assert_eq!(list[2].query, "beta");
}

#[test]
fn history_survives_restart() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("history.db");

    {
        let mut dict = SqliteDictionary::open(&db_path).unwrap();
        dict.add_history_entry(HistoryKind::Search, "persist_me", None, None, 20)
            .unwrap();
        dict.add_history_entry(
            HistoryKind::Translation,
            "hello world",
            Some("en"),
            Some("vi"),
            20,
        )
        .unwrap();
    }

    // Reopen
    let dict = SqliteDictionary::open(&db_path).unwrap();
    let search_history = dict.list_history(HistoryKind::Search, None).unwrap();
    assert_eq!(search_history.len(), 1);
    assert_eq!(search_history[0].query, "persist_me");

    let translation_history = dict.list_history(HistoryKind::Translation, None).unwrap();
    assert_eq!(translation_history.len(), 1);
    assert_eq!(translation_history[0].query, "hello world");
    assert_eq!(translation_history[0].source_lang.as_deref(), Some("en"));
    assert_eq!(translation_history[0].target_lang.as_deref(), Some("vi"));
}

#[test]
fn clear_history_does_not_clear_favorites() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");
    dict.add_history_entry(HistoryKind::Search, "word_to_clear", None, None, 20)
        .unwrap();
    dict.add_favorite(
        "notebook_word",
        None,
        "Snapshot definition",
        Some("Default"),
        &["tag1"],
        Some("My notes"),
    )
    .unwrap();

    assert_eq!(
        dict.list_history(HistoryKind::Search, None).unwrap().len(),
        1
    );
    assert_eq!(dict.list_favorites(None).unwrap().len(), 1);

    // Clear history
    dict.clear_history(None).unwrap();

    assert_eq!(
        dict.list_history(HistoryKind::Search, None).unwrap().len(),
        0
    );
    // Favorites must remain!
    let favs = dict.list_favorites(None).unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].headword, "notebook_word");
    assert_eq!(favs[0].snapshot_definition, "Snapshot definition");
}
