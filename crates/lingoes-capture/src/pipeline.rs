use std::time::{Duration, Instant};

use crate::{CaptureConfig, CaptureContext, CaptureResult, TextCandidate, TextSource};

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CaptureError {
    #[error("capture unsupported: {0}")]
    Unsupported(&'static str),
    #[error("capture provider failed: {0}")]
    Provider(String),
    #[error("invalid captured text: {0}")]
    InvalidText(&'static str),
    #[error("captured text filtered: {0}")]
    Filtered(&'static str),
    #[error("capture timed out")]
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ClipboardContent {
    pub text: Option<String>,
}

pub trait ClipboardStore {
    fn snapshot(&mut self) -> Result<ClipboardContent, CaptureError>;
    fn restore(&mut self, content: ClipboardContent) -> Result<(), CaptureError>;
    fn text(&mut self) -> Result<Option<String>, CaptureError>;
}
pub trait UiAutomationCapture: Send + Sync {
    fn text_at_cursor(
        &self,
        context: &CaptureContext,
    ) -> Result<Option<TextCandidate>, CaptureError>;
    fn selected_text(
        &self,
        context: &CaptureContext,
    ) -> Result<Option<TextCandidate>, CaptureError>;
}
pub trait SelectionCapture: Send + Sync {
    fn selected_text(
        &self,
        context: &CaptureContext,
    ) -> Result<Option<TextCandidate>, CaptureError>;
    fn copy_selection(
        &self,
        clipboard: &mut dyn ClipboardStore,
        timeout: Duration,
    ) -> Result<(), CaptureError>;
}
pub trait OcrEngine: Send + Sync {
    fn recognize(&self, context: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError>;
}

#[derive(Debug, Clone)]
pub struct MockOcrEngine {
    result: Result<Option<TextCandidate>, CaptureError>,
}
impl MockOcrEngine {
    pub fn new(result: Option<TextCandidate>) -> Self {
        Self { result: Ok(result) }
    }
    pub fn unsupported() -> Self {
        Self {
            result: Err(CaptureError::Unsupported("OCR engine")),
        }
    }
}
impl OcrEngine for MockOcrEngine {
    fn recognize(&self, _: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError> {
        self.result.clone()
    }
}

pub fn capture_selection_with_clipboard(
    selection: &dyn SelectionCapture,
    clipboard: &mut dyn ClipboardStore,
    context: &CaptureContext,
    timeout: Duration,
) -> Result<Option<TextCandidate>, CaptureError> {
    if context.secure {
        return Ok(None);
    }
    if let Some(candidate) = selection.selected_text(context)? {
        return Ok(Some(candidate));
    }
    let snapshot = clipboard.snapshot()?;
    let capture = (|| {
        selection.copy_selection(clipboard, timeout)?;
        Ok(clipboard
            .text()?
            .filter(|text| !text.trim().is_empty())
            .map(|text| TextCandidate::new(text, TextSource::Clipboard)))
    })();
    let restore = clipboard.restore(snapshot);
    match (capture, restore) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (_, Err(error)) => Err(error),
    }
}

pub struct CapturePipeline<U, S, O> {
    uia: U,
    selection: S,
    ocr: O,
    config: CaptureConfig,
    last: Option<(String, Instant)>,
}
impl<U: UiAutomationCapture, S: SelectionCapture, O: OcrEngine> CapturePipeline<U, S, O> {
    pub fn new(uia: U, selection: S, ocr: O, config: CaptureConfig) -> Self {
        Self {
            uia,
            selection,
            ocr,
            config,
            last: None,
        }
    }
    pub fn capture(
        &mut self,
        context: &CaptureContext,
        clipboard: &mut dyn ClipboardStore,
        now: Instant,
    ) -> Result<Option<CaptureResult>, CaptureError> {
        if !self.config.allows(context) {
            return Ok(None);
        }
        let candidate = self
            .uia
            .text_at_cursor(context)?
            .or(self.selection.selected_text(context)?)
            .or(if self.config.selection_clipboard_fallback {
                capture_selection_with_clipboard(
                    &self.selection,
                    clipboard,
                    context,
                    self.config.delay,
                )?
            } else {
                None
            })
            .or(if self.config.ocr_enabled {
                self.ocr.recognize(context).ok().flatten()
            } else {
                None
            });
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let text = match self.config.filters.normalize(&candidate.text) {
            Ok(v) => v,
            Err(CaptureError::Filtered(_) | CaptureError::InvalidText(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        if self.last.as_ref().is_some_and(|(last, at)| {
            last == &text && now.saturating_duration_since(*at) < self.config.debounce
        }) {
            return Ok(None);
        }
        self.last = Some((text.clone(), now));
        Ok(Some(CaptureResult {
            text,
            source: candidate.source,
            context: context.clone(),
            bounds: candidate.bounds.filter(|r| r.is_valid()),
            confidence: candidate.confidence,
        }))
    }
}
