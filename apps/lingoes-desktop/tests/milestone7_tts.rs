use std::sync::Arc;

use lingoes_desktop::DesktopApp;
use lingoes_dictionary::SqliteDictionary;
use lingoes_translator::MockTranslator;
use lingoes_tts::{MockTtsEngine, SpeechEventKind, TtsEngine};
use tempfile::tempdir;

#[test]
fn popup_auto_pronounce_triggers_tts_when_enabled() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let tsv_path = dir.path().join("dict.tsv");
    std::fs::write(&tsv_path, "hello\tXin chào\nworld\tThế giới\n").unwrap();

    let dict = SqliteDictionary::open(&db_path).unwrap();
    let translator = Arc::new(MockTranslator::new());
    let tts = Arc::new(MockTtsEngine::new());

    let mut app = DesktopApp::with_services(dict, translator, tts.clone()).unwrap();
    app.import_path(&tsv_path).unwrap();

    // With auto_pronounce = false, capture should not trigger speech
    app.settings_mut().popup.auto_pronounce = false;
    app.handle_captured_text("hello");
    assert!(app.show_quick_popup());
    assert_eq!(app.quick_popup_word(), "hello");
    assert_eq!(tts.last_spoken(), None);

    // With auto_pronounce = true, capture should trigger speech for headword
    app.settings_mut().popup.auto_pronounce = true;
    app.handle_captured_text("world");
    assert!(app.show_quick_popup());
    assert_eq!(app.quick_popup_word(), "world");
    assert_eq!(tts.last_spoken(), Some(("world".into(), "en".into())));
}

#[test]
fn desktop_app_stop_tts_cancels_speech() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("test.db");

    let dict = SqliteDictionary::open(&db_path).unwrap();
    let translator = Arc::new(MockTranslator::new());
    let tts = Arc::new(MockTtsEngine::new());

    let mut app = DesktopApp::with_services(dict, translator, tts.clone()).unwrap();
    app.speak("Testing speech", "en").unwrap();

    let events = tts.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, SpeechEventKind::Started);

    app.stop_tts().unwrap();
    assert_eq!(app.status(), "Đã dừng phát âm");

    let events = tts.drain_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, SpeechEventKind::Stopped);
}
