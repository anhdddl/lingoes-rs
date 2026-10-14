use lingoes_core::LookupQuery;
use lingoes_dictionary::{DictionaryGroup, SqliteDictionary};
use tempfile::tempdir;

#[test]
fn dictionary_ordering_controls_result_priority() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");
    let dir = tempdir().expect("tempdir");

    let dict1_path = dir.path().join("dict1.tsv");
    std::fs::write(&dict1_path, "rust\tSecondary definition\n").unwrap();
    dict.import_path_with_name(&dict1_path, Some("Dict One"))
        .unwrap();

    let dict2_path = dir.path().join("dict2.tsv");
    std::fs::write(&dict2_path, "rust\tPrimary definition\n").unwrap();
    dict.import_path_with_name(&dict2_path, Some("Dict Two"))
        .unwrap();

    let list = dict.list_dictionaries().unwrap();
    let id1 = list.iter().find(|d| d.name == "Dict One").unwrap().id;
    let id2 = list.iter().find(|d| d.name == "Dict Two").unwrap().id;

    // Set Dict Two to sort_order 0 and Dict One to sort_order 10
    dict.set_dictionary_order(id1, 10).unwrap();
    dict.set_dictionary_order(id2, 0).unwrap();

    let results = dict
        .lookup_exact(&LookupQuery::new("rust").unwrap())
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].definition(), "Primary definition");
    assert_eq!(results[1].definition(), "Secondary definition");

    // Reorder: move Dict One to top
    dict.move_dictionary_to_top(id1).unwrap();
    let reordered_results = dict
        .lookup_exact(&LookupQuery::new("rust").unwrap())
        .unwrap();
    assert_eq!(reordered_results[0].definition(), "Secondary definition");
}

#[test]
fn group_filtered_queries_isolate_groups() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");
    let dir = tempdir().expect("tempdir");

    let query_dict_path = dir.path().join("query_dict.tsv");
    std::fs::write(&query_dict_path, "test\tQuery group entry\n").unwrap();
    dict.import_path_with_name(&query_dict_path, Some("Query Only"))
        .unwrap();

    let index_dict_path = dir.path().join("index_dict.tsv");
    std::fs::write(&index_dict_path, "test\tIndex group entry\n").unwrap();
    dict.import_path_with_name(&index_dict_path, Some("Index Only"))
        .unwrap();

    let capture_dict_path = dir.path().join("capture_dict.tsv");
    std::fs::write(&capture_dict_path, "test\tCapture group entry\n").unwrap();
    dict.import_path_with_name(&capture_dict_path, Some("Capture Only"))
        .unwrap();

    let list = dict.list_dictionaries().unwrap();
    let query_id = list.iter().find(|d| d.name == "Query Only").unwrap().id;
    let index_id = list.iter().find(|d| d.name == "Index Only").unwrap().id;
    let capture_id = list.iter().find(|d| d.name == "Capture Only").unwrap().id;

    // Configure distinct memberships
    dict.set_dictionary_groups(query_id, true, false, false)
        .unwrap();
    dict.set_dictionary_groups(index_id, false, true, false)
        .unwrap();
    dict.set_dictionary_groups(capture_id, false, false, true)
        .unwrap();

    let q = LookupQuery::new("test").unwrap();

    // Exact lookup in Query Group
    let query_res = dict
        .lookup_exact_in_group(&q, DictionaryGroup::Query)
        .unwrap();
    assert_eq!(query_res.len(), 1);
    assert_eq!(query_res[0].dictionary_name(), Some("Query Only"));

    // Prefix lookup in Index Group
    let index_res = dict
        .lookup_prefix_in_group(&q, 10, DictionaryGroup::Index)
        .unwrap();
    assert_eq!(index_res.len(), 1);
    assert_eq!(index_res[0].dictionary_name(), Some("Index Only"));

    // Exact lookup in Capture Group
    let capture_res = dict
        .lookup_exact_in_group(&q, DictionaryGroup::Capture)
        .unwrap();
    assert_eq!(capture_res.len(), 1);
    assert_eq!(capture_res[0].dictionary_name(), Some("Capture Only"));

    // Full text lookup in Query Group
    let q_fts = LookupQuery::new("entry").unwrap();
    let fts_query_res = dict
        .lookup_full_text_in_group(&q_fts, 10, DictionaryGroup::Query)
        .unwrap();
    assert_eq!(fts_query_res.len(), 1);
    assert_eq!(fts_query_res[0].dictionary_name(), Some("Query Only"));
}

#[test]
fn disabled_dictionaries_never_leak_through_group_or_fts_lookups() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");
    let dir = tempdir().expect("tempdir");

    let dict_path = dir.path().join("leaker.tsv");
    std::fs::write(&dict_path, "secret\tClassified secret definition\n").unwrap();
    dict.import_path_with_name(&dict_path, Some("Leaker Dict"))
        .unwrap();

    let list = dict.list_dictionaries().unwrap();
    let id = list[0].id;

    // Disable dictionary
    dict.toggle_dictionary(id, false).unwrap();

    let q = LookupQuery::new("secret").unwrap();
    assert!(dict.lookup_exact(&q).unwrap().is_empty());
    assert!(dict.lookup_prefix(&q, 10).unwrap().is_empty());
    assert!(
        dict.lookup_exact_in_group(&q, DictionaryGroup::Query)
            .unwrap()
            .is_empty()
    );
    assert!(
        dict.lookup_prefix_in_group(&q, 10, DictionaryGroup::Index)
            .unwrap()
            .is_empty()
    );
    assert!(
        dict.lookup_full_text(&LookupQuery::new("classified").unwrap(), 10)
            .unwrap()
            .is_empty()
    );
    assert!(
        dict.lookup_full_text_in_group(
            &LookupQuery::new("classified").unwrap(),
            10,
            DictionaryGroup::Query
        )
        .unwrap()
        .is_empty()
    );
}
