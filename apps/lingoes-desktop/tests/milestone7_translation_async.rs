use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tempfile::tempdir;

use lingoes_desktop::{DesktopApp, TranslationOptions};
use lingoes_dictionary::SqliteDictionary;
use lingoes_translator::{MockTranslator, TranslationError, TranslationRequest, Translator};
use lingoes_tts::MockTtsEngine;

#[test]
fn desktop_app_translate_updates_status_with_provider_attribution() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let dict = SqliteDictionary::open(&db_path).unwrap();
    let mut mock_trans = MockTranslator::new();
    mock_trans.add_translation("Hello", "en", "vi", "Xin chào");
    let translator = Arc::new(mock_trans);
    let tts = Arc::new(MockTtsEngine::new());

    let mut app = DesktopApp::with_services(dict, translator, tts).unwrap();
    let result = app.translate("Hello", "en", "vi").unwrap();

    assert_eq!(result, "Xin chào");
    assert!(
        app.status().contains("Mock"),
        "Status should cite provider: {}",
        app.status()
    );
    assert!(!app.has_error());
}

#[test]
fn desktop_app_translate_empty_text_sets_error_status() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let dict = SqliteDictionary::open(&db_path).unwrap();
    let translator = Arc::new(MockTranslator::new());
    let tts = Arc::new(MockTtsEngine::new());

    let mut app = DesktopApp::with_services(dict, translator, tts).unwrap();
    let err = app.translate("   ", "en", "vi").unwrap_err();

    assert!(app.has_error());
    assert!(err.to_string().contains("empty"));
}

#[test]
fn desktop_app_translate_with_engine_attributions() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let dict = SqliteDictionary::open(&db_path).unwrap();
    let mut mock_trans = MockTranslator::new();
    mock_trans.add_translation("Computer", "en", "vi", "Máy tính");
    let translator = Arc::new(mock_trans);
    let tts = Arc::new(MockTtsEngine::new());

    let mut app = DesktopApp::with_services(dict, translator, tts).unwrap();
    let opts = TranslationOptions {
        engine: "mock",
        base_url: None,
        model: None,
        api_key: None,
    };
    let res = app
        .translate_with_engine("Computer", "en", "vi", opts)
        .unwrap();

    assert_eq!(res.text, "Máy tính");
    assert_eq!(res.provider, "Mock");
    assert_eq!(res.source_lang, "en");
    assert_eq!(res.target_lang, "vi");
}

#[tokio::test]
async fn desktop_async_translation_cancellation_aborts_promptly() {
    let mut mock_trans = MockTranslator::new();
    mock_trans.set_delay(Some(Duration::from_millis(500)));
    let translator = Arc::new(mock_trans);

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_flag_clone = Arc::clone(&cancel_flag);

    let trans_clone = Arc::clone(&translator);
    let handle = tokio::spawn(async move {
        let mut req = TranslationRequest::new("Long text to translate", "en", "vi");
        req.cancellation_token = Some(cancel_flag_clone);
        trans_clone.translate(req).await
    });

    // Trigger cancel after 50ms
    tokio::time::sleep(Duration::from_millis(50)).await;
    cancel_flag.store(true, Ordering::SeqCst);

    let res = handle.await.unwrap();
    assert!(
        matches!(res, Err(TranslationError::Cancelled)),
        "Expected Cancelled error, got {res:?}"
    );
}
