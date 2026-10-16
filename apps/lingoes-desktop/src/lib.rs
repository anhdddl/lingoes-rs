use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(not(windows))]
use lingoes_capture::ClipboardWatcher;
use lingoes_capture::SystemClipboardReader;
use lingoes_core::{DictionaryEntry, LookupQuery};
use lingoes_dictionary::{DictionaryMeta, HistoryKind, SqliteDictionary};
use lingoes_platform::settings::{
    CaptureModeSetting, DataRoot, Settings, SettingsStore, resolve_data_root,
    system_installed_data_base,
};
use lingoes_platform::web_search::{BrowserLauncher, DefaultBrowserLauncher, WebSearchCatalog};
use lingoes_translator::{
    GoogleTranslator, OpenAiTranslator, TranslationError, TranslationRequest, TranslationResult,
    Translator,
};
use lingoes_tts::{SpeechOptions, SystemTtsEngine, TtsEngine, TtsError, Voice, VoiceSelection};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use url::Url;

use popup::{PopupContent, PopupController, PopupState};

pub mod popup;

slint::include_modules!();

#[derive(Debug, Clone, Default)]
pub struct TranslationOptions<'a> {
    pub engine: &'a str,
    pub base_url: Option<&'a str>,
    pub model: Option<&'a str>,
    pub api_key: Option<&'a str>,
}

const USAGE: &str = "\
Usage:
  lingoes-desktop import <dictionary.db> <source_path> [--name <name>]
  lingoes-desktop lookup <dictionary.db> <query>
  lingoes-desktop full-text <dictionary.db> <query>
  lingoes-desktop list-dictionaries <dictionary.db>
  lingoes-desktop dict info <dictionary.db> <id>
  lingoes-desktop toggle-dictionary <dictionary.db> <id> <true|false>
  lingoes-desktop delete-dictionary <dictionary.db> <id>
  lingoes-desktop translate <text> [--engine google|openai] [--from <code>] [--to <code>]
  lingoes-desktop tts <text> [--lang <code>]
  lingoes-desktop              Launch the graphical application";

const SEARCH_LIMIT: usize = 100;

#[derive(Debug, Clone)]
pub struct AppError(String);

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for AppError {}

pub struct DesktopApp {
    dictionary: SqliteDictionary,
    results: Vec<DictionaryEntry>,
    selected_index: Option<usize>,
    dictionaries: Vec<DictionaryMeta>,
    translator: Arc<dyn Translator>,
    tts: Arc<dyn TtsEngine>,
    status: String,
    has_error: bool,
    popup: PopupController,
    #[cfg(not(windows))]
    clipboard_running: Option<Arc<AtomicBool>>,
    #[cfg(windows)]
    clipboard_listener: Option<lingoes_capture::windows::WindowsClipboardListener>,
    settings: Settings,
    last_live_query: String,
    pub web_search_catalog: WebSearchCatalog,
    pub browser_launcher: Arc<dyn BrowserLauncher>,
}

