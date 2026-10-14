//! Windows-native capture adapters. Runtime behavior requires an interactive desktop.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, RemoveClipboardFormatListener,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTextPattern,
    IUIAutomationTextPattern2, IUIAutomationTextRange, TextUnit_Word, UIA_TextPattern2Id,
    UIA_TextPatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DispatchMessageW, GWLP_USERDATA, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, HWND_MESSAGE, MSG, PostMessageW, RegisterClassW,
    SetWindowLongPtrW, TranslateMessage, WM_CLIPBOARDUPDATE, WM_CLOSE, WM_NCCREATE, WM_NCDESTROY,
    WNDCLASSW,
};
use windows::core::{Interface, PCWSTR};

use crate::{
    CaptureContext, CaptureError, ClipboardEventProcessor, TextCandidate, TextSource,
    UiAutomationCapture,
};

pub struct WindowsClipboardListener {
    hwnd: HWND,
    running: Arc<AtomicBool>,
}
unsafe impl Send for WindowsClipboardListener {}
impl WindowsClipboardListener {
    pub fn spawn<F, E>(coalesce: Duration, callback: F, emit: E) -> Result<Self, CaptureError>
    where
        F: FnMut() -> Option<String> + Send + 'static,
        E: FnMut(String) + Send + 'static,
    {
        let running = Arc::new(AtomicBool::new(true));
        let running_thread = Arc::clone(&running);
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        std::thread::spawn(move || {
            let state = Box::new(ClipboardWindowState {
                callback: Mutex::new(Box::new(callback)),
                emit: Mutex::new(Box::new(emit)),
                processor: Mutex::new(ClipboardEventProcessor::new(coalesce)),
            });
            let class = wide("LexiForgeClipboardListener");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(clipboard_window_proc),
                lpszClassName: PCWSTR(class.as_ptr()),
                ..Default::default()
            };
            // SAFETY: class and callback remain valid for registration and the message loop.
            if unsafe { RegisterClassW(&wc) } == 0 {
                let _ = tx.send(None);
                return;
            }
            let raw = Box::into_raw(state);
            // SAFETY: registered class and raw state pointer satisfy CreateWindowExW contract.
            let hwnd = unsafe {
                CreateWindowExW(
                    Default::default(),
                    PCWSTR(class.as_ptr()),
                    PCWSTR::null(),
                    Default::default(),
                    0,
                    0,
                    0,
                    0,
                    HWND_MESSAGE,
                    None,
                    None,
                    Some(raw.cast()),
                )
            };
            let Ok(hwnd) = hwnd else {
                drop(unsafe { Box::from_raw(raw) });
                let _ = tx.send(None);
                return;
            };
            // SAFETY: hwnd is a live message-only window.
            if unsafe { AddClipboardFormatListener(hwnd) }.is_err() {
                let _ = tx.send(None);
                return;
            }
            let _ = tx.send(Some(hwnd.0 as isize));
            let mut message = MSG::default();
            while running_thread.load(Ordering::Relaxed) {
                // SAFETY: valid MSG storage and owner thread message queue.
                if unsafe { GetMessageW(&mut message, None, 0, 0) }.0 <= 0 {
                    break;
                }
                unsafe {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let _ = unsafe { RemoveClipboardFormatListener(hwnd) };
        });
        let raw = rx
            .recv()
            .map_err(|e| CaptureError::Provider(e.to_string()))?
            .ok_or_else(|| {
                CaptureError::Provider("cannot create clipboard listener window".into())
            })?;
        Ok(Self {
            hwnd: HWND(raw as *mut _),
            running,
        })
    }
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        let _ = unsafe { PostMessageW(self.hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)) };
    }
}
impl Drop for WindowsClipboardListener {
    fn drop(&mut self) {
        self.stop();
    }
}

