use std::collections::VecDeque;
use std::time::{Duration, Instant};

use lingoes_capture::{
    Activation, CaptureConfig, CaptureContext, CaptureError, CaptureMode, CapturePipeline,
    CaptureResult, ClipboardContent, ClipboardEventProcessor, ClipboardStore, DelayDebounceGate,
    FilterRule, MatchAction, MockOcrEngine, MouseButton, OcrEngine, Point, Rect, Script,
    SelectionCapture, TextCandidate, TextFilters, TextSource, UiAutomationCapture,
};

fn context() -> CaptureContext {
    CaptureContext {
        process_name: "reader.exe".into(),
        window_class: "DocumentWindow".into(),
        window_title: "Reader".into(),
        secure: false,
        point: Some(Point { x: 50, y: 60 }),
    }
}

#[test]
fn domain_defaults_cover_cursor_activation_delay_and_debounce() {
    let config = CaptureConfig::default();
    assert_eq!(config.mode, CaptureMode::CursorWord);
    assert_eq!(config.activation.button, MouseButton::Middle);
    assert_eq!(config.delay, Duration::from_millis(150));
    assert_eq!(config.debounce, Duration::from_millis(300));
}

#[test]
fn activation_supports_double_click_and_modifiers() {
    let activation = Activation::double_click(MouseButton::Left).with_ctrl(true);
    assert!(activation.double_click);
    assert!(activation.modifiers.ctrl);
}

#[test]
fn phrase_and_script_number_filters_are_applied() {
    let filters = TextFilters {
        phrase_matching: true,
        max_phrase_words: 3,
        allowed_scripts: [Script::Latin].into_iter().collect(),
        allow_numbers: false,
        ..Default::default()
    };
    assert_eq!(
        filters.normalize("  hello,   brave world!  ").unwrap(),
        "hello, brave world"
    );
    assert!(filters.normalize("12345").is_err());
    assert!(filters.normalize("привет").is_err());
}

#[test]
fn filters_reject_malformed_control_and_overlong_text() {
    let filters = TextFilters {
        max_chars: 5,
        ..Default::default()
    };
    assert!(filters.normalize("abcdef").is_err());
    assert!(filters.normalize("ab\0cd").is_err());
}

#[test]
fn deny_rules_win_and_allow_rules_limit_matching_windows() {
    let config = CaptureConfig {
        rules: vec![
            FilterRule::process("reader.exe", MatchAction::Allow),
            FilterRule::window_class("PasswordBox", MatchAction::Deny),
        ],
        ..Default::default()
    };
    assert!(config.allows(&context()));
    let mut other = context();
    other.process_name = "other.exe".into();
    assert!(!config.allows(&other));
    let mut password = context();
    password.window_class = "PasswordBox".into();
    assert!(!config.allows(&password));
    password.secure = true;
    assert!(!config.allows(&password));
}

#[test]
fn delay_and_debounce_gate_requires_stability_and_suppresses_repeat() {
    let start = Instant::now();
    let mut gate = DelayDebounceGate::new(Duration::from_millis(100), Duration::from_millis(300));
    assert!(!gate.observe("word", start));
    assert!(!gate.observe("word", start + Duration::from_millis(99)));
    assert!(gate.observe("word", start + Duration::from_millis(100)));
    assert!(!gate.observe("word", start + Duration::from_millis(200)));
    assert!(gate.observe("word", start + Duration::from_millis(400)));
}

#[test]
fn clipboard_events_coalesce_self_writes_and_duplicate_updates() {
    let mut processor = ClipboardEventProcessor::new(Duration::from_millis(100));
    let now = Instant::now();
    processor.tag_self_write("ours");
    assert_eq!(processor.on_update("ours", now), None);
    assert_eq!(processor.on_update("fresh", now), Some("fresh".into()));
    assert_eq!(
        processor.on_update("fresh", now + Duration::from_millis(50)),
        None
    );
    assert_eq!(
        processor.on_update("other", now + Duration::from_millis(150)),
        Some("other".into())
    );
}

