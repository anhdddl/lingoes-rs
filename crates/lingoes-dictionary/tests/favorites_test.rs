use lingoes_core::DictionaryEntry;
use lingoes_dictionary::SqliteDictionary;

#[test]
fn favorites_crud_and_folder_filtering() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");
    let fav_id = dict
        .add_favorite(
            "serendipity",
            None,
            "Finding good things without looking for them",
            Some("Vocabulary"),
            &["english", "positive"],
            Some("Heard in a podcast"),
        )
        .unwrap();

    let fav = dict.get_favorite(fav_id).unwrap().expect("found");
    assert_eq!(fav.headword, "serendipity");
    assert_eq!(fav.folder, "Vocabulary");
    assert_eq!(fav.tags, vec!["english", "positive"]);
    assert_eq!(fav.notes, "Heard in a podcast");

    // List by folder
    let vocab_favs = dict.list_favorites(Some("Vocabulary")).unwrap();
    assert_eq!(vocab_favs.len(), 1);
    let other_favs = dict.list_favorites(Some("Other")).unwrap();
    assert_eq!(other_favs.len(), 0);

    // List folders
    let folders = dict.list_favorite_folders().unwrap();
    assert!(folders.contains(&"Vocabulary".to_string()));

    // Update favorite
    dict.update_favorite(
        fav_id,
        Some("Advanced"),
        Some(&["rare", "noun"]),
        Some("Updated note"),
    )
    .unwrap();
    let updated = dict.get_favorite(fav_id).unwrap().unwrap();
    assert_eq!(updated.folder, "Advanced");
    assert_eq!(updated.tags, vec!["rare", "noun"]);
    assert_eq!(updated.notes, "Updated note");

    // Remove favorite
    dict.remove_favorite(fav_id).unwrap();
    assert!(dict.get_favorite(fav_id).unwrap().is_none());
}

#[test]
fn favorites_survive_dictionary_disable_and_removal() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");
    let entry = DictionaryEntry::new("resilience", "The capacity to recover quickly").unwrap();
    dict.upsert(&entry).unwrap();

    let list = dict.list_dictionaries().unwrap();
    let dict_id = list[0].id;
    let dict_name = &list[0].name;

    // Add favorite referencing this dictionary
    let fav_id = dict
        .add_favorite(
            "resilience",
            Some(dict_id),
            "The capacity to recover quickly",
            Some("Default"),
            &["psychology"],
            None,
        )
        .unwrap();

    let fav = dict.get_favorite(fav_id).unwrap().unwrap();
    assert!(fav.source_available, "source dictionary is enabled");
    assert_eq!(fav.dictionary_name.as_deref(), Some(dict_name.as_str()));

    // Disable dictionary
    dict.toggle_dictionary(dict_id, false).unwrap();
    let fav_disabled = dict.get_favorite(fav_id).unwrap().unwrap();
    assert!(
        !fav_disabled.source_available,
        "source dictionary is disabled -> stale source"
    );
    assert_eq!(
        fav_disabled.snapshot_definition, "The capacity to recover quickly",
        "snapshot must be preserved"
    );

    // Delete dictionary entirely
    dict.delete_dictionary(dict_id).unwrap();
    let fav_after_delete = dict.get_favorite(fav_id).unwrap().unwrap();
    assert!(
        !fav_after_delete.source_available,
        "source dictionary deleted -> source unavailable"
    );
    assert_eq!(
        fav_after_delete.snapshot_definition, "The capacity to recover quickly",
        "snapshot preserved after dictionary deletion"
    );
}