type ClipboardCallback = Box<dyn FnMut() -> Option<String> + Send>;
type ClipboardEmitter = Box<dyn FnMut(String) + Send>;
struct ClipboardWindowState {
    callback: Mutex<ClipboardCallback>,
    emit: Mutex<ClipboardEmitter>,
    processor: Mutex<ClipboardEventProcessor>,
}
unsafe extern "system" fn clipboard_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_NCCREATE {
        let create = lparam.0 as *const CREATESTRUCTW;
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as isize);
        }
    }
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *mut ClipboardWindowState;
    if message == WM_CLIPBOARDUPDATE && !raw.is_null() {
        let state = unsafe { &*raw };
        if let Some(text) = state.callback.lock().expect("clipboard callback poisoned")() {
            if let Some(event) = state
                .processor
                .lock()
                .expect("clipboard processor poisoned")
                .on_update(&text, Instant::now())
            {
                state.emit.lock().expect("clipboard emitter poisoned")(event);
            }
        }
        return LRESULT(0);
    }
    if message == WM_NCDESTROY && !raw.is_null() {
        drop(unsafe { Box::from_raw(raw) });
        unsafe {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        }
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

#[derive(Debug, Default)]
pub struct WindowsUiAutomation;
impl UiAutomationCapture for WindowsUiAutomation {
    fn text_at_cursor(&self, _: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError> {
        capture_uia(false)
    }
    fn selected_text(&self, _: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError> {
        capture_uia(true)
    }
}

fn capture_uia(selection: bool) -> Result<Option<TextCandidate>, CaptureError> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| CaptureError::Provider(e.to_string()))?;
    }
    let automation: IUIAutomation =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }
            .map_err(|e| CaptureError::Provider(e.to_string()))?;
    let mut point = POINT::default();
    if unsafe { GetCursorPos(&mut point) }.is_err() {
        return Ok(None);
    }
    let range = if selection {
        let element = unsafe { automation.GetFocusedElement() }
            .map_err(|e| CaptureError::Provider(e.to_string()))?;
        selection_range(&element)?
    } else {
        let element = unsafe { automation.ElementFromPoint(point) }
            .map_err(|e| CaptureError::Provider(e.to_string()))?;
        point_range(&element, point)?
    };
    let Some(range) = range else { return Ok(None) };
    if !selection {
        unsafe { range.ExpandToEnclosingUnit(TextUnit_Word) }
            .map_err(|e| CaptureError::Provider(e.to_string()))?;
    }
    let text = unsafe { range.GetText(2_001) }
        .map_err(|e| CaptureError::Provider(e.to_string()))?
        .to_string();
    let text = text.trim();
    if text.is_empty() || text.chars().count() > 2_000 {
        return Ok(None);
    }
    Ok(Some(TextCandidate {
        text: text.into(),
        source: if selection {
            TextSource::Selection
        } else {
            TextSource::UiAutomation
        },
        bounds: None,
        confidence: None,
    }))
}
fn point_range(
    element: &IUIAutomationElement,
    point: POINT,
) -> Result<Option<IUIAutomationTextRange>, CaptureError> {
    if let Ok(unknown) = unsafe { element.GetCurrentPattern(UIA_TextPattern2Id) } {
        if let Ok(pattern) = unknown.cast::<IUIAutomationTextPattern2>() {
            return unsafe { pattern.RangeFromPoint(point) }
                .map(Some)
                .map_err(|e| CaptureError::Provider(e.to_string()));
        }
    }
    if let Ok(unknown) = unsafe { element.GetCurrentPattern(UIA_TextPatternId) } {
        if let Ok(pattern) = unknown.cast::<IUIAutomationTextPattern>() {
            return unsafe { pattern.RangeFromPoint(point) }
                .map(Some)
                .map_err(|e| CaptureError::Provider(e.to_string()));
        }
    }
    Ok(None)
}
fn selection_range(
    element: &IUIAutomationElement,
) -> Result<Option<IUIAutomationTextRange>, CaptureError> {
    let Ok(unknown) = (unsafe { element.GetCurrentPattern(UIA_TextPatternId) }) else {
        return Ok(None);
    };
    let pattern: IUIAutomationTextPattern = unknown
        .cast()
        .map_err(|e| CaptureError::Provider(e.to_string()))?;
    let ranges =
        unsafe { pattern.GetSelection() }.map_err(|e| CaptureError::Provider(e.to_string()))?;
    if unsafe { ranges.Length() }.unwrap_or(0) == 0 {
        return Ok(None);
    }
    unsafe { ranges.GetElement(0) }
        .map(Some)
        .map_err(|e| CaptureError::Provider(e.to_string()))
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[derive(Debug, Default)]
pub struct WindowsOcrEngine;
impl crate::OcrEngine for WindowsOcrEngine {
    fn recognize(&self, _: &CaptureContext) -> Result<Option<TextCandidate>, CaptureError> {
        Err(CaptureError::Unsupported(
            "Windows OCR adapter requires bounded screen bitmap integration",
        ))
    }
}
