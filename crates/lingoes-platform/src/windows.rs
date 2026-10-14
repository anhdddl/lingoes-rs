use std::collections::HashSet;
use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use windows::Win32::Foundation::{
    BOOL, ERROR_HOTKEY_ALREADY_REGISTERED, GetLastError, HWND, LPARAM, LRESULT, POINT, RECT,
    SetLastError, WIN32_ERROR, WPARAM,
};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETVERSION,
    NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWL_EXSTYLE, GetCursorPos,
    GetMessageW, GetWindowLongPtrW, HICON, HWND_MESSAGE, HWND_NOTOPMOST, HWND_TOPMOST,
    IDI_APPLICATION, IsWindow, LoadIconW, MSG, PostMessageW, PostQuitMessage, RegisterClassW,
    RegisterWindowMessageW, SET_WINDOW_POS_FLAGS, SW_HIDE, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_DESTROY, WM_HOTKEY, WNDCLASSW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};
use windows::core::PCWSTR;

use crate::{
    GlobalHotkeyService, Hotkey, HotkeyEvent, HotkeyId, PlatformError, TrayService, TrayState,
};

const TRAY_ICON_ID: u32 = 1;
const WM_TRAY: u32 = WM_APP + 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageHostEvent {
    Hotkey(HotkeyEvent),
    TrayActivated,
    TaskbarCreated,
}

struct HostState {
    events: Sender<MessageHostEvent>,
    hotkeys: HashSet<HotkeyId>,
    tray_installed: bool,
    tooltip: String,
    tray_state: TrayState,
    taskbar_created: u32,
}

pub struct WindowsMessageHost {
    hwnd: HWND,
    state: Arc<Mutex<HostState>>,
    events: Receiver<MessageHostEvent>,
}

// The HWND and its state are used only on the thread that creates the host. The Send
// implementation allows ownership to move to that message-loop thread before use.
unsafe impl Send for WindowsMessageHost {}