impl DesktopApp {
    pub fn open(database_path: impl AsRef<Path>) -> Result<Self, AppError> {
        let database_path = database_path.as_ref();
        if let Some(parent) = database_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                AppError(format!(
                    "cannot create database directory {}: {error}",
                    parent.display()
                ))
            })?;
        }
        let dictionary =
            SqliteDictionary::open(database_path).map_err(|error| AppError(error.to_string()))?;
        let translator = Arc::new(GoogleTranslator::new());
        let tts = Arc::new(SystemTtsEngine::new());
        Self::with_services(dictionary, translator, tts)
    }

    pub fn with_services(
        dictionary: SqliteDictionary,
        translator: Arc<dyn Translator>,
        tts: Arc<dyn TtsEngine>,
    ) -> Result<Self, AppError> {
        Self::with_all_services(
            dictionary,
            translator,
            tts,
            Arc::new(DefaultBrowserLauncher),
        )
    }

    pub fn with_all_services(
        dictionary: SqliteDictionary,
        translator: Arc<dyn Translator>,
        tts: Arc<dyn TtsEngine>,
        browser_launcher: Arc<dyn BrowserLauncher>,
    ) -> Result<Self, AppError> {
        let mut app = Self {
            dictionary,
            results: Vec::new(),
            selected_index: None,
            dictionaries: Vec::new(),
            translator,
            tts,
            status: "Sẵn sàng".into(),
            has_error: false,
            popup: PopupController::new(),
            #[cfg(not(windows))]
            clipboard_running: None,
            #[cfg(windows)]
            clipboard_listener: None,
            settings: Settings::default(),
            last_live_query: String::new(),
            web_search_catalog: WebSearchCatalog::default(),
            browser_launcher,
        };
        let _ = app.list_dictionaries();
        Ok(app)
    }

    pub fn web_search(&mut self, query: &str, engine_id: Option<&str>) -> Result<Url, AppError> {
        let url = self
            .web_search_catalog
            .build_url(engine_id, query)
            .map_err(|e| {
                self.status = format!("Lỗi tìm kiếm web: {e}");
                self.has_error = true;
                AppError(e.to_string())
            })?;
        self.browser_launcher.open_url(&url).map_err(|e| {
            self.status = format!("Không thể mở trình duyệt: {e}");
            self.has_error = true;
            AppError(e.to_string())
        })?;
        let engine_name = self
            .web_search_catalog
            .get_engine(engine_id.unwrap_or(self.web_search_catalog.default_engine_id()))
            .map(|e| e.name.as_str())
            .unwrap_or("Web");
        self.status = format!("Mở tìm kiếm web qua {engine_name}...");
        self.has_error = false;
        Ok(url)
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    pub fn set_settings(&mut self, settings: Settings) {
        self.popup.set_preferred_pinned(settings.popup.pinned);
        self.settings = settings;
    }

    pub fn language(&self) -> &str {
        &self.settings.shell.language
    }

    pub fn set_language(&mut self, language: impl Into<String>) {
        let lang = language.into();
        self.settings.shell.language = lang;
        if self.results.is_empty() && self.selected_index.is_none() {
            self.status = if self.language() == "en" {
                "Ready".into()
            } else {
                "Sẵn sàng".into()
            };
        }
    }

    pub fn search(&mut self, query: &str) {
        let trimmed = query.trim();
        self.last_live_query = trimmed.to_string();
        let _ = self
            .dictionary
            .add_history_entry(HistoryKind::Search, trimmed, None, None, 100);
        self.search_with_mode(trimmed, true);
    }

    /// Updates prefix results while typing and skips duplicate normalized input.
    /// Returns true only when a database lookup or clear operation was performed.
    pub fn live_search(&mut self, query: &str) -> bool {
        let normalized = query.trim();
        if normalized == self.last_live_query {
            return false;
        }
        self.last_live_query = normalized.to_string();
        self.search_with_mode(normalized, false);
        true
    }

    pub fn search_full_text(&mut self, query: &str) {
        self.search_with_mode(query, true);
    }

    pub fn go_home(&mut self) {
        self.results.clear();
        self.selected_index = None;
        self.last_live_query.clear();
        self.status = if self.language() == "en" {
            "Enter a word or phrase to look up".into()
        } else {
            "Nhập từ khóa để tra cứu".into()
        };
        self.has_error = false;
    }

    fn search_with_mode(&mut self, query: &str, advanced: bool) {
        let Ok(lookup_query) = LookupQuery::new(query) else {
            self.results.clear();
            self.selected_index = None;
            self.status = if self.language() == "en" {
                "Enter a word or phrase to look up".into()
            } else {
                "Nhập từ khóa để tra cứu".into()
            };
            self.has_error = false;
            return;
        };
        let result = if advanced {
            self.dictionary.lookup_advanced(&lookup_query, SEARCH_LIMIT)
        } else {
            self.dictionary.lookup_smart(&lookup_query, SEARCH_LIMIT)
        };
        match result {
            Ok(results) => {
                self.results = results;
                self.selected_index = (!self.results.is_empty()).then_some(0);
                self.status = if self.language() == "en" {
                    match self.results.len() {
                        0 => "No matching words found".into(),
                        1 => "Found 1 result".into(),
                        count if advanced => format!("Found {count} advanced results"),
                        count => format!("Found {count} results"),
                    }
                } else {
                    match self.results.len() {
                        0 => "Không tìm thấy từ phù hợp".into(),
                        1 => "Tìm thấy 1 kết quả".into(),
                        count if advanced => format!("Tìm thấy {count} kết quả nâng cao"),
                        count => format!("Tìm thấy {count} kết quả"),
                    }
                };
                self.has_error = false;
            }
            Err(error) => self.set_error(error),
        }
    }

    pub fn history_items(&self) -> Vec<String> {
        self.dictionary
            .list_history(HistoryKind::Search, Some(50))
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.query)
            .collect()
    }

    pub fn clear_history(&mut self) -> Result<(), AppError> {
        self.dictionary
            .clear_history(Some(HistoryKind::Search))
            .map_err(|error| AppError(error.to_string()))?;
        self.status = if self.language() == "en" {
            "Search history cleared".into()
        } else {
            "Đã xóa lịch sử tra cứu".into()
        };
        Ok(())
    }

    pub fn notebook_items(&self) -> Vec<String> {
        self.dictionary
            .list_favorites(None)
            .unwrap_or_default()
            .into_iter()
            .map(|f| f.headword)
            .collect()
    }

    pub fn is_notebook_word(&self, headword: &str) -> bool {
        self.dictionary.is_favorite(headword).unwrap_or(false)
    }

    pub fn toggle_notebook(&mut self, headword: &str, definition: &str) -> Result<bool, AppError> {
        let is_fav = self.dictionary.is_favorite(headword).unwrap_or(false);
        if is_fav {
            self.dictionary
                .remove_favorite_by_headword(headword)
                .map_err(|e| AppError(e.to_string()))?;
            self.status = if self.language() == "en" {
                format!("Removed '{headword}' from notebook")
            } else {
                format!("Đã xóa '{headword}' khỏi sổ tay")
            };
            self.has_error = false;
            Ok(false)
        } else {
            self.dictionary
                .add_favorite(headword, None, definition, Some("Default"), &[], None)
                .map_err(|e| AppError(e.to_string()))?;
            self.status = if self.language() == "en" {
                format!("Added '{headword}' to notebook")
            } else {
                format!("Đã thêm '{headword}' vào sổ tay")
            };
            self.has_error = false;
            Ok(true)
        }
    }

    pub fn remove_from_notebook(&mut self, headword: &str) -> Result<(), AppError> {
        self.dictionary
            .remove_favorite_by_headword(headword)
            .map_err(|e| AppError(e.to_string()))?;
        self.status = if self.language() == "en" {
            format!("Removed '{headword}' from notebook")
        } else {
            format!("Đã xóa '{headword}' khỏi sổ tay")
        };
        self.has_error = false;
        Ok(())
    }

    pub fn export_notebook_html(&mut self) -> Result<String, AppError> {
        let favorites = self
            .dictionary
            .list_favorites(None)
            .map_err(|e| AppError(e.to_string()))?;
        if favorites.is_empty() {
            self.status = if self.language() == "en" {
                "Word notebook is empty".into()
            } else {
                "Sổ tay từ vựng đang trống".into()
            };
            self.has_error = false;
            return Ok(String::new());
        }

        let mut html = String::from(
            "<!DOCTYPE html><html><head><meta charset='utf-8'><title>Lingoes RS - Sổ tay từ vựng</title><style>body{font-family:sans-serif;margin:30px;background:#f8fafc;}h1{color:#1e3a8a;border-bottom:2px solid #3b82f6;padding-bottom:10px;}.word-card{background:white;border:1px solid #cbd5e1;border-radius:6px;padding:15px;margin-bottom:12px;box-shadow:0 1px 3px rgba(0,0,0,0.05);}.headword{font-size:18px;font-weight:bold;color:#0f172a;}.def{margin-top:8px;color:#334155;line-height:1.5;white-space:pre-wrap;}</style></head><body>",
        );
        html.push_str("<h1>⭐ Sổ tay từ vựng - Lingoes RS</h1>");
        html.push_str(&format!(
            "<p>Tổng số: <b>{}</b> từ vựng đã lưu</p>",
            favorites.len()
        ));
        for fav in favorites {
            html.push_str("<div class='word-card'>");
            html.push_str(&format!(
                "<div class='headword'>{}</div>",
                lingoes_core::escape_html(&fav.headword)
            ));
            html.push_str(&format!(
                "<div class='def'>{}</div>",
                lingoes_core::escape_html(&fav.snapshot_definition)
            ));
            html.push_str("</div>");
        }
        html.push_str("</body></html>");

        let path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("lingoes_vocabulary_notebook.html");
        std::fs::write(&path, html).map_err(|e| AppError(e.to_string()))?;
        self.status = if self.language() == "en" {
            format!("Exported notebook to {}", path.display())
        } else {
            format!("Đã xuất sổ tay ra {}", path.display())
        };
        self.has_error = false;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn select(&mut self, index: usize) {
        if index < self.results.len() {
            self.selected_index = Some(index);
        }
    }

    pub fn results(&self) -> &[DictionaryEntry] {
        &self.results
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub fn selected_headword(&self) -> &str {
        self.selected_entry().map_or("", DictionaryEntry::headword)
    }

    pub fn selected_definition(&self) -> &str {
        self.selected_entry()
            .map_or("", DictionaryEntry::definition)
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn has_error(&self) -> bool {
        self.has_error
    }

    fn selected_entry(&self) -> Option<&DictionaryEntry> {
        self.selected_index
            .and_then(|index| self.results.get(index))
    }

    pub fn set_error(&mut self, error: impl Display) {
        self.results.clear();
        self.selected_index = None;
        self.status = error.to_string();
        self.has_error = true;
    }

    // Dictionary Management
    pub fn list_dictionaries(&mut self) -> Result<&[DictionaryMeta], AppError> {
        self.dictionaries = self
            .dictionary
            .list_dictionaries()
            .map_err(|e| AppError(e.to_string()))?;
        Ok(&self.dictionaries)
    }

    pub fn toggle_dictionary(&mut self, id: i64, enabled: bool) -> Result<(), AppError> {
        self.dictionary
            .toggle_dictionary(id, enabled)
            .map_err(|e| AppError(e.to_string()))?;
        self.list_dictionaries()?;
        self.status = format!(
            "Từ điển ID {id} đã {}",
            if enabled { "kích hoạt" } else { "tắt" }
        );
        self.has_error = false;
        Ok(())
    }

    pub fn delete_dictionary(&mut self, id: i64) -> Result<(), AppError> {
        self.dictionary
            .delete_dictionary(id)
            .map_err(|e| AppError(e.to_string()))?;
        self.list_dictionaries()?;
        self.status = format!("Đã xóa từ điển ID {id}");
        self.has_error = false;
        Ok(())
    }

    pub fn import_path_with_name(
        &mut self,
        source_path: impl AsRef<Path>,
        name: Option<&str>,
    ) -> Result<(), AppError> {
        let source_path = source_path.as_ref();
        let result: Result<(), AppError> = (|| {
            let report = self
                .dictionary
                .import_path_with_name(source_path, name)
                .map_err(|error| AppError(error.to_string()))?;
            let entry_text = if report.imported == 1 {
                "entry"
            } else {
                "entries"
            };
            self.status = format!(
                "Imported {} {} ({} new, {} updated)",
                report.imported, entry_text, report.inserted, report.updated
            );
            self.has_error = false;
            Ok(())
        })();
        if let Err(error) = &result {
            self.status = error.to_string();
            self.has_error = true;
        }
        let _ = self.list_dictionaries();
        result
    }

    pub fn import_path(&mut self, source_path: impl AsRef<Path>) -> Result<(), AppError> {
        self.import_path_with_name(source_path, None)
    }

    pub fn dictionaries(&self) -> &[DictionaryMeta] {
        &self.dictionaries
    }

    // Translation
    pub fn translate(&mut self, text: &str, from: &str, to: &str) -> Result<String, AppError> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AppError(format!("runtime error: {e}")))?;
        let translator = Arc::clone(&self.translator);
        let req = TranslationRequest::new(text, from, to);
        let result = rt.block_on(translator.translate(req));
        match result {
            Ok(translated) => {
                self.status = format!("Dịch thành công qua {}", translated.provider);
                self.has_error = false;
                Ok(translated.text)
            }
            Err(err) => {
                self.set_error(&err);
                Err(AppError(err.to_string()))
            }
        }
    }

    pub fn translate_with_engine(
        &mut self,
        text: &str,
        from: &str,
        to: &str,
        opts: TranslationOptions<'_>,
    ) -> Result<TranslationResult, AppError> {
        let translator: Arc<dyn Translator> = match opts.engine.to_lowercase().trim() {
            "openai" | "llm" | "local" => {
                let mut builder = OpenAiTranslator::builder();
                if let Some(url) = opts.base_url {
                    if !url.trim().is_empty() {
                        builder = builder.base_url(url);
                    }
                }
                if let Some(m) = opts.model {
                    if !m.trim().is_empty() {
                        builder = builder.model(m);
                    }
                }
                if let Some(k) = opts.api_key {
                    if !k.trim().is_empty() {
                        builder = builder.api_key(k);
                    }
                }
                Arc::new(builder.build())
            }
            "google" => Arc::new(GoogleTranslator::new()),
            _ => Arc::clone(&self.translator),
        };

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AppError(format!("runtime error: {e}")))?;
        let req = TranslationRequest::new(text, from, to);
        let result = rt.block_on(translator.translate(req));
        match result {
            Ok(translated) => {
                self.status = format!("Dịch thành công qua {}", translated.provider);
                self.has_error = false;
                Ok(translated)
            }
            Err(err) => {
                self.set_error(&err);
                Err(AppError(err.to_string()))
            }
        }
    }

    // TTS
    pub fn speak(&mut self, text: &str, lang: &str) -> Result<(), AppError> {
        let options = SpeechOptions::default();
        match self
            .tts
            .speak_async(text, lang, VoiceSelection::Automatic, options)
        {
            Ok(_) => {
                self.status = format!("Đang phát âm: {text}");
                self.has_error = false;
                Ok(())
            }
            Err(TtsError::Unsupported(_)) => {
                let tts = Arc::clone(&self.tts);
                let text_owned = text.to_owned();
                let lang_owned = lang.to_owned();
                std::thread::spawn(move || {
                    let _ = tts.speak_blocking(
                        &text_owned,
                        &lang_owned,
                        VoiceSelection::Automatic,
                        options,
                    );
                });
                self.status = format!("Đang phát âm: {text}");
                self.has_error = false;
                Ok(())
            }
            Err(err) => {
                self.set_error(&err);
                Err(AppError(err.to_string()))
            }
        }
    }

    pub fn stop_tts(&mut self) -> Result<(), AppError> {
        match self.tts.stop() {
            Ok(()) => {
                self.status = if self.language() == "en" {
                    "Speech stopped".into()
                } else {
                    "Đã dừng phát âm".into()
                };
                self.has_error = false;
                Ok(())
            }
            Err(TtsError::Unsupported(_)) => {
                self.status = if self.language() == "en" {
                    "Stop speech is not supported on this platform".into()
                } else {
                    "Dừng phát âm không khả dụng trên hệ thống này".into()
                };
                self.has_error = false;
                Ok(())
            }
            Err(err) => {
                self.set_error(&err);
                Err(AppError(err.to_string()))
            }
        }
    }

    pub fn tts_voices(&self) -> Result<Vec<Voice>, AppError> {
        self.tts.voices().map_err(|e| AppError(e.to_string()))
    }

    // Clipboard & Quick Lookup
    pub fn copy_to_clipboard(&mut self, text: &str) -> Result<(), AppError> {
        let mut board = arboard::Clipboard::new()
            .map_err(|e| AppError(format!("Không thể truy cập clipboard: {e}")))?;
        board
            .set_text(text)
            .map_err(|e| AppError(format!("Lỗi sao chép vào clipboard: {e}")))?;
        self.status = if self.language() == "en" {
            "Copied to clipboard".into()
        } else {
            "Đã sao chép vào bộ nhớ tạm".into()
        };
        self.has_error = false;
        Ok(())
    }

    pub fn handle_captured_text(&mut self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let token = self.popup.begin_capture();
        self.search(trimmed);
        let content = self.selected_entry().map(|entry| {
            PopupContent::new(entry.headword().to_string(), entry.definition().to_string())
        });
        let completed = self.popup.complete_capture(token, content);
        if completed && self.settings.popup.auto_pronounce {
            let word = self.quick_popup_word().to_owned();
            if !word.is_empty() {
                let _ = self.speak(&word, "en");
            }
        }
    }

    pub fn show_quick_popup(&self) -> bool {
        matches!(
            self.popup.state(),
            PopupState::VisibleTransient | PopupState::Pinned | PopupState::Expanded
        )
    }

    pub fn quick_popup_word(&self) -> &str {
        self.popup
            .content()
            .map_or("", |content| content.headword())
    }

    pub fn quick_popup_def(&self) -> &str {
        self.popup
            .content()
            .map_or("", |content| content.definition())
    }

    pub fn dismiss_quick_popup(&mut self) {
        self.popup.close();
    }

    pub fn pin_quick_popup(&mut self, pinned: bool) {
        if pinned {
            self.popup.pin();
        } else {
            self.popup.unpin();
        }
        self.settings.popup.pinned = pinned;
    }

    pub fn expand_quick_popup(&mut self, expanded: bool) {
        if expanded {
            self.popup.expand();
        } else {
            self.popup.collapse();
        }
    }

    pub fn quick_popup_state(&self) -> PopupState {
        self.popup.state()
    }

    pub fn quick_popup_is_pinned(&self) -> bool {
        self.popup.is_pinned()
    }

    pub fn is_clipboard_monitoring(&self) -> bool {
        #[cfg(not(windows))]
        {
            self.clipboard_running
                .as_ref()
                .is_some_and(|flag| flag.load(Ordering::Relaxed))
        }
        #[cfg(windows)]
        {
            self.clipboard_listener.is_some()
        }
    }

    pub fn stop_clipboard_monitor(&mut self) {
        #[cfg(not(windows))]
        if let Some(flag) = self.clipboard_running.take() {
            flag.store(false, Ordering::Relaxed);
        }
        #[cfg(windows)]
        if let Some(listener) = self.clipboard_listener.take() {
            listener.stop();
        }
        self.settings.capture.enabled = false;
        self.settings.capture.clipboard = false;
    }
}

