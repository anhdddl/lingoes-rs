use serde::{Deserialize, Serialize};

/// Comprehensive metadata for dictionary archives and packages (DICT-06).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DictionaryMetadata {
    pub id: Option<i64>,
    pub title: String,
    pub author: Option<String>,
    pub publisher: Option<String>,
    pub website: Option<String>,
    pub copyright: Option<String>,
    pub description: Option<String>,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
    pub format: Option<String>,
    pub entry_count: Option<usize>,
}

impl DictionaryMetadata {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: None,
            title: title.into(),
            author: None,
            publisher: None,
            website: None,
            copyright: None,
            description: None,
            source_lang: None,
            target_lang: None,
            format: None,
            entry_count: None,
        }
    }
}
