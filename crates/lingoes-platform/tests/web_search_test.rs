use lingoes_platform::web_search::{
    BrowserLauncher, MockBrowserLauncher, WebEngine, WebSearchCatalog, WebSearchError,
};
use url::Url;

#[test]
fn default_catalog_has_google_and_oxford() {
    let catalog = WebSearchCatalog::default_catalog();
    assert!(catalog.get_engine("google").is_some());
    assert!(catalog.get_engine("wikipedia").is_some());
    assert_eq!(catalog.default_engine_id(), "google");
}

#[test]
fn build_url_interpolates_and_encodes_query() {
    let catalog = WebSearchCatalog::default_catalog();
    let url = catalog
        .build_url(Some("google"), "hello world & rust")
        .unwrap();

    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("www.google.com"));
    assert!(
        url.query().unwrap().contains("q=hello+world+%26+rust")
            || url
                .query()
                .unwrap()
                .contains("q=hello%20world%20%26%20rust")
    );
}

#[test]
fn build_url_rejects_empty_query() {
    let catalog = WebSearchCatalog::default_catalog();
    let err = catalog.build_url(Some("google"), "   ").unwrap_err();
    assert!(matches!(err, WebSearchError::EmptyQuery));
}

#[test]
fn build_url_rejects_insecure_or_malicious_schemes() {
    let mut catalog = WebSearchCatalog::default_catalog();
    catalog.add_engine(WebEngine {
        id: "evil".into(),
        name: "Evil".into(),
        template_url: "javascript:alert([$key])".into(),
        category: "malicious".into(),
    });

    let err = catalog.build_url(Some("evil"), "test").unwrap_err();
    assert!(matches!(err, WebSearchError::InsecureScheme(_)));

    catalog.add_engine(WebEngine {
        id: "file_exec".into(),
        name: "File".into(),
        template_url: "file:///bin/sh?[$key]".into(),
        category: "malicious".into(),
    });
    let err2 = catalog.build_url(Some("file_exec"), "test").unwrap_err();
    assert!(matches!(err2, WebSearchError::InsecureScheme(_)));
}

#[test]
fn parse_webengine_xml_snippet() {
    let xml = r#"
    <web_engine ver="1">
        <lang code="" desc="Default">
            <cata>
                <engine id="bing" name="Bing" url="https://www.bing.com/search?q=[$key]" />
                <engine id="wiki" name="Wikipedia" url="https://en.wikipedia.org/wiki/[$key]" />
            </cata>
        </lang>
    </web_engine>
    "#;

    let catalog = WebSearchCatalog::from_xml(xml).unwrap();
    assert_eq!(catalog.engines().len(), 2);
    assert!(catalog.get_engine("bing").is_some());
    assert!(catalog.get_engine("wiki").is_some());
}

#[test]
fn mock_browser_launcher_records_launches() {
    let launcher = MockBrowserLauncher::new();
    let url = Url::parse("https://www.google.com/search?q=rust").unwrap();

    launcher.open_url(&url).unwrap();

    let recorded = launcher.recorded_urls();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0], "https://www.google.com/search?q=rust");
}