pub fn portable_database_path(executable: &Path) -> Result<PathBuf, AppError> {
    let directory = executable
        .parent()
        .ok_or_else(|| AppError("executable path has no parent directory".into()))?;
    Ok(directory.join("data").join("lingoes.db"))
}

pub fn resolve_app_data_root(executable: &Path) -> Result<DataRoot, AppError> {
    let installed_base = system_installed_data_base()
        .unwrap_or_else(|_| executable.parent().unwrap_or(Path::new(".")).join("data"));
    resolve_data_root(executable, &installed_base)
        .map_err(|error| AppError(format!("cannot resolve data root: {error}")))
}

pub fn resolve_database_path(data_root: &DataRoot) -> PathBuf {
    let branded = data_root.database_path();
    if branded.exists() {
        return branded;
    }
    let legacy = data_root.path.join("lingoes.db");
    if legacy.exists() {
        return legacy;
    }
    branded
}

pub fn launch_gui() -> Result<(), AppError> {
    launch_gui_with_options(None, None)
}

pub fn launch_gui_with_options(
    initial_query: Option<&str>,
    show_popup_word: Option<&str>,
) -> Result<(), AppError> {
    log_startup("launch_gui_with_options started");
    let executable = std::env::current_exe()
        .map_err(|error| AppError(format!("cannot locate executable: {error}")))?;
    log_startup(&format!("executable: {}", executable.display()));

    let data_root = resolve_app_data_root(&executable)?;
    log_startup(&format!(
        "data root: {:?} at {}",
        data_root.mode,
        data_root.path.display()
    ));

    let settings_store = SettingsStore::new(data_root.settings_path());
    let settings = settings_store
        .load()
        .map_err(|error| AppError(format!("cannot load settings: {error}")))?;

    let database_path = resolve_database_path(&data_root);
    log_startup(&format!("database path: {}", database_path.display()));
    if !database_path.exists() {
        let err_msg = format!(
            "Không tìm thấy tệp cơ sở dữ liệu từ điển 'lexiforge.db' tại:\n{}\n\nNguyên nhân phổ biến:\n1. Bạn đang mở trực tiếp file .exe bên trong file ZIP mà chưa GIẢI NÉN (Extract All).\n   -> Hướng dẫn: Nhấp chuột phải vào file ZIP, chọn 'Extract All...' (Giải nén tất cả), sau đó mở thư mục đã giải nén rồi mới chạy lingoes-desktop.exe.\n2. Thư mục 'data' hoặc tệp 'lexiforge.db' bị thiếu hoặc bị di chuyển.",
            database_path.display()
        );
        show_fatal_error("Lingoes RS - Thiếu tệp dữ liệu", &err_msg);
        return Err(AppError(err_msg));
    }

    log_startup("opening dictionary database...");
    let mut desktop_app = DesktopApp::open(&database_path)?;
    desktop_app.set_settings(settings);
    let application = Arc::new(Mutex::new(desktop_app));

    log_startup("initializing main window...");
    let window = match AppWindow::new() {
        Ok(w) => {
            log_startup("AppWindow initialized successfully");
            w
        }
        Err(err) => {
            log_startup(&format!(
                "Hardware renderer failed ({err}), falling back to software renderer..."
            ));
            unsafe {
                std::env::set_var("SLINT_BACKEND", "winit-software");
            }
            AppWindow::new().map_err(|err2| {
                AppError(format!(
                    "Không thể khởi tạo giao diện đồ họa (Hardware: {err}, Software fallback: {err2})"
                ))
            })?
        }
    };

    log_startup("initializing popup window...");
    let popup_window = match PopupWindow::new() {
        Ok(p) => p,
        Err(err) => {
            log_startup(&format!(
                "PopupWindow hardware init failed ({err}), falling back to software..."
            ));
            unsafe {
                std::env::set_var("SLINT_BACKEND", "winit-software");
            }
            PopupWindow::new()
                .map_err(|err2| AppError(format!("Không thể khởi tạo cửa sổ popup: {err2}")))?
        }
    };
    #[cfg(windows)]
    let _popup_native = match materialize_windows_popup(&popup_window) {
        Ok(native) => Some(native),
        Err(error) => {
            eprintln!("Warning: could not materialize Windows popup adapter: {error}");
            None
        }
    };

    // Initialize Appendices data in window
    {
        let verbs: Vec<IrregularVerbItem> = lingoes_core::list_irregular_verbs()
            .iter()
            .map(|v| IrregularVerbItem {
                v1: SharedString::from(v.v1),
                v2: SharedString::from(v.v2),
                v3: SharedString::from(v.v3),
                meaning: SharedString::from(v.meaning_vi),
            })
            .collect();
        window.set_irregular_verbs(ModelRc::new(VecModel::from(verbs)));

        let is_en = application.lock().unwrap().language() == "en";
        let dialing: Vec<DialingCodeItem> = lingoes_core::DIALING_CODES
            .iter()
            .map(|d| DialingCodeItem {
                country: SharedString::from(if is_en { d.country_en } else { d.country_vi }),
                code: SharedString::from(d.code),
                flag: SharedString::from(d.iso),
            })
            .collect();
        window.set_dialing_codes(ModelRc::new(VecModel::from(dialing)));

        let ipa: Vec<IpaItem> = lingoes_core::IPA_SYMBOLS
            .iter()
            .map(|s| IpaItem {
                symbol: SharedString::from(s.symbol),
                category: SharedString::from(s.category),
                sample_word: SharedString::from(s.sample_word),
                sample_ipa: SharedString::from(s.sample_ipa),
                description: SharedString::from(s.description_vi),
            })
            .collect();
        window.set_ipa_symbols(ModelRc::new(VecModel::from(ipa)));

        let initial_units = compute_unit_rows("length", "m", "100");
        window.set_unit_rows(ModelRc::new(VecModel::from(initial_units)));
    }

    // Word Notebook callbacks
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_toggle_notebook(move |headword, definition| {
            let mut app = application.lock().unwrap();
            let _ = app.toggle_notebook(headword.as_str(), definition.as_str());
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &app);
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_remove_notebook_item(move |headword| {
            let mut app = application.lock().unwrap();
            let _ = app.remove_from_notebook(headword.as_str());
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &app);
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_export_notebook(move || {
            let mut app = application.lock().unwrap();
            let _ = app.export_notebook_html();
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &app);
            }
        });
    }

    // Appendices & Tools callbacks
    {
        let window_weak = window.as_weak();
        window.on_search_irregular_verbs(move |query| {
            let filtered: Vec<IrregularVerbItem> =
                lingoes_core::search_irregular_verbs(query.as_str())
                    .into_iter()
                    .map(|v| IrregularVerbItem {
                        v1: SharedString::from(v.v1),
                        v2: SharedString::from(v.v2),
                        v3: SharedString::from(v.v3),
                        meaning: SharedString::from(v.meaning_vi),
                    })
                    .collect();
            if let Some(window) = window_weak.upgrade() {
                window.set_irregular_verbs(ModelRc::new(VecModel::from(filtered)));
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_search_dialing_codes(move |query| {
            let app = application.lock().unwrap();
            let is_en = app.language() == "en";
            let filtered: Vec<DialingCodeItem> = lingoes_core::search_dialing_codes(query.as_str())
                .into_iter()
                .map(|d| DialingCodeItem {
                    country: SharedString::from(if is_en { d.country_en } else { d.country_vi }),
                    code: SharedString::from(d.code),
                    flag: SharedString::from(d.iso),
                })
                .collect();
            if let Some(window) = window_weak.upgrade() {
                window.set_dialing_codes(ModelRc::new(VecModel::from(filtered)));
            }
        });
    }
    {
        let window_weak = window.as_weak();
        window.on_update_unit_conversions(move |category, from_unit, value_str| {
            let rows = compute_unit_rows(category.as_str(), from_unit.as_str(), value_str.as_str());
            if let Some(window) = window_weak.upgrade() {
                window.set_unit_rows(ModelRc::new(VecModel::from(rows)));
            }
        });
    }

    // Search callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_search(move |query| {
            application.lock().unwrap().search(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Prefix lookup on every edit. The indexed query is fast enough to stay
    // synchronous while avoiding timer lag and stale-result races.
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_live_search(move |query| {
            let changed = application.lock().unwrap().live_search(query.as_str());
            if changed {
                if let Some(window) = window_weak.upgrade() {
                    update_window(&window, &application.lock().unwrap());
                }
            }
        });
    }

    // Full-text search callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_full_text_search(move |query| {
            application.lock().unwrap().search_full_text(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Select result callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_select_result(move |index| {
            if index >= 0 {
                application.lock().unwrap().select(index as usize);
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Navigate within the current ordered result set.
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_navigate_back(move || {
            let mut app = application.lock().unwrap();
            if let Some(index) = app.selected_index().filter(|index| *index > 0) {
                app.select(index - 1);
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &app);
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_navigate_forward(move || {
            let mut app = application.lock().unwrap();
            if let Some(index) = app.selected_index() {
                if index + 1 < app.results().len() {
                    app.select(index + 1);
                }
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &app);
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_go_home(move || {
            let mut app = application.lock().unwrap();
            app.go_home();
            if let Some(window) = window_weak.upgrade() {
                window.set_active_tab(0);
                window.set_query("".into());
                window.set_selected_headword("".into());
                window.set_selected_definition("".into());
                window.set_selected_index(-1);
                update_window(&window, &app);
            }
        });
    }

    // Launch a validated web-search URL through the platform adapter.
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_web_search(move |query, engine| {
            let engine = (!engine.trim().is_empty()).then_some(engine.as_str());
            let _ = application
                .lock()
                .unwrap()
                .web_search(query.as_str(), engine);
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Import dictionary callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_import_dictionary(move |path, name| {
            let path = path.trim();
            if path.is_empty() {
                application
                    .lock()
                    .unwrap()
                    .set_error("Chọn hoặc nhập đường dẫn file TSV, StarDict, hoặc MDict");
            } else {
                let name_opt = if name.trim().is_empty() {
                    None
                } else {
                    Some(name.as_str())
                };
                let _ = application
                    .lock()
                    .unwrap()
                    .import_path_with_name(path, name_opt);
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Toggle dictionary callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_toggle_dictionary(move |id, enabled| {
            let _ = application
                .lock()
                .unwrap()
                .toggle_dictionary(id as i64, enabled);
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Delete dictionary callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_delete_dictionary(move |id| {
            let _ = application.lock().unwrap().delete_dictionary(id as i64);
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Speak callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_speak(move |text, lang| {
            let _ = application
                .lock()
                .unwrap()
                .speak(text.as_str(), lang.as_str());
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Stop TTS callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_stop_tts(move || {
            let _ = application.lock().unwrap().stop_tts();
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Toggle auto-pronounce callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_toggle_auto_pronounce(move |enabled| {
            application
                .lock()
                .unwrap()
                .settings_mut()
                .popup
                .auto_pronounce = enabled;
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Copy to clipboard callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_copy_to_clipboard(move |text| {
            let _ = application.lock().unwrap().copy_to_clipboard(text.as_str());
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Export selected rich result as standalone sanitized HTML (IO-02).
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_export_result_html(move || {
            let mut app = application.lock().unwrap();
            if let Some(entry) = app.selected_entry() {
                let attribution = entry.dictionary_name().unwrap_or("Dictionary");
                let document = lingoes_core::RichDocument::from_html(
                    entry.headword(),
                    entry.definition(),
                    Some(attribution),
                );
                let safe_name: String = entry
                    .headword()
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .collect();
                let path = std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(format!("{safe_name}.html"));
                match fs::write(&path, document.save_as_html()) {
                    Ok(()) => {
                        app.status = format!("Đã xuất HTML: {}", path.display());
                        app.has_error = false;
                    }
                    Err(error) => {
                        app.status = format!("Không thể xuất HTML: {error}");
                        app.has_error = true;
                    }
                }
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &app);
            }
        });
    }

    // Translate & Cancel Translate callbacks (completely asynchronous off UI thread)
    let active_translation_token: Arc<Mutex<Option<Arc<AtomicBool>>>> = Arc::new(Mutex::new(None));
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        let active_token = Arc::clone(&active_translation_token);
        window.on_translate(move |text, from, to, engine| {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return;
            }

            // Cancel any in-flight translation
            if let Some(prev) = active_token.lock().unwrap().take() {
                prev.store(true, Ordering::SeqCst);
            }

            let new_cancel = Arc::new(AtomicBool::new(false));
            *active_token.lock().unwrap() = Some(Arc::clone(&new_cancel));

            if let Some(w) = window_weak.upgrade() {
                w.set_is_translating(true);
                w.set_translate_target_text("Đang dịch...".into());
                let base_url = w.get_openai_base_url().to_string();
                let model = w.get_openai_model().to_string();
                let api_key = w.get_openai_api_key().to_string();
                let fallback_target = w.get_default_target_lang().to_string();

                let app_lock = application.lock().unwrap();
                let translator: Arc<dyn Translator> = match engine.to_lowercase().trim() {
                    "openai" | "llm" | "local" => {
                        let mut builder = OpenAiTranslator::builder();
                        if !base_url.trim().is_empty() {
                            builder = builder.base_url(base_url);
                        }
                        if !model.trim().is_empty() {
                            builder = builder.model(model);
                        }
                        if !api_key.trim().is_empty() {
                            builder = builder.api_key(api_key);
                        }
                        Arc::new(builder.build())
                    }
                    "google" => Arc::new(GoogleTranslator::new()),
                    _ => Arc::clone(&app_lock.translator),
                };
                drop(app_lock);

                let cancel_for_worker = Arc::clone(&new_cancel);
                let app_for_worker = Arc::clone(&application);
                let win_for_worker = window_weak.clone();
                let text_owned = text.to_string();
                let from_owned = from.to_string();
                let to_owned = to.to_string();

                std::thread::spawn(move || {
                    let mut req = TranslationRequest::new(text_owned, from_owned, to_owned);
                    if !fallback_target.trim().is_empty() {
                        req = req.with_fallback(fallback_target);
                    }
                    req = req.with_cancellation(Arc::clone(&cancel_for_worker));

                    let rt = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(rt) => rt,
                        Err(e) => {
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w) = win_for_worker.upgrade() {
                                    w.set_is_translating(false);
                                    w.set_translate_target_text(format!("Lỗi runtime: {e}").into());
                                }
                            });
                            return;
                        }
                    };

                    let res = rt.block_on(translator.translate(req));

                    let _ = slint::invoke_from_event_loop(move || {
                        if cancel_for_worker.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Some(w) = win_for_worker.upgrade() {
                            w.set_is_translating(false);
                            let mut app = app_for_worker.lock().unwrap();
                            match res {
                                Ok(trans_res) => {
                                    w.set_translate_target_text(trans_res.text.into());
                                    app.status =
                                        format!("Dịch thành công qua {}", trans_res.provider);
                                    app.has_error = false;
                                }
                                Err(TranslationError::Cancelled) => {
                                    w.set_translate_target_text("Đã hủy dịch.".into());
                                    app.status = "Đã hủy phiên dịch".into();
                                    app.has_error = false;
                                }
                                Err(err) => {
                                    w.set_translate_target_text(format!("Lỗi: {err}").into());
                                    app.status = format!("Lỗi dịch: {err}");
                                    app.has_error = true;
                                }
                            }
                            update_window(&w, &app);
                        }
                    });
                });
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        let active_token = Arc::clone(&active_translation_token);
        window.on_cancel_translation(move || {
            if let Some(token) = active_token.lock().unwrap().take() {
                token.store(true, Ordering::SeqCst);
            }
            if let Some(w) = window_weak.upgrade() {
                w.set_is_translating(false);
                w.set_translate_target_text("Đã hủy dịch.".into());
                let mut app = application.lock().unwrap();
                app.status = "Đã hủy phiên dịch".into();
                app.has_error = false;
                update_window(&w, &app);
            }
        });
    }

    // Standalone quick-lookup popup callbacks.
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        popup_window.window().on_close_requested(move || {
            application.lock().unwrap().dismiss_quick_popup();
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        popup_window.on_close(move || {
            application.lock().unwrap().dismiss_quick_popup();
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        popup_window.on_pin(move |pinned| {
            application.lock().unwrap().pin_quick_popup(pinned);
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        popup_window.on_expand(move |expanded| {
            application.lock().unwrap().expand_quick_popup(expanded);
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        let window_weak = window.as_weak();
        popup_window.on_previous(move || {
            let mut app = application.lock().unwrap();
            let index = app.selected_index().unwrap_or(0).saturating_sub(1);
            app.select(index);
            let token = app.popup.begin_capture();
            let content = app.selected_entry().map(|entry| {
                PopupContent::new(entry.headword().to_owned(), entry.definition().to_owned())
            });
            let _ = app.popup.complete_capture(token, content);
            if let Some(main) = window_weak.upgrade() {
                update_window(&main, &app);
            }
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &app);
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        let window_weak = window.as_weak();
        popup_window.on_next(move || {
            let mut app = application.lock().unwrap();
            let index = app
                .selected_index()
                .map_or(0, |index| index.saturating_add(1));
            if index < app.results().len() {
                app.select(index);
                let token = app.popup.begin_capture();
                let content = app.selected_entry().map(|entry| {
                    PopupContent::new(entry.headword().to_owned(), entry.definition().to_owned())
                });
                let _ = app.popup.complete_capture(token, content);
            }
            if let Some(main) = window_weak.upgrade() {
                update_window(&main, &app);
            }
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &app);
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        popup_window.on_copy(move || {
            let text = application.lock().unwrap().quick_popup_def().to_owned();
            let _ = application.lock().unwrap().copy_to_clipboard(&text);
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        popup_window.on_speak(move || {
            let word = application.lock().unwrap().quick_popup_word().to_owned();
            let _ = application.lock().unwrap().speak(&word, "en");
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        popup_window.on_stop(move || {
            let _ = application.lock().unwrap().stop_tts();
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
        });
    }
    {
        let application = Arc::clone(&application);
        let popup_weak = popup_window.as_weak();
        let window_weak = window.as_weak();
        popup_window.on_open_main(move || {
            let query = application.lock().unwrap().quick_popup_word().to_owned();
            application.lock().unwrap().dismiss_quick_popup();
            if let Some(main) = window_weak.upgrade() {
                main.set_active_tab(0);
                main.set_query(query.clone().into());
                main.invoke_search(query.into());
                let _ = main.show();
            }
            if let Some(popup) = popup_weak.upgrade() {
                update_popup_window(&popup, &application.lock().unwrap());
            }
        });
    }

    // Capture mode settings feed all captured text through handle_captured_text.
    {
        let application = Arc::clone(&application);
        window.on_configure_capture(move |cursor, selection, ocr| {
            let mut app = application.lock().unwrap();
            app.settings.capture.cursor = cursor;
            app.settings.capture.ocr_enabled = ocr;
            app.settings.capture.mode = if selection {
                CaptureModeSetting::SelectedText
            } else if cursor {
                CaptureModeSetting::CursorWord
            } else {
                CaptureModeSetting::Clipboard
            };
            app.settings.capture.enabled = cursor || selection || app.settings.capture.clipboard;
        });
    }

    // Toggle clipboard monitor callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        let popup_weak = popup_window.as_weak();
        window.on_toggle_clipboard_monitor(move |enabled| {
            if enabled {
                #[cfg(not(windows))]
                if let Ok(reader) = SystemClipboardReader::new() {
                    let watcher = ClipboardWatcher::default();
                    let (rx, running, _handle) = watcher.spawn(reader);
                    let mut guard = application.lock().unwrap();
                    guard.clipboard_running = Some(running);
                    guard.settings.capture.enabled = true;
                    guard.settings.capture.clipboard = true;
                    drop(guard);

                    let app_for_thread = Arc::clone(&application);
                    let win_for_thread = window_weak.clone();
                    let popup_for_thread = popup_weak.clone();

                    std::thread::spawn(move || {
                        while let Ok(text) = rx.recv() {
                            let app_capture = Arc::clone(&app_for_thread);
                            let win_capture = win_for_thread.clone();
                            let popup_capture = popup_for_thread.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                app_capture.lock().unwrap().handle_captured_text(&text);
                                if let Some(w) = win_capture.upgrade() {
                                    update_window(&w, &app_capture.lock().unwrap());
                                }
                                if let Some(popup) = popup_capture.upgrade() {
                                    update_popup_window(&popup, &app_capture.lock().unwrap());
                                }
                            });
                        }
                    });
                }
                #[cfg(windows)]
                if let Ok(reader) = SystemClipboardReader::new() {
                    use lingoes_capture::ClipboardReader;
                    use std::time::Duration;
                    let reader = Arc::new(Mutex::new(reader));
                    let source = Arc::clone(&reader);
                    let app_for_event = Arc::clone(&application);
                    let win_for_event = window_weak.clone();
                    let popup_for_event = popup_weak.clone();
                    if let Ok(listener) = lingoes_capture::windows::WindowsClipboardListener::spawn(
                        Duration::from_millis(300),
                        move || source.lock().ok()?.get_text().ok(),
                        move |text| {
                            let app_capture = Arc::clone(&app_for_event);
                            let win_capture = win_for_event.clone();
                            let popup_capture = popup_for_event.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                app_capture.lock().unwrap().handle_captured_text(&text);
                                if let Some(window) = win_capture.upgrade() {
                                    update_window(&window, &app_capture.lock().unwrap());
                                }
                                if let Some(popup) = popup_capture.upgrade() {
                                    update_popup_window(&popup, &app_capture.lock().unwrap());
                                }
                            });
                        },
                    ) {
                        let mut guard = application.lock().unwrap();
                        guard.clipboard_listener = Some(listener);
                        guard.settings.capture.enabled = true;
                        guard.settings.capture.clipboard = true;
                    }
                }
            } else {
                application.lock().unwrap().stop_clipboard_monitor();
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.lock().unwrap());
            }
        });
    }

    // Change UI language callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        let popup_weak = popup_window.as_weak();
        window.on_change_language(move |lang| {
            let mut app = application.lock().unwrap();
            app.set_language(lang.as_str());
            if let Some(w) = window_weak.upgrade() {
                w.set_ui_lang(lang.clone());
            }
            if let Some(p) = popup_weak.upgrade() {
                p.set_ui_lang(lang);
            }
        });
    }

    // Clear history callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_clear_history(move || {
            let mut app = application.lock().unwrap();
            let _ = app.clear_history();
            if let Some(w) = window_weak.upgrade() {
                update_window(&w, &app);
            }
        });
    }

    // Exit application callback
    {
        let window_weak = window.as_weak();
        window.on_exit_app(move || {
            if let Some(w) = window_weak.upgrade() {
                let _ = w.hide();
            }
            std::process::exit(0);
        });
    }

    // Open URL callback
    {
        window.on_open_url(move |url| {
            if let Ok(parsed) = Url::parse(url.as_str()) {
                let launcher = DefaultBrowserLauncher;
                let _ = launcher.open_url(&parsed);
            }
        });
    }

    if let Some(query) = initial_query {
        let mut app = application.lock().unwrap();
        app.search(query);
        if !app.results().is_empty() {
            app.select(0);
        }
        window.set_query(query.into());
    }
    if let Some(word) = show_popup_word {
        let mut app = application.lock().unwrap();
        app.handle_captured_text(word);
    }

    update_window(&window, &application.lock().unwrap());
    update_popup_window(&popup_window, &application.lock().unwrap());
    let run_result = window.run().map_err(|error| AppError(error.to_string()));

    let settings_result = settings_store
        .save(application.lock().unwrap().settings())
        .map_err(|error| AppError(format!("cannot save settings: {error}")));
    run_result.and(settings_result)
}

#[cfg(windows)]
fn materialize_windows_popup(
    popup: &PopupWindow,
) -> Result<lingoes_platform::windows::WindowsPopupWindowAdapter, AppError> {
    // Slint 1.8 creates the backend HWND when the component is instantiated. Apply passive styles
    // while it remains hidden so the first user-visible show cannot activate the popup.
    let adapter = attach_windows_popup_adapter(popup)?;
    adapter
        .prepare(lingoes_platform::popup::PopupActivation::Passive)
        .map_err(|error| AppError(error.to_string()))?;
    adapter
        .hide()
        .map_err(|error| AppError(error.to_string()))?;
    Ok(adapter)
}

#[cfg(windows)]
fn attach_windows_popup_adapter(
    popup: &PopupWindow,
) -> Result<lingoes_platform::windows::WindowsPopupWindowAdapter, AppError> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = popup.window().window_handle();
    let raw = handle
        .window_handle()
        .map_err(|error| AppError(format!("cannot get popup window handle: {error}")))?;
    let hwnd = match raw.as_raw() {
        RawWindowHandle::Win32(handle) => handle.hwnd.get(),
        _ => {
            return Err(AppError(
                "popup backend did not provide a Win32 HWND".into(),
            ));
        }
    };
    lingoes_platform::windows::WindowsPopupWindowAdapter::from_raw(hwnd)
        .map_err(|error| AppError(error.to_string()))
}

fn update_popup_window(window: &PopupWindow, application: &DesktopApp) {
    window.set_ui_lang(application.language().into());
    window.set_headword(application.quick_popup_word().into());
    let formatted_def = lingoes_core::format_definition_for_display(application.quick_popup_def());
    window.set_definition(formatted_def.into());
    let dict_badge = application
        .selected_entry()
        .and_then(|e| e.dictionary_name())
        .unwrap_or_else(|| {
            if application.quick_popup_word().is_empty() {
                ""
            } else {
                "Lingoes DB"
            }
        });
    window.set_dictionary_name(dict_badge.into());
    window.set_status_text(application.status().into());
    window.set_can_go_previous(application.selected_index().is_some_and(|index| index > 0));
    window.set_can_go_next(
        application
            .selected_index()
            .is_some_and(|index| index.saturating_add(1) < application.results().len()),
    );
    window.set_pinned(application.quick_popup_is_pinned());
    window.set_expanded(application.quick_popup_state() == PopupState::Expanded);

    if application.show_quick_popup() {
        #[cfg(windows)]
        if let Err(error) = show_windows_popup(window, application) {
            eprintln!("cannot show native popup: {error}");
        }
        #[cfg(not(windows))]
        let _ = window.show();
    } else {
        #[cfg(windows)]
        if let Ok(adapter) = attach_windows_popup_adapter(window) {
            let _ = adapter.hide();
        }
        let _ = window.hide();
    }
}

#[cfg(windows)]
fn show_windows_popup(window: &PopupWindow, application: &DesktopApp) -> Result<(), AppError> {
    use lingoes_platform::popup::{
        LogicalSize, PopupActivation, PopupPlacementRequest, place_popup,
    };

    let monitors =
        lingoes_platform::windows::popup_monitors().map_err(|error| AppError(error.to_string()))?;
    let cursor = lingoes_platform::windows::popup_cursor_position()
        .map_err(|error| AppError(error.to_string()))?;
    let (logical_width, logical_height) = if application.quick_popup_state() == PopupState::Expanded
    {
        (560.0, 480.0)
    } else {
        (
            f64::from(application.settings().popup.logical_width.max(320)),
            f64::from(application.settings().popup.logical_height.max(180)),
        )
    };
    let placement = place_popup(
        PopupPlacementRequest {
            bounds: None,
            cursor,
            logical_size: LogicalSize::new(logical_width, logical_height),
            selected_monitor: None,
        },
        &monitors,
    )
    .ok_or_else(|| AppError("no monitor is available for popup placement".into()))?;
    let adapter = attach_windows_popup_adapter(window)?;
    adapter
        .show(PopupActivation::Passive, placement.rect)
        .map_err(|error| AppError(error.to_string()))
}

fn compute_unit_rows(category: &str, from_unit: &str, value_str: &str) -> Vec<UnitRow> {
    let value = value_str.trim().parse::<f64>().unwrap_or(0.0);
    let units: &[(&str, &str)] = match category {
        "length" => &[
            ("Mét", "m"),
            ("Kilômét", "km"),
            ("Xentimét", "cm"),
            ("Milimét", "mm"),
            ("Inch (in)", "inch"),
            ("Foot (ft)", "foot"),
            ("Yard (yd)", "yard"),
            ("Dặm (Mile)", "mile"),
        ],
        "weight" => &[
            ("Kilôgam", "kg"),
            ("Gam", "g"),
            ("Miligam", "mg"),
            ("Pound (lb)", "lb"),
            ("Ounce (oz)", "oz"),
            ("Tấn", "ton"),
        ],
        "temperature" => &[
            ("Độ Celsius (°C)", "C"),
            ("Độ Fahrenheit (°F)", "F"),
            ("Kelvin (K)", "K"),
        ],
        "volume" => &[
            ("Lít (L)", "L"),
            ("Mililít (mL)", "mL"),
            ("Gallon (US)", "gal"),
            ("Fluid Ounce (fl oz)", "fl_oz"),
        ],
        _ => &[],
    };

    units
        .iter()
        .map(|(name, sym)| {
            let val_str = match lingoes_core::convert_units(category, value, from_unit, sym) {
                Some(v) => {
                    if v.abs() >= 100_000.0 || (v.abs() < 0.0001 && v != 0.0) {
                        format!("{:.4e}", v)
                    } else {
                        let s = format!("{:.4}", v);
                        let s = s.trim_end_matches('0');
                        s.trim_end_matches('.').to_string()
                    }
                }
                None => "—".to_string(),
            };
            UnitRow {
                name: SharedString::from(*name),
                symbol: SharedString::from(*sym),
                value_str: SharedString::from(val_str),
            }
        })
        .collect()
}

fn update_window(window: &AppWindow, application: &DesktopApp) {
    let rows: Vec<SearchResult> = application
        .results()
        .iter()
        .map(|entry| SearchResult {
            headword: SharedString::from(entry.headword()),
            definition: SharedString::from(entry.definition()),
            dictionary_name: SharedString::from(""),
        })
        .collect();
    window.set_results(ModelRc::new(VecModel::from(rows)));
    window.set_selected_index(
        application
            .selected_index()
            .map_or(-1, |index| index as i32),
    );
    window.set_can_go_back(application.selected_index().is_some_and(|index| index > 0));
    window.set_can_go_forward(
        application
            .selected_index()
            .is_some_and(|index| index + 1 < application.results().len()),
    );
    window.set_selected_headword(application.selected_headword().into());
    let formatted_def =
        lingoes_core::format_definition_for_display(application.selected_definition());
    window.set_selected_definition(formatted_def.into());
    let dict_badge = application
        .selected_entry()
        .and_then(|e| e.dictionary_name())
        .unwrap_or_else(|| {
            if application.selected_headword().is_empty() {
                ""
            } else {
                "Lingoes DB"
            }
        });
    window.set_selected_dict_badge(dict_badge.into());

    let lines_data: Vec<FormattedEntryLine> = if application.selected_headword().is_empty() {
        Vec::new()
    } else {
        lingoes_core::parse_formatted_lines(application.selected_definition())
            .into_iter()
            .map(|l| FormattedEntryLine {
                kind: SharedString::from(l.kind),
                prefix: SharedString::from(l.prefix),
                body: SharedString::from(l.body),
            })
            .collect()
    };
    window.set_definition_lines(ModelRc::new(VecModel::from(lines_data)));

    let dict_rows: Vec<DictionaryItem> = application
        .dictionaries()
        .iter()
        .map(|meta| DictionaryItem {
            id: meta.id as i32,
            name: SharedString::from(&meta.name),
            format: SharedString::from(&meta.format),
            entry_count: meta.entry_count as i32,
            enabled: meta.enabled,
        })
        .collect();
    window.set_dictionaries(ModelRc::new(VecModel::from(dict_rows)));

    window.set_clipboard_monitor_enabled(application.is_clipboard_monitoring());
    window.set_cursor_capture_enabled(application.settings().capture.cursor);
    window.set_selection_capture_enabled(matches!(
        application.settings().capture.mode,
        CaptureModeSetting::SelectedText
    ));
    window.set_ocr_fallback_enabled(application.settings().capture.ocr_enabled);
    window.set_auto_pronounce_enabled(application.settings().popup.auto_pronounce);
    window.set_ui_lang(application.language().into());

    let notebook_items: Vec<SharedString> = application
        .notebook_items()
        .into_iter()
        .map(SharedString::from)
        .collect();
    window.set_notebook_items(ModelRc::new(VecModel::from(notebook_items)));
    let is_fav = application.is_notebook_word(application.selected_headword());
    window.set_is_current_favorite(is_fav);

    let history_items: Vec<SharedString> = application
        .history_items()
        .into_iter()
        .map(SharedString::from)
        .collect();
    window.set_history_items(ModelRc::new(VecModel::from(history_items)));

    window.set_status_text(application.status().into());
    window.set_status_is_error(application.has_error());
}

pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<String, AppError> {
    let translator = Arc::new(GoogleTranslator::new());
    let tts = Arc::new(SystemTtsEngine::new());
    run_with_services(arguments, translator, tts)
}

pub fn run_with_services(
    arguments: impl IntoIterator<Item = String>,
    translator: Arc<dyn Translator>,
    tts: Arc<dyn TtsEngine>,
) -> Result<String, AppError> {
    let arguments: Vec<_> = arguments.into_iter().collect();
    if arguments.is_empty() {
        return Err(AppError(USAGE.into()));
    }

    let command = arguments[0].as_str();
    match command {
        "import" => {
            if arguments.len() < 3 {
                return Err(AppError(format!(
                    "missing required arguments for import\n{USAGE}"
                )));
            }
            let database_path = &arguments[1];
            let source_path = &arguments[2];
            let mut name = None;
            let mut i = 3;
            while i < arguments.len() {
                if arguments[i] == "--name" && i + 1 < arguments.len() {
                    name = Some(arguments[i + 1].as_str());
                    i += 2;
                } else {
                    i += 1;
                }
            }

            let mut dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            let report = dictionary
                .import_path_with_name(source_path, name)
                .map_err(|error| AppError(error.to_string()))?;
            let entry_text = if report.imported == 1 {
                "entry"
            } else {
                "entries"
            };
            Ok(format!(
                "Imported {} {} ({} new, {} updated)",
                report.imported, entry_text, report.inserted, report.updated
            ))
        }
        "lookup" | "full-text" => {
            if arguments.len() < 3 {
                return Err(AppError(format!("missing query for lookup\n{USAGE}")));
            }
            let database_path = &arguments[1];
            let query_str = &arguments[2];
            let dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            let query =
                LookupQuery::new(query_str).map_err(|_| AppError("query is empty".into()))?;
            let entries = if command == "full-text" {
                dictionary.lookup_full_text(&query, 20)
            } else {
                dictionary.lookup_prefix(&query, 20)
            }
            .map_err(|error| AppError(error.to_string()))?;
            if entries.is_empty() {
                return Ok("No results".into());
            }
            Ok(entries
                .iter()
                .map(|entry| format!("{}\t{}", entry.headword(), entry.definition()))
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "list-dictionaries" => {
            if arguments.len() < 2 {
                return Err(AppError(format!(
                    "missing database path for list-dictionaries\n{USAGE}"
                )));
            }
            let database_path = &arguments[1];
            let dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            let list = dictionary
                .list_dictionaries()
                .map_err(|error| AppError(error.to_string()))?;
            if list.is_empty() {
                return Ok("No dictionaries found".into());
            }
            Ok(list
                .iter()
                .map(|d| {
                    format!(
                        "[{}] {} ({}) - {} entries [{}]",
                        d.id,
                        d.name,
                        d.format,
                        d.entry_count,
                        if d.enabled { "enabled" } else { "disabled" }
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "dict" => {
            if arguments.len() < 4 || arguments[1] != "info" {
                return Err(AppError(format!(
                    "usage: dict info <dictionary.db> <id>\n{USAGE}"
                )));
            }
            let database_path = &arguments[2];
            let id = arguments[3]
                .parse::<i64>()
                .map_err(|e| AppError(format!("invalid dictionary id: {e}")))?;
            let dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            let metadata = dictionary
                .get_dictionary_metadata(id)
                .map_err(|error| AppError(error.to_string()))?
                .ok_or_else(|| AppError(format!("dictionary {id} not found")))?;
            let field =
                |label: &str, value: Option<&str>| format!("{label}: {}", value.unwrap_or("-"));
            Ok([
                format!("ID: {id}"),
                format!("Title: {}", metadata.title),
                field("Author", metadata.author.as_deref()),
                field("Publisher", metadata.publisher.as_deref()),
                field("Website", metadata.website.as_deref()),
                field("Copyright", metadata.copyright.as_deref()),
                field("Description", metadata.description.as_deref()),
                field("Source language", metadata.source_lang.as_deref()),
                field("Target language", metadata.target_lang.as_deref()),
                field("Format", metadata.format.as_deref()),
                format!("Entries: {}", metadata.entry_count.unwrap_or_default()),
            ]
            .join("\n"))
        }
        "toggle-dictionary" => {
            if arguments.len() < 4 {
                return Err(AppError(format!(
                    "missing arguments for toggle-dictionary\n{USAGE}"
                )));
            }
            let database_path = &arguments[1];
            let id = arguments[2]
                .parse::<i64>()
                .map_err(|e| AppError(format!("invalid dictionary id: {e}")))?;
            let enabled = arguments[3]
                .parse::<bool>()
                .map_err(|e| AppError(format!("invalid boolean value: {e}")))?;
            let mut dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            dictionary
                .toggle_dictionary(id, enabled)
                .map_err(|error| AppError(error.to_string()))?;
            Ok(format!(
                "Dictionary {id} {}",
                if enabled { "enabled" } else { "disabled" }
            ))
        }
        "delete-dictionary" => {
            if arguments.len() < 3 {
                return Err(AppError(format!(
                    "missing arguments for delete-dictionary\n{USAGE}"
                )));
            }
            let database_path = &arguments[1];
            let id = arguments[2]
                .parse::<i64>()
                .map_err(|e| AppError(format!("invalid dictionary id: {e}")))?;
            let mut dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            dictionary
                .delete_dictionary(id)
                .map_err(|error| AppError(error.to_string()))?;
            Ok(format!("Dictionary {id} deleted"))
        }
        "translate" => {
            if arguments.len() < 2 {
                return Err(AppError(format!("missing text for translate\n{USAGE}")));
            }
            let text = &arguments[1];
            let mut from = "auto";
            let mut to = "vi";
            let mut _engine = "google";

            let mut i = 2;
            while i < arguments.len() {
                if arguments[i] == "--from" && i + 1 < arguments.len() {
                    from = arguments[i + 1].as_str();
                    i += 2;
                } else if arguments[i] == "--to" && i + 1 < arguments.len() {
                    to = arguments[i + 1].as_str();
                    i += 2;
                } else if arguments[i] == "--engine" && i + 1 < arguments.len() {
                    _engine = arguments[i + 1].as_str();
                    i += 2;
                } else {
                    i += 1;
                }
            }

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| AppError(format!("runtime error: {e}")))?;
            let req = TranslationRequest::new(text.as_str(), from, to);
            let translated = rt
                .block_on(translator.translate(req))
                .map_err(|e| AppError(e.to_string()))?;
            Ok(translated.text)
        }
        "tts" => {
            if arguments.len() < 2 {
                return Err(AppError(format!("missing text for tts\n{USAGE}")));
            }
            let text = &arguments[1];
            let mut lang = "en";
            let mut i = 2;
            while i < arguments.len() {
                if arguments[i] == "--lang" && i + 1 < arguments.len() {
                    lang = arguments[i + 1].as_str();
                    i += 2;
                } else {
                    i += 1;
                }
            }
            tts.speak(text, lang).map_err(|e| AppError(e.to_string()))?;
            Ok(format!("Spoken: \"{text}\" ({lang})"))
        }
        _ => Err(AppError(format!(
            "unknown command or invalid arguments: {command}\n{USAGE}"
        ))),
    }
}

pub fn log_startup(message: &str) {
    use std::io::Write;
    eprintln!("[Lingoes] {message}");
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let log_file = dir.join("lingoes_startup.log");
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)
            {
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let _ = writeln!(f, "[+{timestamp}s] {message}");
            }
        }
    }
}

pub fn show_fatal_error(title: &str, message: &str) {
    log_startup(&format!("FATAL ERROR - {title}: {message}"));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let log_file = dir.join("lingoes_crash.log");
            let _ = std::fs::write(&log_file, format!("{title}\n\n{message}\n"));
        }
    }
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        use windows::core::HSTRING;
        let title_h = HSTRING::from(title);
        let message_h = HSTRING::from(message);
        unsafe {
            let _ = MessageBoxW(None, &message_h, &title_h, MB_OK | MB_ICONERROR);
        }
    }
}

pub fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("Application panic:\n{info}");
        show_fatal_error("Lingoes RS - Fatal Error", &msg);
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use lingoes_translator::MockTranslator;
    use lingoes_tts::MockTtsEngine;

    fn temporary_path(name: &str, extension: &str) -> PathBuf {
        let unique = format!(
            "lingoes-{name}-{}-{}.{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            extension
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn usage_is_returned_when_no_command_is_given() {
        let error = run(Vec::<String>::new()).unwrap_err();
        assert!(error.to_string().contains("Usage:"));
    }

    #[test]
    fn unknown_command_is_rejected() {
        let error = run(["nope".to_owned()]).unwrap_err();
        assert!(error.to_string().contains("unknown command"));
    }

    #[test]
    fn application_search_populates_results_and_selects_first_entry() {
        let database_path = temporary_path("search", "db");
        let source_path = temporary_path("search", "tsv");
        std::fs::write(
            &source_path,
            "rust\tA systems language\nrustic\tSimple or rural\n",
        )
        .unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();
        application.import_path(&source_path).unwrap();

        application.search("RUST");

        assert_eq!(application.results().len(), 2);
        assert_eq!(application.selected_index(), Some(0));
        assert_eq!(application.selected_definition(), "A systems language");
        assert!(application.status().contains("2"));
        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn live_search_updates_on_each_distinct_trimmed_query() {
        let database_path = temporary_path("live-search", "db");
        let source_path = temporary_path("live-search", "tsv");
        std::fs::write(
            &source_path,
            "hello\tGreeting\nhelloed\tPast tense\nhelp\tAssist\n",
        )
        .unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();
        application.import_path(&source_path).unwrap();

        assert!(application.live_search("h"));
        assert_eq!(application.selected_headword(), "hello");
        assert!(application.live_search("hel"));
        assert_eq!(application.results().len(), 3);
        assert!(!application.live_search(" hel "));
        assert!(application.live_search("hello"));
        assert_eq!(application.selected_headword(), "hello");
        assert_eq!(application.results()[0].headword(), "hello");

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn live_search_blank_input_clears_results_once() {
        let database_path = temporary_path("live-search-blank", "db");
        let source_path = temporary_path("live-search-blank", "tsv");
        std::fs::write(&source_path, "hello\tGreeting\n").unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();
        application.import_path(&source_path).unwrap();
        application.search("hello");

        assert!(application.live_search(""));
        assert!(application.results().is_empty());
        assert!(!application.live_search("   "));

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn go_home_resets_results_selection_and_status() {
        let database_path = temporary_path("home-test", "db");
        let source_path = temporary_path("home-test", "tsv");
        std::fs::write(&source_path, "rust\tA systems language\n").unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();
        application.import_path(&source_path).unwrap();

        application.search("rust");
        assert_eq!(application.selected_headword(), "rust");
        assert_eq!(application.results().len(), 1);

        application.go_home();
        assert_eq!(application.selected_headword(), "");
        assert_eq!(application.results().len(), 0);
        assert_eq!(application.selected_index(), None);
        assert_eq!(application.status(), "Nhập từ khóa để tra cứu");
        assert!(!application.has_error());

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn blank_search_clears_results_without_becoming_an_error() {
        let database_path = temporary_path("blank", "db");
        let mut application = DesktopApp::open(&database_path).unwrap();

        application.search("   ");

        assert!(application.results().is_empty());
        assert_eq!(application.selected_index(), None);
        assert!(!application.has_error());
        let _ = std::fs::remove_file(database_path);
    }

    #[test]
    fn selecting_a_result_updates_the_detail() {
        let database_path = temporary_path("selection", "db");
        let source_path = temporary_path("selection", "tsv");
        std::fs::write(&source_path, "app\tA program\napple\tA fruit\n").unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();
        application.import_path(&source_path).unwrap();
        application.search("app");

        application.select(1);

        assert_eq!(application.selected_index(), Some(1));
        assert_eq!(application.selected_headword(), "apple");
        assert_eq!(application.selected_definition(), "A fruit");
        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn full_text_search_finds_words_in_definitions() {
        let database_path = temporary_path("fts-app", "db");
        let source_path = temporary_path("fts-app", "tsv");
        std::fs::write(&source_path, "tiếng việt\tNgôn ngữ Việt Nam\n").unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();
        application.import_path(&source_path).unwrap();

        application.search_full_text("Việt Nam");

        assert_eq!(application.results().len(), 1);
        assert_eq!(application.selected_headword(), "tiếng việt");
        assert!(application.status().contains("1"));
        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn cli_full_text_command_works() {
        let database_path = temporary_path("cli-fts", "db");
        let source_path = temporary_path("cli-fts", "tsv");
        std::fs::write(&source_path, "rust\ta systems language\n").unwrap();
        let import_output = run([
            "import".into(),
            database_path.to_str().unwrap().into(),
            source_path.to_str().unwrap().into(),
        ])
        .unwrap();
        assert!(import_output.contains("Imported 1 entry"));

        let lookup_output = run([
            "full-text".into(),
            database_path.to_str().unwrap().into(),
            "language".into(),
        ])
        .unwrap();
        assert_eq!(lookup_output, "rust\ta systems language");
        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn importing_a_tsv_reports_success_and_malformed_input_reports_an_error() {
        let database_path = temporary_path("import", "db");
        let valid_path = temporary_path("valid", "tsv");
        let invalid_path = temporary_path("invalid", "tsv");
        std::fs::write(&valid_path, "hello\tXin chao\n").unwrap();
        std::fs::write(&invalid_path, "missing tab\n").unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();

        application.import_path(&valid_path).unwrap();
        assert_eq!(application.status(), "Imported 1 entry (1 new, 0 updated)");
        assert!(!application.has_error());

        assert!(application.import_path(&invalid_path).is_err());
        assert!(application.status().contains("line 1"));
        assert!(application.has_error());
        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(valid_path);
        let _ = std::fs::remove_file(invalid_path);
    }

    #[test]
    fn portable_database_path_is_stable_beside_the_executable() {
        let executable = Path::new("/opt/lingoes/lingoes-desktop");
        assert_eq!(
            portable_database_path(executable).unwrap(),
            Path::new("/opt/lingoes/data/lingoes.db")
        );
    }

    #[test]
    fn dictionary_management_list_toggle_and_delete() {
        let database_path = temporary_path("dict-mgr", "db");
        let source_path = temporary_path("dict-mgr", "tsv");
        std::fs::write(&source_path, "water\tNước\n").unwrap();

        let mut app = DesktopApp::open(&database_path).unwrap();
        app.import_path_with_name(&source_path, Some("English-Vietnamese"))
            .unwrap();

        let dicts = app.list_dictionaries().unwrap();
        assert_eq!(dicts.len(), 1);
        let dict_id = dicts[0].id;
        assert_eq!(dicts[0].name, "English-Vietnamese");
        assert!(dicts[0].enabled);

        // Toggle off
        app.toggle_dictionary(dict_id, false).unwrap();
        assert!(!app.dictionaries()[0].enabled);

        // Toggle on
        app.toggle_dictionary(dict_id, true).unwrap();
        assert!(app.dictionaries()[0].enabled);

        // Delete
        app.delete_dictionary(dict_id).unwrap();
        assert_eq!(app.dictionaries().len(), 0);

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn translate_flow_with_mock_translator() {
        let database_path = temporary_path("trans-mock", "db");
        let dict = SqliteDictionary::open(&database_path).unwrap();

        let mut mock_trans = MockTranslator::new();
        mock_trans.add_translation("Hello", "en", "vi", "Xin chào");
        let mock_trans = Arc::new(mock_trans);
        let mock_tts = Arc::new(MockTtsEngine::new());

        let mut app = DesktopApp::with_services(dict, mock_trans, mock_tts).unwrap();
        let res = app.translate("Hello", "en", "vi").unwrap();
        assert_eq!(res, "Xin chào");
        assert!(app.status().starts_with("Dịch thành công"));
        assert!(app.status().contains("Mock"));

        let _ = std::fs::remove_file(database_path);
    }

    #[test]
    fn tts_flow_with_mock_tts_engine() {
        let database_path = temporary_path("tts-mock", "db");
        let dict = SqliteDictionary::open(&database_path).unwrap();
        let mock_trans = Arc::new(MockTranslator::new());
        let mock_tts = Arc::new(MockTtsEngine::new());

        let mut app = DesktopApp::with_services(dict, mock_trans, mock_tts.clone()).unwrap();
        app.speak("Good morning", "en").unwrap();

        assert_eq!(
            mock_tts.last_spoken(),
            Some(("Good morning".into(), "en".into()))
        );
        assert!(app.status().contains("Good morning"));

        let _ = std::fs::remove_file(database_path);
    }

    #[test]
    fn clipboard_capture_and_quick_popup_lookup() {
        let database_path = temporary_path("clip-popup", "db");
        let source_path = temporary_path("clip-popup", "tsv");
        std::fs::write(&source_path, "keyboard\tBàn phím máy tính\n").unwrap();

        let mut app = DesktopApp::open(&database_path).unwrap();
        app.import_path(&source_path).unwrap();

        assert!(!app.show_quick_popup());

        app.handle_captured_text("  keyboard  ");
        assert!(app.show_quick_popup());
        assert_eq!(app.quick_popup_word(), "keyboard");
        assert_eq!(app.quick_popup_def(), "Bàn phím máy tính");

        app.dismiss_quick_popup();
        assert!(!app.show_quick_popup());

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn desktop_app_search_vietnamese_diacritics_and_history() {
        let database_path = temporary_path("vi-search-history", "db");
        let source_path = temporary_path("vi-search-history", "tsv");
        std::fs::write(
            &source_path,
            "chào\tLời chào hỏi\nchào cờ\tNghi thức chào cờ đầu tuần\nanh chàng keo kiệt\tNgười đàn ông bủn xỉn\n",
        )
        .unwrap();

        let mut app = DesktopApp::open(&database_path).unwrap();
        app.import_path(&source_path).unwrap();

        // 1. Live search with unaccented single word
        assert!(app.live_search("chao"));
        assert!(!app.results().is_empty());
        assert_eq!(app.results()[0].headword(), "chào");

        // 2. Live search with unaccented multi-word (2 words)
        assert!(app.live_search("chao co"));
        assert!(!app.results().is_empty());
        assert_eq!(app.results()[0].headword(), "chào cờ");

        // 3. Search with 4 words (unaccented)
        app.search("anh chang keo kiet");
        assert!(!app.results().is_empty());
        assert_eq!(app.results()[0].headword(), "anh chàng keo kiệt");

        // 4. Verify search history was recorded
        let history = app.history_items();
        assert!(history.contains(&"anh chang keo kiet".to_string()));

        // 5. Clear history
        app.clear_history().unwrap();
        assert!(app.history_items().is_empty());

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn cli_list_dictionaries_command_works() {
        let database_path = temporary_path("cli-list-dict", "db");
        let source_path = temporary_path("cli-list-dict", "tsv");
        std::fs::write(&source_path, "sun\tMặt trời\n").unwrap();

        let _ = run([
            "import".into(),
            database_path.to_str().unwrap().into(),
            source_path.to_str().unwrap().into(),
            "--name".into(),
            "SolarDict".into(),
        ])
        .unwrap();

        let output = run([
            "list-dictionaries".into(),
            database_path.to_str().unwrap().into(),
        ])
        .unwrap();
        assert!(output.contains("SolarDict"));
        assert!(output.contains("enabled"));

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn cli_toggle_and_delete_dictionary_command_works() {
        let database_path = temporary_path("cli-toggle-del", "db");
        let source_path = temporary_path("cli-toggle-del", "tsv");
        std::fs::write(&source_path, "moon\tMặt trăng\n").unwrap();

        let _ = run([
            "import".into(),
            database_path.to_str().unwrap().into(),
            source_path.to_str().unwrap().into(),
            "--name".into(),
            "LunarDict".into(),
        ])
        .unwrap();

        // Toggle
        let toggle_out = run([
            "toggle-dictionary".into(),
            database_path.to_str().unwrap().into(),
            "1".into(),
            "false".into(),
        ])
        .unwrap();
        assert!(toggle_out.contains("disabled"));

        // Delete
        let del_out = run([
            "delete-dictionary".into(),
            database_path.to_str().unwrap().into(),
            "1".into(),
        ])
        .unwrap();
        assert!(del_out.contains("deleted"));

        let _ = std::fs::remove_file(database_path);
        let _ = std::fs::remove_file(source_path);
    }

    #[test]
    fn cli_translate_command_works_with_service() {
        let mut mock_trans = MockTranslator::new();
        mock_trans.add_translation("cat", "en", "vi", "con mèo");
        let mock_trans = Arc::new(mock_trans);
        let mock_tts = Arc::new(MockTtsEngine::new());

        let out = run_with_services(
            [
                "translate".into(),
                "cat".into(),
                "--from".into(),
                "en".into(),
                "--to".into(),
                "vi".into(),
            ],
            mock_trans,
            mock_tts,
        )
        .unwrap();

        assert_eq!(out, "con mèo");
    }

    #[test]
    fn cli_tts_command_works_with_service() {
        let mock_trans = Arc::new(MockTranslator::new());
        let mock_tts = Arc::new(MockTtsEngine::new());

        let out = run_with_services(
            ["tts".into(), "hello".into(), "--lang".into(), "en".into()],
            mock_trans,
            mock_tts.clone(),
        )
        .unwrap();

        assert_eq!(out, "Spoken: \"hello\" (en)");
        assert_eq!(mock_tts.last_spoken(), Some(("hello".into(), "en".into())));
    }

    #[test]
    fn desktop_app_holds_and_modifies_settings() {
        let database_path = temporary_path("settings-test", "db");
        let mut app = DesktopApp::open(&database_path).unwrap();
        assert_eq!(app.settings().schema_version, 1);
        assert!(!app.settings().shell.close_to_tray);
        let mut updated = app.settings().clone();
        updated.shell.close_to_tray = true;
        app.set_settings(updated);
        assert!(app.settings().shell.close_to_tray);
        let _ = std::fs::remove_file(database_path);
    }

    #[test]
    fn desktop_app_language_switching_and_bilingual_status() {
        let database_path = temporary_path("language-test", "db");
        let mut app = DesktopApp::open(&database_path).unwrap();
        assert_eq!(app.language(), "vi");

        // Status reflects default Vietnamese
        app.search("");
        assert_eq!(app.status(), "Nhập từ khóa để tra cứu");

        // Switch to English
        app.set_language("en");
        assert_eq!(app.language(), "en");
        app.search("");
        assert_eq!(app.status(), "Enter a word or phrase to look up");

        // Switch back to Vietnamese
        app.set_language("vi");
        assert_eq!(app.language(), "vi");
        app.search("");
        assert_eq!(app.status(), "Nhập từ khóa để tra cứu");

        let _ = std::fs::remove_file(database_path);
    }

    #[test]
    fn resolve_database_path_uses_data_root() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("lexiforge");
        let data_root = resolve_app_data_root(&exe).unwrap();
        let db_path = resolve_database_path(&data_root);
        assert!(db_path.ends_with("lexiforge.db") || db_path.ends_with("lingoes.db"));
    }

    #[test]
    fn desktop_app_notebook_toggle_remove_and_export_html() {
        let database_path = temporary_path("notebook-test", "db");
        let mut app = DesktopApp::open(&database_path).unwrap();

        assert!(!app.is_notebook_word("rust"));
        assert!(app.notebook_items().is_empty());

        // Toggle add
        let added = app
            .toggle_notebook("rust", "A systems programming language")
            .unwrap();
        assert!(added);
        assert!(app.is_notebook_word("rust"));
        let items = app.notebook_items();
        assert_eq!(items, vec!["rust".to_string()]);

        // Toggle remove
        let removed = app.toggle_notebook("rust", "").unwrap();
        assert!(!removed);
        assert!(!app.is_notebook_word("rust"));
        assert!(app.notebook_items().is_empty());

        // Re-add and export HTML
        app.toggle_notebook("slint", "A declarative GUI framework")
            .unwrap();
        app.toggle_notebook("rust", "Systems language").unwrap();
        let export_path = app.export_notebook_html().unwrap();
        assert!(export_path.ends_with(".html"));
        let content = std::fs::read_to_string(&export_path).unwrap();
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("Sổ tay từ vựng"));
        assert!(content.contains("slint"));
        assert!(content.contains("rust"));
        let _ = std::fs::remove_file(&export_path);

        // Direct remove
        app.remove_from_notebook("slint").unwrap();
        assert!(!app.is_notebook_word("slint"));
        assert_eq!(app.notebook_items(), vec!["rust".to_string()]);

        let _ = std::fs::remove_file(database_path);
    }
}
