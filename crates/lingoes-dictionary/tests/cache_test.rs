use lingoes_dictionary::SqliteDictionary;

#[test]
fn online_cache_store_retrieve_and_clear() {
    let mut dict = SqliteDictionary::in_memory().expect("in-memory");

    // Initially nothing in cache
    assert!(
        dict.get_cached_online_result("bonjour", "google", "fr", "en")
            .unwrap()
            .is_none()
    );

    // Cache a result
    dict.cache_online_result("bonjour", "google", "fr", "en", "hello")
        .unwrap();

    // Retrieve cached result
    let cached = dict
        .get_cached_online_result("bonjour", "google", "fr", "en")
        .unwrap();
    assert_eq!(cached.as_deref(), Some("hello"));

    // Different language pair or engine is not returned
    assert!(
        dict.get_cached_online_result("bonjour", "openai", "fr", "en")
            .unwrap()
            .is_none()
    );
    assert!(
        dict.get_cached_online_result("bonjour", "google", "fr", "vi")
            .unwrap()
            .is_none()
    );

    // Clear online cache
    dict.clear_online_cache().unwrap();
    assert!(
        dict.get_cached_online_result("bonjour", "google", "fr", "en")
            .unwrap()
            .is_none()
    );
}
