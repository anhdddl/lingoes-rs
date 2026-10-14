use std::collections::BTreeSet;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureMode {
    CursorWord,
    SelectedText,
    Clipboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    X1,
    X2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Activation {
    pub button: MouseButton,
    pub modifiers: Modifiers,
    pub double_click: bool,
}
impl Activation {
    pub fn double_click(button: MouseButton) -> Self {
        Self {
            button,
            modifiers: Modifiers::default(),
            double_click: true,
        }
    }
    #[must_use]
    pub fn with_ctrl(mut self, enabled: bool) -> Self {
        self.modifiers.ctrl = enabled;
        self
    }
}
impl Default for Activation {
    fn default() -> Self {
        Self {
            button: MouseButton::Middle,
            modifiers: Modifiers::default(),
            double_click: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Rect {
    pub fn is_valid(self) -> bool {
        self.right >= self.left && self.bottom >= self.top
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CaptureContext {
    pub process_name: String,
    pub window_class: String,
    pub window_title: String,
    pub secure: bool,
    pub point: Option<Point>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSource {
    UiAutomation,
    Selection,
    Clipboard,
    Ocr,
}
#[derive(Debug, Clone, PartialEq)]
pub struct TextCandidate {
    pub text: String,
    pub source: TextSource,
    pub bounds: Option<Rect>,
    pub confidence: Option<f32>,
}
impl TextCandidate {
    pub fn new(text: impl Into<String>, source: TextSource) -> Self {
        Self {
            text: text.into(),
            source,
            bounds: None,
            confidence: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct CaptureResult {
    pub text: String,
    pub source: TextSource,
    pub context: CaptureContext,
    pub bounds: Option<Rect>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Script {
    Latin,
    Cyrillic,
    Arabic,
    Hebrew,
    Cjk,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFilters {
    pub phrase_matching: bool,
    pub max_phrase_words: usize,
    pub allowed_scripts: BTreeSet<Script>,
    pub allow_numbers: bool,
    pub min_chars: usize,
    pub max_chars: usize,
}
impl Default for TextFilters {
    fn default() -> Self {
        Self {
            phrase_matching: false,
            max_phrase_words: 1,
            allowed_scripts: BTreeSet::new(),
            allow_numbers: true,
            min_chars: 1,
            max_chars: 2_000,
        }
    }
}
impl TextFilters {
    pub fn normalize(&self, input: &str) -> Result<String, crate::CaptureError> {
        if input
            .chars()
            .any(|c| c == '\0' || (c.is_control() && !c.is_whitespace()))
        {
            return Err(crate::CaptureError::InvalidText("control character"));
        }
        let collapsed = input.split_whitespace().collect::<Vec<_>>().join(" ");
        let value = collapsed
            .trim_matches(|c: char| c.is_ascii_punctuation() && c != '\'' && c != '-')
            .to_owned();
        let count = value.chars().count();
        if count < self.min_chars || count > self.max_chars {
            return Err(crate::CaptureError::InvalidText(
                "length outside configured bounds",
            ));
        }
        let words = value.split_whitespace().count();
        if (!self.phrase_matching && words > 1) || words > self.max_phrase_words.max(1) {
            return Err(crate::CaptureError::InvalidText(
                "phrase exceeds configured bounds",
            ));
        }
        let mut has_letter = false;
        let mut has_number = false;
        for c in value.chars() {
            if c.is_numeric() {
                has_number = true;
                continue;
            }
            if c.is_alphabetic() {
                has_letter = true;
                let script = script_of(c);
                if !self.allowed_scripts.is_empty() && !self.allowed_scripts.contains(&script) {
                    return Err(crate::CaptureError::Filtered("script is disabled"));
                }
            }
        }
        if has_number && (!self.allow_numbers || !has_letter) {
            return Err(crate::CaptureError::Filtered("numbers are disabled"));
        }
        Ok(value)
    }
}

fn script_of(c: char) -> Script {
    match c as u32 {
        0x0041..=0x024f | 0x1e00..=0x1eff => Script::Latin,
        0x0400..=0x052f => Script::Cyrillic,
        0x0590..=0x05ff => Script::Hebrew,
        0x0600..=0x08ff => Script::Arabic,
        0x3040..=0x30ff | 0x3400..=0x9fff | 0xac00..=0xd7af => Script::Cjk,
        _ => Script::Other,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchAction {
    Allow,
    Deny,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRule {
    pub process: Option<String>,
    pub window_class: Option<String>,
    pub action: MatchAction,
}
impl FilterRule {
    pub fn process(value: impl Into<String>, action: MatchAction) -> Self {
        Self {
            process: Some(value.into()),
            window_class: None,
            action,
        }
    }
    pub fn window_class(value: impl Into<String>, action: MatchAction) -> Self {
        Self {
            process: None,
            window_class: Some(value.into()),
            action,
        }
    }
    fn matches(&self, context: &CaptureContext) -> bool {
        self.process
            .as_ref()
            .is_none_or(|v| wildcard_eq(v, &context.process_name))
            && self
                .window_class
                .as_ref()
                .is_none_or(|v| wildcard_eq(v, &context.window_class))
    }
}
fn wildcard_eq(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        true
    } else {
        pattern.eq_ignore_ascii_case(value)
    }
}

#[derive(Debug, Clone)]
pub struct CaptureConfig {
    pub mode: CaptureMode,
    pub activation: Activation,
    pub delay: Duration,
    pub debounce: Duration,
    pub selection_clipboard_fallback: bool,
    pub ocr_enabled: bool,
    pub filters: TextFilters,
    pub rules: Vec<FilterRule>,
}
impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            mode: CaptureMode::CursorWord,
            activation: Activation::default(),
            delay: Duration::from_millis(150),
            debounce: Duration::from_millis(300),
            selection_clipboard_fallback: false,
            ocr_enabled: false,
            filters: TextFilters::default(),
            rules: Vec::new(),
        }
    }
}
impl CaptureConfig {
    pub fn allows(&self, context: &CaptureContext) -> bool {
        if context.secure || context.process_name.trim().is_empty() {
            return false;
        }
        if self
            .rules
            .iter()
            .any(|r| r.action == MatchAction::Deny && r.matches(context))
        {
            return false;
        }
        let allows: Vec<_> = self
            .rules
            .iter()
            .filter(|r| r.action == MatchAction::Allow)
            .collect();
        allows.is_empty() || allows.iter().any(|r| r.matches(context))
    }
}
