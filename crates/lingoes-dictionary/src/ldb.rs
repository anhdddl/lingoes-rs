use std::io::Read;
use std::path::Path;

use crate::ImportError;
use crate::ld2::Ld2Dictionary;

pub struct LdbArchive;

impl LdbArchive {
    pub fn extract_and_parse(path: impl AsRef<Path>) -> Result<Vec<Ld2Dictionary>, ImportError> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ImportError::Format(format!("invalid ldb zip: {e}")))?;

        if archive.len() > 1000 {
            return Err(ImportError::Format("ldb contains too many files".into()));
        }

        let mut dicts = Vec::new();
        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|e| ImportError::Format(format!("invalid zip entry: {e}")))?;
            let name = file.name().to_string();

            if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
                return Err(ImportError::Format(format!(
                    "path traversal detected in ldb: {name}"
                )));
            }

            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".ld2") || lower.ends_with(".ldx") {
                let mut bytes = Vec::new();
                let mut reader = file.take(256 * 1024 * 1024);
                reader.read_to_end(&mut bytes)?;
                let dict = Ld2Dictionary::from_bytes(&bytes, Some(&name))
                    .map_err(|e| ImportError::Format(e.to_string()))?;
                dicts.push(dict);
            }
        }

        if dicts.is_empty() {
            return Err(ImportError::Format(
                "no supported dictionaries (.ld2/.ldx) found in ldb archive".into(),
            ));
        }

        Ok(dicts)
    }
}
