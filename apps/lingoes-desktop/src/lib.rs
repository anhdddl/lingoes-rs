use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(not(windows))]
use lingoes_capture::ClipboardWatcher;
use lingoes_capture::SystemClipboardReader;
use lingoes_core::{DictionaryEntry, LookupQuery};
use lingoes_dictionary::{DictionaryMeta, SqliteDictionary};
use lingoes_platform::settings::{
    CaptureModeSetting, DataRoot, Settings, SettingsStore, resolve_data_root,
    system_installed_data_base,
};
use lingoes_translator::{GoogleTranslator, OpenAiTranslator, Translator};
use lingoes_tts::{SpeechOptions, SystemTtsEngine, TtsEngine, TtsError, Voice, VoiceSelection};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

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
        };
        let _ = app.list_dictionaries();
        Ok(app)
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

    pub fn search(&mut self, query: &str) {
        self.search_with_mode(query, false);
    }

    pub fn search_full_text(&mut self, query: &str) {
        self.search_with_mode(query, true);
    }

    fn search_with_mode(&mut self, query: &str, full_text: bool) {
        let Ok(query) = LookupQuery::new(query) else {
            self.results.clear();
            self.selected_index = None;
            self.status = "Nhập từ khóa để tra cứu".into();
            self.has_error = false;
            return;
        };
        let result = if full_text {
            self.dictionary.lookup_full_text(&query, SEARCH_LIMIT)
        } else {
            self.dictionary.lookup_prefix(&query, SEARCH_LIMIT)
        };
        match result {
            Ok(results) => {
                self.results = results;
                self.selected_index = (!self.results.is_empty()).then_some(0);
                self.status = match self.results.len() {
                    0 => "Không tìm thấy từ phù hợp".into(),
                    1 => "Tìm thấy 1 kết quả".into(),
                    count => format!("Tìm thấy {count} kết quả"),
                };
                self.has_error = false;
            }
            Err(error) => self.set_error(error),
        }
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
        let result = rt.block_on(translator.translate(text, from, to));
        match result {
            Ok(translated) => {
                self.status = "Dịch thành công".into();
                self.has_error = false;
                Ok(translated)
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
    ) -> Result<String, AppError> {
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
        let result = rt.block_on(translator.translate(text, from, to));
        match result {
            Ok(translated) => {
                self.status = "Dịch thành công".into();
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
                self.status = "Đã dừng phát âm".into();
                self.has_error = false;
                Ok(())
            }
            Err(TtsError::Unsupported(_)) => {
                self.status = "Dừng phát âm không khả dụng trên hệ thống này".into();
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
        self.status = "Đã sao chép vào bộ nhớ tạm".into();
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
    let executable = std::env::current_exe()
        .map_err(|error| AppError(format!("cannot locate executable: {error}")))?;
    let data_root = resolve_app_data_root(&executable)?;
    let settings_store = SettingsStore::new(data_root.settings_path());
    let settings = settings_store
        .load()
        .map_err(|error| AppError(format!("cannot load settings: {error}")))?;
    let database_path = resolve_database_path(&data_root);
    let mut desktop_app = DesktopApp::open(database_path)?;
    desktop_app.set_settings(settings);
    let application = Arc::new(Mutex::new(desktop_app));
    let window = AppWindow::new().map_err(|error| AppError(error.to_string()))?;
    let popup_window = PopupWindow::new().map_err(|error| AppError(error.to_string()))?;
    #[cfg(windows)]
    let _popup_native = materialize_windows_popup(&popup_window)?;

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

    // Full text search callback
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

    // Translate callback
    {
        let application = Arc::clone(&application);
        let window_weak = window.as_weak();
        window.on_translate(move |text, from, to, engine| {
            if let Some(w) = window_weak.upgrade() {
                w.set_is_translating(true);
                let base_url = w.get_openai_base_url();
                let model = w.get_openai_model();
                let api_key = w.get_openai_api_key();

                let res = application.lock().unwrap().translate_with_engine(
                    text.as_str(),
                    from.as_str(),
                    to.as_str(),
                    TranslationOptions {
                        engine: engine.as_str(),
                        base_url: Some(base_url.as_str()),
                        model: Some(model.as_str()),
                        api_key: Some(api_key.as_str()),
                    },
                );
                w.set_is_translating(false);
                match res {
                    Ok(translated) => {
                        w.set_translate_target_text(translated.into());
                    }
                    Err(err) => {
                        w.set_translate_target_text(format!("Lỗi: {err}").into());
                    }
                }
                update_window(&w, &application.lock().unwrap());
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
    window.set_headword(application.quick_popup_word().into());
    window.set_definition(application.quick_popup_def().into());
    window.set_dictionary_name(if application.quick_popup_word().is_empty() {
        "".into()
    } else {
        "Lingoes DB".into()
    });
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
    window.set_selected_headword(application.selected_headword().into());
    window.set_selected_definition(application.selected_definition().into());
    window.set_selected_dict_badge(if application.selected_headword().is_empty() {
        "".into()
    } else {
        "Lingoes DB".into()
    });

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
            let translated = rt
                .block_on(translator.translate(text, from, to))
                .map_err(|e| AppError(e.to_string()))?;
            Ok(translated)
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
        assert_eq!(app.status(), "Dịch thành công");

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
    fn resolve_database_path_uses_data_root() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("lexiforge");
        let data_root = resolve_app_data_root(&exe).unwrap();
        let db_path = resolve_database_path(&data_root);
        assert!(db_path.ends_with("lexiforge.db") || db_path.ends_with("lingoes.db"));
    }
}
