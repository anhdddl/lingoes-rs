pub mod ipc;
pub mod popup;
pub mod settings;
pub mod web_search;

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    Unsupported(&'static str),
    Conflict(String),
    Other(String),
}

impl Display for PlatformError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(capability) => write!(formatter, "{capability} is unsupported"),
            Self::Conflict(message) | Self::Other(message) => formatter.write_str(message),
        }
    }
}

impl Error for PlatformError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HotkeyId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hotkey {
    pub modifiers: u32,
    pub virtual_key: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Pressed(HotkeyId),
}

pub trait GlobalHotkeyService: Send {
    fn register(&mut self, id: HotkeyId, hotkey: Hotkey) -> Result<(), PlatformError>;
    fn unregister(&mut self, id: HotkeyId) -> Result<(), PlatformError>;
    fn disable_all(&mut self) -> Result<(), PlatformError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowMain,
    ShowMini,
    OpenSettings,
    OpenDictionaries,
    ToggleClipboardCapture,
    ToggleCursorCapture,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrayState {
    pub clipboard_capture: bool,
    pub cursor_capture: bool,
}

pub trait TrayService: Send {
    fn install(&mut self, tooltip: &str, state: TrayState) -> Result<(), PlatformError>;
    fn update(&mut self, state: TrayState) -> Result<(), PlatformError>;
    fn remove(&mut self) -> Result<(), PlatformError>;
}

pub trait WindowLocator: Send + Sync {
    fn foreground_window(&self) -> Result<Option<u64>, PlatformError>;
}

pub trait TextCaptureProvider: Send + Sync {
    fn capture_text(&self) -> Result<Option<String>, PlatformError>;
}

pub trait RegionCaptureProvider: Send + Sync {
    fn capture_region(&self) -> Result<Vec<u8>, PlatformError>;
}

pub trait OcrProvider: Send + Sync {
    fn recognize(&self, image: &[u8]) -> Result<Option<String>, PlatformError>;
}

pub trait PrintService: Send + Sync {
    fn print(&self, document: &str) -> Result<(), PlatformError>;
}

pub trait SecretStore: Send + Sync {
    fn load(&self, key: &str) -> Result<Option<String>, PlatformError>;
    fn store(&self, key: &str, value: &str) -> Result<(), PlatformError>;
}

pub trait AutostartService: Send + Sync {
    fn set_enabled(&self, enabled: bool) -> Result<(), PlatformError>;
}

pub trait FileAssociationService: Send + Sync {
    fn set_associated(&self, extension: &str, associated: bool) -> Result<(), PlatformError>;
}

pub trait NativeTtsService: Send + Sync {
    fn speak(&self, text: &str, language: &str) -> Result<(), PlatformError>;
    fn stop(&self) -> Result<(), PlatformError>;
}

#[derive(Debug, Default)]
pub struct UnsupportedPlatform;

impl GlobalHotkeyService for UnsupportedPlatform {
    fn register(&mut self, _: HotkeyId, _: Hotkey) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("global hotkeys"))
    }

    fn unregister(&mut self, _: HotkeyId) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("global hotkeys"))
    }

    fn disable_all(&mut self) -> Result<(), PlatformError> {
        Ok(())
    }
}

impl TrayService for UnsupportedPlatform {
    fn install(&mut self, _: &str, _: TrayState) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("system tray"))
    }

    fn update(&mut self, _: TrayState) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported("system tray"))
    }

    fn remove(&mut self) -> Result<(), PlatformError> {
        Ok(())
    }
}

#[cfg(windows)]
pub mod windows;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_adapter_reports_capabilities_explicitly() {
        let mut platform = UnsupportedPlatform;
        assert_eq!(
            platform
                .register(
                    HotkeyId(1),
                    Hotkey {
                        modifiers: 2,
                        virtual_key: 65,
                    },
                )
                .unwrap_err(),
            PlatformError::Unsupported("global hotkeys")
        );
        assert_eq!(
            platform
                .install("LexiForge", TrayState::default())
                .unwrap_err(),
            PlatformError::Unsupported("system tray")
        );
        assert!(platform.disable_all().is_ok());
        assert!(platform.remove().is_ok());
    }
}