impl WindowsMessageHost {
    pub fn create() -> Result<Self, PlatformError> {
        let (event_sender, events) = mpsc::channel();
        let class_name = wide("LexiForgeMessageHost");
        let taskbar_name = wide("TaskbarCreated");
        // SAFETY: taskbar_name is a stable NUL-terminated UTF-16 buffer for the call.
        let taskbar_created = unsafe { RegisterWindowMessageW(PCWSTR(taskbar_name.as_ptr())) };
        let state = Arc::new(Mutex::new(HostState {
            events: event_sender,
            hotkeys: HashSet::new(),
            tray_installed: false,
            tooltip: String::new(),
            tray_state: TrayState::default(),
            taskbar_created,
        }));
        let window_class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        // SAFETY: window_class references class_name for this call, and the callback has the
        // required system ABI.
        let atom = unsafe { RegisterClassW(&window_class) };
        if atom == 0 {
            return Err(last_error("cannot register message-host window class"));
        }
        let raw_state = Arc::into_raw(Arc::clone(&state));
        // SAFETY: class_name is registered above. HWND_MESSAGE creates a non-visible host. The
        // raw Arc pointer remains valid until WM_DESTROY or explicit cleanup on failure.
        let hwnd = match unsafe {
            CreateWindowExW(
                Default::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR::null(),
                Default::default(),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                None,
                Some(raw_state.cast()),
            )
        } {
            Ok(hwnd) => hwnd,
            Err(error) => {
                // SAFETY: CreateWindowExW failed, so reclaim the raw pointer.
                drop(unsafe { Arc::from_raw(raw_state) });
                return Err(PlatformError::Other(format!(
                    "cannot create message-only window: {error}"
                )));
            }
        };
        Ok(Self {
            hwnd,
            state,
            events,
        })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    pub fn try_event(&self) -> Option<MessageHostEvent> {
        self.events.try_recv().ok()
    }

    pub fn run_message_loop(&self) -> Result<(), PlatformError> {
        let mut message = MSG::default();
        loop {
            // SAFETY: message points to valid writable storage; this thread owns the window.
            let result = unsafe { GetMessageW(&mut message, None, 0, 0) };
            if result.0 == -1 {
                return Err(last_error("message loop failed"));
            }
            if result.0 == 0 {
                return Ok(());
            }
            // SAFETY: message was initialized by GetMessageW.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    pub fn request_shutdown(&self) -> Result<(), PlatformError> {
        // SAFETY: hwnd is owned by this host and WM_DESTROY is handled by window_proc.
        unsafe { PostMessageW(self.hwnd, WM_DESTROY, WPARAM(0), LPARAM(0)) }
            .map_err(|error| PlatformError::Other(error.to_string()))
    }

    fn tray_data(&self, flags: u32) -> NOTIFYICONDATAW {
        let state = self.state.lock().expect("message host state poisoned");
        tray_data(self.hwnd, flags, &state.tooltip)
    }
}

impl GlobalHotkeyService for WindowsMessageHost {
    fn register(&mut self, id: HotkeyId, hotkey: Hotkey) -> Result<(), PlatformError> {
        // SAFETY: hwnd belongs to this host; the numeric hotkey definition is caller-owned data.
        if let Err(error) = unsafe {
            RegisterHotKey(
                self.hwnd,
                id.0 as i32,
                windows::Win32::UI::Input::KeyboardAndMouse::HOT_KEY_MODIFIERS(hotkey.modifiers),
                hotkey.virtual_key,
            )
        } {
            // SAFETY: GetLastError is read immediately after the failed Win32 call.
            if unsafe { GetLastError() } == ERROR_HOTKEY_ALREADY_REGISTERED {
                return Err(PlatformError::Conflict(format!(
                    "hotkey {} is already registered",
                    id.0
                )));
            }
            return Err(PlatformError::Other(error.to_string()));
        }
        self.state
            .lock()
            .expect("message host state poisoned")
            .hotkeys
            .insert(id);
        Ok(())
    }

    fn unregister(&mut self, id: HotkeyId) -> Result<(), PlatformError> {
        let removed = self
            .state
            .lock()
            .expect("message host state poisoned")
            .hotkeys
            .remove(&id);
        if removed {
            // SAFETY: this host registered this identifier on this HWND.
            unsafe { UnregisterHotKey(self.hwnd, id.0 as i32) }
                .map_err(|error| PlatformError::Other(error.to_string()))?;
        }
        Ok(())
    }

    fn disable_all(&mut self) -> Result<(), PlatformError> {
        let ids: Vec<_> = self
            .state
            .lock()
            .expect("message host state poisoned")
            .hotkeys
            .iter()
            .copied()
            .collect();
        for id in ids {
            self.unregister(id)?;
        }
        Ok(())
    }
}

impl TrayService for WindowsMessageHost {
    fn install(&mut self, tooltip: &str, state: TrayState) -> Result<(), PlatformError> {
        {
            let mut host = self.state.lock().expect("message host state poisoned");
            host.tooltip = tooltip.to_owned();
            host.tray_state = state;
        }
        let mut data = self.tray_data((NIF_MESSAGE | NIF_ICON | NIF_TIP).0);
        // SAFETY: data is fully initialized and contains an HWND owned by this host.
        if !unsafe { Shell_NotifyIconW(NIM_ADD, &data) }.as_bool() {
            return Err(last_error("cannot add tray icon"));
        }
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        // SAFETY: data identifies the tray icon added immediately above.
        if !unsafe { Shell_NotifyIconW(NIM_SETVERSION, &data) }.as_bool() {
            let _ = unsafe { Shell_NotifyIconW(NIM_DELETE, &data) };
            return Err(last_error("cannot set tray icon version"));
        }
        self.state
            .lock()
            .expect("message host state poisoned")
            .tray_installed = true;
        Ok(())
    }

    fn update(&mut self, state: TrayState) -> Result<(), PlatformError> {
        self.state
            .lock()
            .expect("message host state poisoned")
            .tray_state = state;
        let data = self.tray_data((NIF_MESSAGE | NIF_ICON | NIF_TIP).0);
        // SAFETY: data identifies an icon installed by this host.
        if !unsafe { Shell_NotifyIconW(NIM_MODIFY, &data) }.as_bool() {
            return Err(last_error("cannot update tray icon"));
        }
        Ok(())
    }

    fn remove(&mut self) -> Result<(), PlatformError> {
        let installed = self
            .state
            .lock()
            .expect("message host state poisoned")
            .tray_installed;
        if installed {
            let data = self.tray_data(0);
            // SAFETY: data identifies an icon installed by this host.
            if !unsafe { Shell_NotifyIconW(NIM_DELETE, &data) }.as_bool() {
                return Err(last_error("cannot remove tray icon"));
            }
            self.state
                .lock()
                .expect("message host state poisoned")
                .tray_installed = false;
        }
        Ok(())
    }
}

impl Drop for WindowsMessageHost {
    fn drop(&mut self) {
        let _ = self.disable_all();
        let _ = self.remove();
        // SAFETY: hwnd is exclusively owned by this host and is still valid unless its message
        // loop already handled WM_DESTROY.
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        CREATESTRUCTW, GWLP_USERDATA, GetWindowLongPtrW, SetWindowLongPtrW, WM_NCCREATE,
        WM_NCDESTROY,
    };

    if message == WM_NCCREATE {
        let create = lparam.0 as *const CREATESTRUCTW;
        // SAFETY: WM_NCCREATE supplies a valid CREATESTRUCTW and lpCreateParams is the Arc pointer
        // passed to CreateWindowExW.
        let raw = unsafe { (*create).lpCreateParams as *const Mutex<HostState> };
        // SAFETY: hwnd is being initialized and the pointer fits LONG_PTR.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, raw as isize) };
    }
    // SAFETY: the value was installed during WM_NCCREATE and remains valid through WM_NCDESTROY.
    let raw = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Mutex<HostState>;
    if !raw.is_null() {
        // SAFETY: raw is backed by an Arc retained for the HWND lifetime.
        let state = unsafe { &*raw };
        let guard = state.lock().expect("message host state poisoned");
        if message == WM_HOTKEY {
            let _ = guard
                .events
                .send(MessageHostEvent::Hotkey(HotkeyEvent::Pressed(HotkeyId(
                    wparam.0 as u32,
                ))));
            return LRESULT(0);
        }
        if message == WM_TRAY {
            let _ = guard.events.send(MessageHostEvent::TrayActivated);
            return LRESULT(0);
        }
        if message == guard.taskbar_created {
            let _ = guard.events.send(MessageHostEvent::TaskbarCreated);
            if guard.tray_installed {
                let data = tray_data(hwnd, (NIF_MESSAGE | NIF_ICON | NIF_TIP).0, &guard.tooltip);
                // SAFETY: Explorer requested tray recreation and data references this live HWND.
                let _ = unsafe { Shell_NotifyIconW(NIM_ADD, &data) };
            }
            return LRESULT(0);
        }
        drop(guard);
    }
    if message == WM_DESTROY {
        // SAFETY: called on the owner message-loop thread.
        unsafe { PostQuitMessage(0) };
        return LRESULT(0);
    }
    if message == WM_NCDESTROY && !raw.is_null() {
        // SAFETY: this consumes exactly the Arc reference transferred at CreateWindowExW.
        drop(unsafe { Arc::from_raw(raw) });
        // SAFETY: clears stale user data before returning.
        unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
    }
    // SAFETY: all unhandled messages are delegated to the system default procedure.
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn tray_data(hwnd: HWND, flags: u32, tooltip: &str) -> NOTIFYICONDATAW {
    // SAFETY: IDI_APPLICATION is a system-owned icon resource and needs no destruction.
    let icon = unsafe { LoadIconW(None, IDI_APPLICATION) }.unwrap_or(HICON::default());
    let mut data = NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: windows::Win32::UI::Shell::NOTIFY_ICON_DATA_FLAGS(flags),
        uCallbackMessage: WM_TRAY,
        hIcon: icon,
        ..Default::default()
    };
    let tooltip = wide(tooltip);
    let count = tooltip.len().min(data.szTip.len());
    data.szTip[..count].copy_from_slice(&tooltip[..count]);
    data
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

#[derive(Debug, Clone, Copy)]
struct WindowConfiguration {
    extended_style: WINDOW_EX_STYLE,
    position_flags: SET_WINDOW_POS_FLAGS,
}

fn window_configuration(behavior: crate::popup::PopupNativeBehavior) -> WindowConfiguration {
    let mut extended_style = WINDOW_EX_STYLE(0);
    if behavior.no_activate {
        extended_style |= WS_EX_NOACTIVATE;
    }
    if behavior.tool_window {
        extended_style |= WS_EX_TOOLWINDOW;
    }

    let mut position_flags = SWP_SHOWWINDOW;
    if !behavior.activate {
        position_flags |= SWP_NOACTIVATE;
    }
    WindowConfiguration {
        extended_style,
        position_flags,
    }
}

/// Adapts a caller-owned Win32 window to the popup policy in [`crate::popup`].
///
/// This adapter never destroys the window. The caller must keep the `HWND` alive
/// and invoke it on a thread that is allowed to manipulate that window.
#[derive(Debug, Clone, Copy)]
pub struct WindowsPopupWindowAdapter {
    hwnd: HWND,
}

impl WindowsPopupWindowAdapter {
    /// Wraps a caller-owned window after validating that it currently exists.
    pub fn new(hwnd: HWND) -> Result<Self, PlatformError> {
        // SAFETY: `IsWindow` only inspects the supplied handle. It does not dereference a
        // Rust pointer; stale or null handles are reported as invalid.
        if !unsafe { IsWindow(hwnd) }.as_bool() {
            return Err(PlatformError::Other(
                "popup HWND is not a live window".into(),
            ));
        }
        Ok(Self { hwnd })
    }

    /// Wraps the raw integer representation used by windowing-toolkit APIs.
    pub fn from_raw(raw: isize) -> Result<Self, PlatformError> {
        Self::new(HWND(raw as *mut _))
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Applies native popup styles without making the caller-owned window visible.
    pub fn prepare(&self, activation: crate::popup::PopupActivation) -> Result<(), PlatformError> {
        let behavior = crate::popup::native_behavior(activation);
        let configuration = window_configuration(behavior);
        let style_changed = self.apply_extended_style(configuration.extended_style)?;
        if style_changed {
            // SAFETY: the HWND is caller-owned and validated. This refreshes the non-client frame
            // without moving, resizing, activating, changing z-order, or showing the window.
            unsafe {
                SetWindowPos(
                    self.hwnd,
                    HWND_NOTOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
                )
            }
            .map_err(|error| {
                PlatformError::Other(format!("cannot prepare popup window: {error}"))
            })?;
        }
        Ok(())
    }

    /// Applies native style policy and shows the popup at a computed physical rect.
    pub fn show(
        &self,
        activation: crate::popup::PopupActivation,
        rect: crate::popup::PhysicalRect,
    ) -> Result<(), PlatformError> {
        if rect.is_empty() {
            return Err(PlatformError::Other("popup rectangle is empty".into()));
        }
        let behavior = crate::popup::native_behavior(activation);
        let configuration = window_configuration(behavior);
        let style_changed = self.apply_extended_style(configuration.extended_style)?;

        let width = rect.right.checked_sub(rect.left).ok_or_else(|| {
            PlatformError::Other("popup rectangle width exceeds Win32 range".into())
        })?;
        let height = rect.bottom.checked_sub(rect.top).ok_or_else(|| {
            PlatformError::Other("popup rectangle height exceeds Win32 range".into())
        })?;
        let insert_after = if behavior.topmost {
            HWND_TOPMOST
        } else {
            HWND_NOTOPMOST
        };
        // SAFETY: the HWND is caller-owned and was validated at construction. Coordinates and
        // flags are plain values; SetWindowPos validates the handle again atomically.
        unsafe {
            SetWindowPos(
                self.hwnd,
                insert_after,
                rect.left,
                rect.top,
                width,
                height,
                configuration.position_flags
                    | if style_changed {
                        SWP_FRAMECHANGED
                    } else {
                        SET_WINDOW_POS_FLAGS(0)
                    },
            )
        }
        .map_err(|error| PlatformError::Other(format!("cannot show popup window: {error}")))
    }

    /// Hides the popup and removes its topmost status so it cannot remain above other apps.
    pub fn hide(&self) -> Result<(), PlatformError> {
        // SAFETY: the HWND is caller-owned and was validated at construction. This call changes
        // only z-order; it deliberately avoids activation, movement, and resizing.
        unsafe {
            SetWindowPos(
                self.hwnd,
                HWND_NOTOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            )
        }
        .map_err(|error| {
            PlatformError::Other(format!("cannot remove popup topmost state: {error}"))
        })?;
        // SAFETY: ShowWindow accepts a live HWND and SW_HIDE; its return value is the prior
        // visibility state, not an error indicator.
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
        Ok(())
    }

    fn apply_extended_style(&self, desired: WINDOW_EX_STYLE) -> Result<bool, PlatformError> {
        // GetWindowLongPtrW and SetWindowLongPtrW use zero as both a valid result and an error
        // sentinel, so clear and inspect last-error around each call.
        // SAFETY: the adapter holds a validated HWND and GWL_EXSTYLE selects an integer style.
        unsafe { SetLastError(WIN32_ERROR(0)) };
        let current = unsafe { GetWindowLongPtrW(self.hwnd, GWL_EXSTYLE) };
        if current == 0 {
            // SAFETY: reads thread-local Win32 last-error immediately after the ambiguous call.
            let error = unsafe { GetLastError() };
            if error != WIN32_ERROR(0) {
                return Err(last_error_code("cannot read popup extended style", error));
            }
        }
        let controlled = WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0;
        let next = (current as u32 & !controlled) | desired.0;
        if next == current as u32 {
            return Ok(false);
        }

        // SAFETY: same validated HWND; unrelated extended-style bits are preserved.
        unsafe { SetLastError(WIN32_ERROR(0)) };
        let previous = unsafe { SetWindowLongPtrW(self.hwnd, GWL_EXSTYLE, next as isize) };
        if previous == 0 {
            // SAFETY: reads thread-local Win32 last-error immediately after the ambiguous call.
            let error = unsafe { GetLastError() };
            if error != WIN32_ERROR(0) {
                return Err(last_error_code("cannot set popup extended style", error));
            }
        }
        Ok(true)
    }
}

pub fn popup_cursor_position() -> Result<crate::popup::PhysicalPoint, PlatformError> {
    let mut point = POINT::default();
    // SAFETY: point is valid writable storage for the duration of the call.
    unsafe { GetCursorPos(&mut point) }
        .map_err(|error| PlatformError::Other(format!("cannot read cursor position: {error}")))?;
    Ok(crate::popup::PhysicalPoint::new(point.x, point.y))
}

/// Enumerates physical monitor work areas and effective DPI for popup placement.
pub fn popup_monitors() -> Result<Vec<crate::popup::Monitor>, PlatformError> {
    struct Enumeration {
        monitors: Vec<crate::popup::Monitor>,
        error: Option<PlatformError>,
    }

    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _: HDC,
        _: *mut RECT,
        data: LPARAM,
    ) -> BOOL {
        // SAFETY: `data` points to the Enumeration borrowed for the synchronous
        // EnumDisplayMonitors call below.
        let enumeration = unsafe { &mut *(data.0 as *mut Enumeration) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: monitor is supplied by EnumDisplayMonitors and info is writable.
        if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            enumeration.error = Some(last_error("cannot read monitor work area"));
            return BOOL(0);
        }
        let mut dpi_x = 96;
        let mut dpi_y = 96;
        // SAFETY: monitor is valid for the callback and both DPI outputs are writable. A DPI
        // query can fail on older configurations; 96 DPI is a safe placement fallback.
        let _ = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) };
        enumeration.monitors.push(crate::popup::Monitor {
            id: monitor.0 as usize as u64,
            work_area: crate::popup::PhysicalRect::new(
                info.rcWork.left,
                info.rcWork.top,
                info.rcWork.right,
                info.rcWork.bottom,
            ),
            dpi: dpi_x,
        });
        BOOL(1)
    }

    let mut enumeration = Enumeration {
        monitors: Vec::new(),
        error: None,
    };
    // SAFETY: callback is valid and receives a pointer to `enumeration`; enumeration remains
    // alive because EnumDisplayMonitors invokes callbacks synchronously.
    let completed = unsafe {
        EnumDisplayMonitors(
            HDC::default(),
            None,
            Some(callback),
            LPARAM((&mut enumeration as *mut Enumeration) as isize),
        )
    };
    if let Some(error) = enumeration.error {
        return Err(error);
    }
    if !completed.as_bool() {
        return Err(last_error("cannot enumerate monitors"));
    }
    Ok(enumeration.monitors)
}

fn last_error_code(context: &str, code: WIN32_ERROR) -> PlatformError {
    PlatformError::Other(format!("{context}: Windows error {}", code.0))
}

fn last_error(context: &str) -> PlatformError {
    // SAFETY: GetLastError has no preconditions.
    let code = unsafe { GetLastError() };
    PlatformError::Other(format!("{context}: Windows error {}", code.0))
}

#[cfg(test)]
mod popup_adapter_tests {
    use super::*;
    use crate::popup::{PopupActivation, native_behavior};
    use windows::Win32::UI::WindowsAndMessaging::{
        SWP_NOACTIVATE, SWP_SHOWWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    #[test]
    fn passive_policy_maps_to_nonactivating_tool_window_flags() {
        let configuration = window_configuration(native_behavior(PopupActivation::Passive));

        assert!(configuration.extended_style.contains(WS_EX_NOACTIVATE));
        assert!(configuration.extended_style.contains(WS_EX_TOOLWINDOW));
        assert!(configuration.position_flags.contains(SWP_NOACTIVATE));
        assert!(configuration.position_flags.contains(SWP_SHOWWINDOW));
    }

    #[test]
    fn keyboard_policy_removes_only_the_no_activate_flags() {
        let configuration =
            window_configuration(native_behavior(PopupActivation::KeyboardRequested));

        assert!(!configuration.extended_style.contains(WS_EX_NOACTIVATE));
        assert!(configuration.extended_style.contains(WS_EX_TOOLWINDOW));
        assert!(!configuration.position_flags.contains(SWP_NOACTIVATE));
        assert!(configuration.position_flags.contains(SWP_SHOWWINDOW));
    }
}