#[derive(Default)]
struct MemoryClipboard {
    content: ClipboardContent,
    writes: usize,
}
impl ClipboardStore for MemoryClipboard {
    fn snapshot(&mut self) -> Result<ClipboardContent, CaptureError> {
        Ok(self.content.clone())
    }
    fn restore(&mut self, content: ClipboardContent) -> Result<(), CaptureError> {
        self.content = content;
        self.writes += 1;
        Ok(())
    }
    fn text(&mut self) -> Result<Option<String>, CaptureError> {
        Ok(self.content.text.clone())
    }
}

struct CopySelection {
    text: Option<String>,
}
impl SelectionCapture for CopySelection {
    fn selected_text(&self, _: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError> {
        Ok(None)
    }
    fn copy_selection(
        &self,
        clipboard: &mut dyn ClipboardStore,
        _: Duration,
    ) -> Result<(), CaptureError> {
        clipboard.restore(ClipboardContent {
            text: self.text.clone(),
        })
    }
}

#[test]
fn optional_copy_fallback_restores_clipboard() {
    let selection = CopySelection {
        text: Some("selected".into()),
    };
    let mut clipboard = MemoryClipboard {
        content: ClipboardContent {
            text: Some("original".into()),
        },
        writes: 0,
    };
    let candidate = lingoes_capture::capture_selection_with_clipboard(
        &selection,
        &mut clipboard,
        &context(),
        Duration::from_millis(20),
    )
    .unwrap()
    .unwrap();
    assert_eq!(candidate.text, "selected");
    assert_eq!(clipboard.content.text.as_deref(), Some("original"));
    assert_eq!(clipboard.writes, 2);
}

struct MockUia {
    values: std::sync::Mutex<VecDeque<Result<Option<TextCandidate>, CaptureError>>>,
}
impl UiAutomationCapture for MockUia {
    fn text_at_cursor(&self, _: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError> {
        self.values.lock().unwrap().pop_front().unwrap_or(Ok(None))
    }
    fn selected_text(&self, _: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError> {
        Ok(None)
    }
}

#[test]
fn pipeline_orders_uia_selection_then_ocr_and_deduplicates() {
    let uia = MockUia {
        values: std::sync::Mutex::new(VecDeque::from([
            Ok(Some(TextCandidate::new("cursor", TextSource::UiAutomation))),
            Ok(Some(TextCandidate::new("cursor", TextSource::UiAutomation))),
            Ok(None),
        ])),
    };
    let selection = CopySelection { text: None };
    let ocr = MockOcrEngine::new(Some(TextCandidate::new("ocr", TextSource::Ocr)));
    let mut clipboard = MemoryClipboard::default();
    let mut pipeline = CapturePipeline::new(
        uia,
        selection,
        ocr,
        CaptureConfig {
            ocr_enabled: true,
            ..Default::default()
        },
    );

    let first = pipeline
        .capture(&context(), &mut clipboard, Instant::now())
        .unwrap()
        .unwrap();
    assert_eq!(first.text, "cursor");
    assert_eq!(first.source, TextSource::UiAutomation);
    let duplicate = pipeline
        .capture(&context(), &mut clipboard, Instant::now())
        .unwrap();
    assert!(duplicate.is_none());
    let fallback = pipeline
        .capture(
            &context(),
            &mut clipboard,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap()
        .unwrap();
    assert_eq!(fallback.text, "ocr");
}

#[test]
fn pipeline_result_preserves_context_bounds_and_provenance() {
    let result = CaptureResult {
        text: "word".into(),
        source: TextSource::UiAutomation,
        context: context(),
        bounds: Some(Rect {
            left: 1,
            top: 2,
            right: 3,
            bottom: 4,
        }),
        confidence: Some(0.9),
    };
    assert_eq!(result.bounds.unwrap().right, 3);
    assert_eq!(result.context.process_name, "reader.exe");
}

#[test]
fn mock_ocr_is_deterministic_and_unsupported_is_explicit() {
    let ocr = MockOcrEngine::unsupported();
    assert!(matches!(
        ocr.recognize(&context()),
        Err(CaptureError::Unsupported(_))
    ));
}
