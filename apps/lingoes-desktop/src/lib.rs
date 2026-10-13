use std::cell::RefCell;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lingoes_core::{DictionaryEntry, LookupQuery};
use lingoes_dictionary::SqliteDictionary;
use slint::{ModelRc, SharedString, VecModel};

slint::include_modules!();

const USAGE: &str = "Usage:\n  lingoes-desktop import <dictionary.db> <entries.tsv>\n  lingoes-desktop lookup <dictionary.db> <query>\n  lingoes-desktop              Launch the graphical application";
const SEARCH_LIMIT: usize = 100;

#[derive(Debug)]
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
    status: String,
    has_error: bool,
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
        Ok(Self {
            dictionary,
            results: Vec::new(),
            selected_index: None,
            status: "Type a word to search".into(),
            has_error: false,
        })
    }

    pub fn search(&mut self, query: &str) {
        let Ok(query) = LookupQuery::new(query) else {
            self.results.clear();
            self.selected_index = None;
            self.status = "Type a word to search".into();
            self.has_error = false;
            return;
        };
        match self.dictionary.lookup_prefix(&query, SEARCH_LIMIT) {
            Ok(results) => {
                self.results = results;
                self.selected_index = (!self.results.is_empty()).then_some(0);
                self.status = match self.results.len() {
                    0 => "No results".into(),
                    1 => "1 result".into(),
                    count => format!("{count} results"),
                };
                self.has_error = false;
            }
            Err(error) => self.set_error(error),
        }
    }

    pub fn import_path(&mut self, source_path: impl AsRef<Path>) -> Result<(), AppError> {
        let source_path = source_path.as_ref();
        let result: Result<(), AppError> = (|| {
            let source = File::open(source_path).map_err(|error| {
                AppError(format!("cannot open {}: {error}", source_path.display()))
            })?;
            let report = self
                .dictionary
                .import_tsv(source)
                .map_err(|error| AppError(error.to_string()))?;
            self.status = if report.imported == 1 {
                "Imported 1 entry".into()
            } else {
                format!("Imported {} entries", report.imported)
            };
            self.has_error = false;
            Ok(())
        })();
        if let Err(error) = &result {
            self.status = error.to_string();
            self.has_error = true;
        }
        result
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

    fn set_error(&mut self, error: impl Display) {
        self.results.clear();
        self.selected_index = None;
        self.status = error.to_string();
        self.has_error = true;
    }
}

pub fn portable_database_path(executable: &Path) -> Result<PathBuf, AppError> {
    let directory = executable
        .parent()
        .ok_or_else(|| AppError("executable path has no parent directory".into()))?;
    Ok(directory.join("data").join("lingoes.db"))
}

pub fn launch_gui() -> Result<(), AppError> {
    let executable = std::env::current_exe()
        .map_err(|error| AppError(format!("cannot locate executable: {error}")))?;
    let database_path = portable_database_path(&executable)?;
    let application = Rc::new(RefCell::new(DesktopApp::open(database_path)?));
    let window = AppWindow::new().map_err(|error| AppError(error.to_string()))?;

    {
        let application = Rc::clone(&application);
        let window_weak = window.as_weak();
        window.on_search(move |query| {
            application.borrow_mut().search(query.as_str());
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.borrow());
            }
        });
    }
    {
        let application = Rc::clone(&application);
        let window_weak = window.as_weak();
        window.on_select_result(move |index| {
            if index >= 0 {
                application.borrow_mut().select(index as usize);
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.borrow());
            }
        });
    }
    {
        let application = Rc::clone(&application);
        let window_weak = window.as_weak();
        window.on_import_dictionary(move |path| {
            let path = path.trim();
            if path.is_empty() {
                application
                    .borrow_mut()
                    .set_error("Choose or enter a TSV file path");
            } else {
                let _ = application.borrow_mut().import_path(path);
            }
            if let Some(window) = window_weak.upgrade() {
                update_window(&window, &application.borrow());
            }
        });
    }

    update_window(&window, &application.borrow());
    window.run().map_err(|error| AppError(error.to_string()))
}

fn update_window(window: &AppWindow, application: &DesktopApp) {
    let rows: Vec<SearchResult> = application
        .results()
        .iter()
        .map(|entry| SearchResult {
            headword: SharedString::from(entry.headword()),
            definition: SharedString::from(entry.definition()),
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
    window.set_status_text(application.status().into());
    window.set_status_is_error(application.has_error());
}

pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<String, AppError> {
    let arguments: Vec<_> = arguments.into_iter().collect();
    match arguments.as_slice() {
        [command, database_path, source_path] if command == "import" => {
            let source = File::open(source_path)
                .map_err(|error| AppError(format!("cannot open {source_path}: {error}")))?;
            let mut dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            let report = dictionary
                .import_tsv(source)
                .map_err(|error| AppError(error.to_string()))?;
            Ok(format!("Imported {} entries", report.imported))
        }
        [command, database_path, query] if command == "lookup" => {
            let dictionary = SqliteDictionary::open(database_path)
                .map_err(|error| AppError(error.to_string()))?;
            let query = LookupQuery::new(query).map_err(|_| AppError("query is empty".into()))?;
            let entries = dictionary
                .lookup_prefix(&query, 20)
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
        [] => Err(AppError(USAGE.into())),
        [command, ..] => Err(AppError(format!(
            "unknown command or invalid arguments: {command}\n{USAGE}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(application.status(), "2 results");
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
        assert_eq!(application.status(), "Type a word to search");
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
    fn importing_a_tsv_reports_success_and_malformed_input_reports_an_error() {
        let database_path = temporary_path("import", "db");
        let valid_path = temporary_path("valid", "tsv");
        let invalid_path = temporary_path("invalid", "tsv");
        std::fs::write(&valid_path, "hello\tXin chao\n").unwrap();
        std::fs::write(&invalid_path, "missing tab\n").unwrap();
        let mut application = DesktopApp::open(&database_path).unwrap();

        application.import_path(&valid_path).unwrap();
        assert_eq!(application.status(), "Imported 1 entry");
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
}
