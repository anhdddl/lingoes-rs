use lingoes_core::{DictionaryEntry, LookupQuery, normalize_search_text};
use lingoes_dictionary::SqliteDictionary;
use proptest::prelude::*;
use unicode_normalization::UnicodeNormalization;

proptest! {
    #[test]
    fn unicode_nfc_nfd_equivalence(ref s in "\\PC*") {
        let nfc: String = s.nfc().collect();
        let nfd: String = s.nfd().collect();
        prop_assert_eq!(normalize_search_text(&nfc), normalize_search_text(&nfd));
    }

    #[test]
    fn arbitrary_queries_never_panic_in_exact_or_prefix(ref query_str in "\\PC*") {
        let mut dict = SqliteDictionary::in_memory().unwrap();
        let _ = dict.upsert(&DictionaryEntry::new("test", "definition").unwrap());

        if let Ok(query) = LookupQuery::new(query_str) {
            let exact_res = dict.lookup_exact(&query);
            prop_assert!(exact_res.is_ok());
            let prefix_res = dict.lookup_prefix(&query, 10);
            prop_assert!(prefix_res.is_ok());
        }
    }

    #[test]
    fn fts_special_characters_never_syntax_error(ref query_str in ".*") {
        let mut dict = SqliteDictionary::in_memory().unwrap();
        let _ = dict.upsert(&DictionaryEntry::new("test", "A definition with various words").unwrap());

        if let Ok(query) = LookupQuery::new(query_str) {
            let fts_res = dict.lookup_full_text(&query, 10);
            prop_assert!(fts_res.is_ok());
        }
    }
}

#[test]
fn specific_adversarial_sql_and_fts_inputs() {
    let mut dict = SqliteDictionary::in_memory().unwrap();
    dict.upsert(&DictionaryEntry::new("attack", "A cyber attack or physical attack").unwrap())
        .unwrap();

    let adversarial = [
        "\"",
        "\"\"",
        "\"\"\"",
        "' OR '1'='1",
        "'; DROP TABLE entries; --",
        "AND",
        "OR",
        "NOT",
        "NEAR",
        "***",
        "(((((",
        ")))))",
        ":::",
        "^ * : - ( ) \"",
        "test\0hidden",
        "Tiếng Việt có dấu ế, ồ, ẵ, ặ, ỳ",
        "日本語の検索テスト",
        "العربية",
        "русский язык",
    ];

    for term in adversarial {
        if let Ok(query) = LookupQuery::new(term) {
            assert!(dict.lookup_exact(&query).is_ok());
            assert!(dict.lookup_prefix(&query, 10).is_ok());
            assert!(dict.lookup_full_text(&query, 10).is_ok());
        }
    }
}
