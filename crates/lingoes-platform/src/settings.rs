use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::fs::File;

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;
pub const INDEPENDENT_BRAND: &str = "LexiForge";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataMode {
    Portable,
    Installed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataRoot {
    pub mode: DataMode,
    pub path: PathBuf,
}

impl DataRoot {
    pub fn database_path(&self) -> PathBuf {
        self.path.join("lexiforge.db")
    }

    pub fn settings_path(&self) -> PathBuf {
        self.path.join("settings.toml")
    }
}

pub fn resolve_data_root(executable: &Path, installed_base: &Path) -> io::Result<DataRoot> {
    let executable_dir = executable.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "executable path has no parent directory",
        )
    })?;
    if executable_dir.join("portable.toml").is_file() {
        Ok(DataRoot {
            mode: DataMode::Portable,
            path: executable_dir.join("data"),
        })
    } else {
        Ok(DataRoot {
            mode: DataMode::Installed,
            path: installed_base.join(INDEPENDENT_BRAND),
        })
    }
}

pub fn system_installed_data_base() -> io::Result<PathBuf> {
    #[cfg(windows)]
    let variable = "LOCALAPPDATA";
    #[cfg(not(windows))]
    let variable = "XDG_DATA_HOME";

    if let Some(path) = std::env::var_os(variable).filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }

    #[cfg(not(windows))]
    if let Some(home) = std::env::var_os("HOME").filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(home).join(".local").join("share"));
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("{variable} is not defined"),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u32,
    pub shell: ShellSettings,
    pub hotkeys: HotkeySettings,
    pub capture: CaptureSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            shell: ShellSettings::default(),
            hotkeys: HotkeySettings::default(),
            capture: CaptureSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ShellSettings {
    pub start_minimized: bool,
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HotkeySettings {
    pub enabled: bool,
    pub bindings: Vec<HotkeyBinding>,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            enabled: true,
            bindings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyBinding {
    pub action: String,
    pub modifiers: u32,
    pub virtual_key: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CaptureSettings {
    pub clipboard: bool,
    pub cursor: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    #[error("settings I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("settings are invalid: {0}")]
    Invalid(#[from] toml::de::Error),
    #[error("settings serialization failed: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("settings schema {found} is newer than supported schema {supported}")]
    UnsupportedVersion { found: u32, supported: u32 },
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Settings, SettingsError> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                let settings: Settings = toml::from_str(&contents)?;
                if settings.schema_version > SETTINGS_SCHEMA_VERSION {
                    return Err(SettingsError::UnsupportedVersion {
                        found: settings.schema_version,
                        supported: SETTINGS_SCHEMA_VERSION,
                    });
                }
                Ok(settings)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(error) => Err(error.into()),
        }
    }

    pub fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        if settings.schema_version > SETTINGS_SCHEMA_VERSION {
            return Err(SettingsError::UnsupportedVersion {
                found: settings.schema_version,
                supported: SETTINGS_SCHEMA_VERSION,
            });
        }
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "settings path has no parent")
        })?;
        fs::create_dir_all(parent)?;
        let contents = toml::to_string_pretty(settings)?;
        let mut temporary = NamedTempFile::new_in(parent)?;
        temporary.write_all(contents.as_bytes())?;
        temporary.as_file_mut().sync_all()?;
        temporary.persist(&self.path).map_err(|error| error.error)?;
        sync_directory(parent)?;
        Ok(())
    }
}

fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_selects_portable_data_beside_executable() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lexiforge.exe");
        fs::write(&executable, b"").unwrap();
        fs::write(root.path().join("portable.toml"), b"mode = true\n").unwrap();

        let resolved = resolve_data_root(&executable, Path::new("/installed")).unwrap();

        assert_eq!(resolved.mode, DataMode::Portable);
        assert_eq!(resolved.path, root.path().join("data"));
        assert_eq!(
            resolved.database_path(),
            root.path().join("data/lexiforge.db")
        );
    }

    #[test]
    fn absent_marker_selects_independently_branded_installed_root() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lexiforge");
        fs::write(&executable, b"").unwrap();
        let installed = root.path().join("local-data");

        let resolved = resolve_data_root(&executable, &installed).unwrap();

        assert_eq!(resolved.mode, DataMode::Installed);
        assert_eq!(resolved.path, installed.join(INDEPENDENT_BRAND));
    }

    #[test]
    fn missing_settings_have_versioned_defaults() {
        let root = tempfile::tempdir().unwrap();
        let store = SettingsStore::new(root.path().join("settings.toml"));

        let settings = store.load().unwrap();

        assert_eq!(settings.schema_version, SETTINGS_SCHEMA_VERSION);
        assert!(settings.hotkeys.enabled);
        assert!(!settings.shell.start_minimized);
    }

    #[test]
    fn settings_round_trip_through_atomic_store() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("nested/settings.toml");
        let store = SettingsStore::new(&path);
        let mut settings = Settings::default();
        settings.shell.start_minimized = true;
        settings.hotkeys.bindings.push(HotkeyBinding {
            action: "show-main".into(),
            modifiers: 3,
            virtual_key: 76,
        });

        store.save(&settings).unwrap();

        assert_eq!(store.load().unwrap(), settings);
        assert!(path.is_file());
        assert_eq!(
            fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1,
            "atomic temporary file must not remain"
        );
    }

    #[test]
    fn newer_settings_schema_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("settings.toml");
        fs::write(
            &path,
            "schema_version = 99\n[shell]\n[hotkeys]\n[capture]\n",
        )
        .unwrap();

        let error = SettingsStore::new(path).load().unwrap_err();

        assert!(matches!(
            error,
            SettingsError::UnsupportedVersion {
                found: 99,
                supported: SETTINGS_SCHEMA_VERSION
            }
        ));
    }
}
