use std::io::Read;
use std::path::Path;

use crate::ImportError;
use crate::ld2::Ld2Dictionary;

pub struct LdpPackage;

impl LdpPackage {
    pub fn extract_and_parse(path: impl AsRef<Path>) -> Result<Ld2Dictionary, ImportError> {
        let file = std::fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| ImportError::Format(format!("invalid ldp zip: {e}")))?;

        // Enforce maximum entries to guard against zip bombs
        if archive.len() > 1000 {
            return Err(ImportError::Format("ldp contains too many files".into()));
        }

        // Look for .ld2 or .ldx file first
        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|e| ImportError::Format(format!("invalid zip entry: {e}")))?;
            let name = file.name().to_string();

            // Guard against path traversal
            if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
                return Err(ImportError::Format(format!(
                    "path traversal detected in ldp: {name}"
                )));
            }

            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".ld2") || lower.ends_with(".ldx") {
                let mut bytes = Vec::new();
                let mut reader = file.take(256 * 1024 * 1024);
                reader.read_to_end(&mut bytes)?;
                let dict = Ld2Dictionary::from_bytes(&bytes, Some(&name))
                    .map_err(|e| ImportError::Format(e.to_string()))?;
                return Ok(dict);
            }
        }

        // Fallback: look for xml definition files
        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|e| ImportError::Format(format!("invalid zip entry: {e}")))?;
            let name = file.name().to_string();
            if name.contains("..") || name.starts_with('/') || name.starts_with('\\') {
                return Err(ImportError::Format(format!(
                    "path traversal detected in ldp: {name}"
                )));
            }
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".xml") {
                let mut text = String::new();
                let mut reader = file.take(64 * 1024 * 1024);
                reader.read_to_string(&mut text)?;
                // Parse simple XML definitions if present
                if let Ok(dict) = parse_xml_definitions(&text, &name) {
                    return Ok(dict);
                }
            }
        }

        Err(ImportError::Format(
            "no valid .ld2, .ldx or xml definitions found inside ldp package".into(),
        ))
    }
}

/// Fallback XML definition parser for packages containing plain XML headwords/definitions.
fn parse_xml_definitions(xml: &str, name: &str) -> Result<Ld2Dictionary, ImportError> {
    // If the XML is empty or not containing entries, return format error
    let cleaned = crate::ld2::clean_lingoes_xml(xml);
    if cleaned.trim().is_empty() {
        return Err(ImportError::Format("empty xml definitions".into()));
    }
    // We can synthesize a single entry or extract <word> / <entry> elements if present
    let synthetic = crate::ld2::Ld2Dictionary::from_bytes(
        crate::ld2::clean_lingoes_xml(xml).as_bytes(),
        Some(name),
    );
    synthetic.map_err(|e| ImportError::Format(e.to_string()))
}
