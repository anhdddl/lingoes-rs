#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DictionaryEntry {
    headword: String,
    normalized_headword: String,
    definition: String,
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
        Ok(Self {
            headword: headword.to_owned(),
            normalized_headword: headword.to_lowercase(),
            definition: definition.to_owned(),
        })
    }

    pub fn headword(&self) -> &str {
        &self.headword
    }

    pub fn normalized_headword(&self) -> &str {
        &self.normalized_headword
    }

    pub fn definition(&self) -> &str {
        &self.definition
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LookupQuery(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    Empty,
}

impl LookupQuery {
    pub fn new(query: impl AsRef<str>) -> Result<Self, QueryError> {
        let query = query.as_ref().trim();
        if query.is_empty() {
            return Err(QueryError::Empty);
        }
        Ok(Self(query.to_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
}
