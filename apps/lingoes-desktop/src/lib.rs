use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;

use lingoes_core::LookupQuery;
use lingoes_dictionary::SqliteDictionary;

const USAGE: &str = "Usage:\n  lingoes-desktop import <dictionary.db> <entries.tsv>\n  lingoes-desktop lookup <dictionary.db> <query>";

#[derive(Debug)]
pub struct AppError(String);

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for AppError {}

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
}
