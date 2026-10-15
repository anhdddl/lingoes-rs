use unicode_normalization::UnicodeNormalization;

pub mod appendices;
pub mod document;
pub mod metadata;

pub use appendices::*;
pub use document::{
    DefinitionSection, EntryBlock, FindMatch, FindResult, FormattedLine, LinkReference,
    PartOfSpeech, Phonetic, RichDocument, TextRange, escape_html, find_in_result,
    format_definition_for_display, parse_formatted_lines, parse_rich_document, sanitize_html,
    save_as_html, strip_tags,
};
pub use metadata::DictionaryMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryEntry {
    headword: String,
    normalized_headword: String,
    unaccented_headword: String,
    definition: String,
    dictionary_id: Option<i64>,
    dictionary_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryError {
    EmptyHeadword,
    EmptyDefinition,
}

impl DictionaryEntry {
    pub fn new(headword: impl AsRef<str>, definition: impl AsRef<str>) -> Result<Self, EntryError> {
        let headword = headword.as_ref().trim();
        let definition = definition.as_ref().trim();
        if headword.is_empty() {
            return Err(EntryError::EmptyHeadword);
        }
        if definition.is_empty() {
            return Err(EntryError::EmptyDefinition);
        }
        let normalized_headword = normalize_search_text(headword);
        let unaccented_headword = strip_vietnamese_diacritics(headword);
        Ok(Self {
            headword: headword.to_owned(),
            normalized_headword,
            unaccented_headword,
            definition: definition.to_owned(),
            dictionary_id: None,
            dictionary_name: None,
        })
    }

    pub fn with_dictionary(
        headword: impl AsRef<str>,
        definition: impl AsRef<str>,
        dictionary_id: i64,
        dictionary_name: impl Into<String>,
    ) -> Result<Self, EntryError> {
        let mut entry = Self::new(headword, definition)?;
        entry.dictionary_id = Some(dictionary_id);
        entry.dictionary_name = Some(dictionary_name.into());
        Ok(entry)
    }

    pub fn set_dictionary(&mut self, dictionary_id: i64, dictionary_name: impl Into<String>) {
        self.dictionary_id = Some(dictionary_id);
        self.dictionary_name = Some(dictionary_name.into());
    }

    pub fn headword(&self) -> &str {
        &self.headword
    }

    pub fn normalized_headword(&self) -> &str {
        &self.normalized_headword
    }

    pub fn unaccented_headword(&self) -> &str {
        &self.unaccented_headword
    }

    pub fn definition(&self) -> &str {
        &self.definition
    }

    pub fn dictionary_id(&self) -> Option<i64> {
        self.dictionary_id
    }

    pub fn dictionary_name(&self) -> Option<&str> {
        self.dictionary_name.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupQuery {
    normalized: String,
    unaccented: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    Empty,
}

pub fn normalize_search_text(value: &str) -> String {
    value.nfc().flat_map(char::to_lowercase).nfc().collect()
}

pub fn strip_vietnamese_diacritics(value: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    let mapped: String = value
        .chars()
        .map(|c| match c {
            'đ' | 'Đ' => 'd',
            other => other,
        })
        .collect();
    mapped
        .nfd()
        .filter(|c| !('\u{0300}'..='\u{036f}').contains(c))
        .flat_map(char::to_lowercase)
        .nfc()
        .collect()
}

impl LookupQuery {
    pub fn new(query: impl AsRef<str>) -> Result<Self, QueryError> {
        let query = query.as_ref().trim();
        if query.is_empty() {
            return Err(QueryError::Empty);
        }
        Ok(Self {
            normalized: normalize_search_text(query),
            unaccented: strip_vietnamese_diacritics(query),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    pub fn unaccented(&self) -> &str {
        &self.unaccented
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_rejects_blank_headword() {
        let error = DictionaryEntry::new("   ", "definition").unwrap_err();
        assert_eq!(error, EntryError::EmptyHeadword);
    }

    #[test]
    fn entry_rejects_blank_definition() {
        let error = DictionaryEntry::new("rust", "  ").unwrap_err();
        assert_eq!(error, EntryError::EmptyDefinition);
    }

    #[test]
    fn entry_trims_but_preserves_display_case() {
        let entry = DictionaryEntry::new("  Rust  ", " language ").unwrap();
        assert_eq!(entry.headword(), "Rust");
        assert_eq!(entry.normalized_headword(), "rust");
        assert_eq!(entry.definition(), "language");
    }

    #[test]
    fn query_is_trimmed_and_case_folded() {
        let query = LookupQuery::new("  HELLO  ").unwrap();
        assert_eq!(query.as_str(), "hello");
    }

    #[test]
    fn query_rejects_blank_input() {
        assert_eq!(LookupQuery::new("\t ").unwrap_err(), QueryError::Empty);
    }

    #[test]
    fn normalization_canonicalizes_unicode_and_case() {
        let composed = DictionaryEntry::new("TIẾNG VIỆT", "language").unwrap();
        let decomposed = LookupQuery::new("tie\u{302}\u{301}ng vie\u{323}\u{302}t").unwrap();
        assert_eq!(composed.normalized_headword(), decomposed.as_str());
    }

    #[test]
    fn normalization_handles_unicode_case_expansion() {
        let entry = DictionaryEntry::new("İSTANBUL", "city").unwrap();
        assert_eq!(entry.normalized_headword(), "i\u{307}stanbul");
    }

    #[test]
    fn strip_vietnamese_diacritics_handles_single_and_multi_word_phrases() {
        assert_eq!(strip_vietnamese_diacritics("chào"), "chao");
        assert_eq!(strip_vietnamese_diacritics("chào cờ"), "chao co");
        assert_eq!(strip_vietnamese_diacritics("cảm ơn"), "cam on");
        assert_eq!(strip_vietnamese_diacritics("tạm biệt"), "tam biet");
        assert_eq!(strip_vietnamese_diacritics("đường phố"), "duong pho");
        assert_eq!(
            strip_vietnamese_diacritics("anh chàng keo kiệt"),
            "anh chang keo kiet"
        );
        assert_eq!(
            strip_vietnamese_diacritics("Cộng Hòa Xã Hội Chủ Nghĩa Việt Nam"),
            "cong hoa xa hoi chu nghia viet nam"
        );
        let query = LookupQuery::new("Chào Cờ").unwrap();
        assert_eq!(query.as_str(), "chào cờ");
        assert_eq!(query.unaccented(), "chao co");
    }

    #[test]
    fn entry_supports_dictionary_attribution() {
        let entry = DictionaryEntry::with_dictionary("hello", "greeting", 42, "Oxford").unwrap();
        assert_eq!(entry.headword(), "hello");
        assert_eq!(entry.definition(), "greeting");
        assert_eq!(entry.dictionary_id(), Some(42));
        assert_eq!(entry.dictionary_name(), Some("Oxford"));

        let mut default_entry = DictionaryEntry::new("world", "earth").unwrap();
        assert_eq!(default_entry.dictionary_id(), None);
        assert_eq!(default_entry.dictionary_name(), None);

        default_entry.set_dictionary(7, "Cambridge");
        assert_eq!(default_entry.dictionary_id(), Some(7));
        assert_eq!(default_entry.dictionary_name(), Some("Cambridge"));
    }
}
