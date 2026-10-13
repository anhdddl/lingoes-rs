use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("api returned error (status {status}): {message}")]
    Api { status: u16, message: String },
    #[error("failed to parse response: {0}")]
    InvalidResponse(String),
    #[error("empty input text")]
    EmptyText,
    #[error("translation error: {0}")]
    Other(String),
}

pub trait Translator: Send + Sync {
    fn translate(
        &self,
        text: &str,
        from: &str,
        to: &str,
    ) -> impl std::future::Future<Output = Result<String, TranslationError>> + Send;
}

#[derive(Debug, Default, Clone)]
pub struct MockTranslator {
    canned: HashMap<(String, String, String), String>,
    simulated_error: Option<String>,
    history: Arc<Mutex<Vec<(String, String, String)>>>,
}

impl MockTranslator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_translation(
        &mut self,
        text: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        translation: impl Into<String>,
    ) {
        self.canned
            .insert((text.into(), from.into(), to.into()), translation.into());
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.simulated_error = error;
    }

    pub fn recorded_requests(&self) -> Vec<(String, String, String)> {
        self.history.lock().unwrap().clone()
    }
}

impl Translator for MockTranslator {
    async fn translate(
        &self,
        text: &str,
        from: &str,
        to: &str,
    ) -> Result<String, TranslationError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TranslationError::EmptyText);
        }
        self.history
            .lock()
            .unwrap()
            .push((text.to_owned(), from.to_owned(), to.to_owned()));

        if let Some(err) = &self.simulated_error {
            return Err(TranslationError::Other(err.clone()));
        }

        if let Some(translation) =
            self.canned
                .get(&(text.to_owned(), from.to_owned(), to.to_owned()))
        {
            return Ok(translation.clone());
        }

        Ok(format!("[{from}->{to}] {text}"))
    }
}

#[derive(Clone)]
pub struct GoogleTranslator {
    base_url: String,
    client: reqwest::Client,
}

impl GoogleTranslator {
    pub fn new() -> Self {
        Self::with_base_url("https://translate.googleapis.com")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client_and_base_url(client: reqwest::Client, base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client,
        }
    }
}

impl Default for GoogleTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator for GoogleTranslator {
    async fn translate(
        &self,
        text: &str,
        from: &str,
        to: &str,
    ) -> Result<String, TranslationError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TranslationError::EmptyText);
        }

        let url = format!("{}/translate_a/single", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .get(&url)
            .query(&[
                ("client", "gtx"),
                ("sl", from),
                ("tl", to),
                ("dt", "t"),
                ("q", text),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(TranslationError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let json: serde_json::Value = response.json().await?;
        let segments = json.get(0).and_then(|v| v.as_array()).ok_or_else(|| {
            TranslationError::InvalidResponse("missing root segment array".into())
        })?;

        let mut translated = String::new();
        for segment in segments {
            if let Some(text_piece) = segment.get(0).and_then(|v| v.as_str()) {
                translated.push_str(text_piece);
            }
        }

        if translated.is_empty() {
            return Err(TranslationError::InvalidResponse(
                "no translated fragments in response".into(),
            ));
        }

        Ok(translated)
    }
}

#[derive(Clone)]
pub struct OpenAiTranslator {
    base_url: String,
    model: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Default)]
pub struct OpenAiTranslatorBuilder {
    base_url: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    client: Option<reqwest::Client>,
}

impl OpenAiTranslatorBuilder {
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    pub fn build(self) -> OpenAiTranslator {
        OpenAiTranslator {
            base_url: self
                .base_url
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model: self.model.unwrap_or_else(|| "gpt-4o-mini".into()),
            api_key: self.api_key,
            client: self.client.unwrap_or_default(),
        }
    }
}

impl OpenAiTranslator {
    pub fn builder() -> OpenAiTranslatorBuilder {
        OpenAiTranslatorBuilder::default()
    }

    pub fn new(model: impl Into<String>) -> Self {
        Self::builder().model(model).build()
    }
}

impl Translator for OpenAiTranslator {
    async fn translate(
        &self,
        text: &str,
        from: &str,
        to: &str,
    ) -> Result<String, TranslationError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TranslationError::EmptyText);
        }

        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        if let Some(token) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let system_prompt = format!(
            "You are a professional translator. Translate the text from {from} to {to}. Output only the translation without any explanation or extra text."
        );

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": text
                }
            ]
        });

        let response = request.json(&body).send().await?;
        let status = response.status();
        if !status.is_success() {
            let message = response.text().await.unwrap_or_default();
            return Err(TranslationError::Api {
                status: status.as_u16(),
                message,
            });
        }

        let json: serde_json::Value = response.json().await?;
        let translation = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                TranslationError::InvalidResponse("missing choices[0].message.content".into())
            })?;

        Ok(translation.trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn mock_translator_default_and_canned() {
        let mut mock = MockTranslator::new();
        mock.add_translation("Hello", "en", "vi", "Xin chào");

        let res1 = mock.translate("Hello", "en", "vi").await.unwrap();
        assert_eq!(res1, "Xin chào");

        let res2 = mock.translate("Goodbye", "en", "vi").await.unwrap();
        assert_eq!(res2, "[en->vi] Goodbye");
    }

    #[tokio::test]
    async fn mock_translator_error_simulation() {
        let mut mock = MockTranslator::new();
        mock.set_error(Some("Service unavailable".into()));

        let err = mock.translate("Hello", "en", "vi").await.unwrap_err();
        match err {
            TranslationError::Other(msg) => assert_eq!(msg, "Service unavailable"),
            _ => panic!("Expected Other error"),
        }
    }

    #[tokio::test]
    async fn mock_translator_records_history() {
        let mock = MockTranslator::new();
        mock.translate("One", "en", "fr").await.unwrap();
        mock.translate("Two", "en", "de").await.unwrap();

        let history = mock.recorded_requests();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], ("One".into(), "en".into(), "fr".into()));
        assert_eq!(history[1], ("Two".into(), "en".into(), "de".into()));
    }

    #[tokio::test]
    async fn mock_translator_rejects_empty_text() {
        let mock = MockTranslator::new();
        let err = mock.translate("   ", "en", "vi").await.unwrap_err();
        assert!(matches!(err, TranslationError::EmptyText));
    }

    #[tokio::test]
    async fn google_translator_success_with_wiremock() {
        let server = MockServer::start().await;

        let mock_body = r#"[[["Xin chào thế giới","Hello world",null,null,10]],null,"en"]"#;
        Mock::given(method("GET"))
            .and(path("/translate_a/single"))
            .and(query_param("client", "gtx"))
            .and(query_param("sl", "en"))
            .and(query_param("tl", "vi"))
            .and(query_param("dt", "t"))
            .and(query_param("q", "Hello world"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(mock_body, "application/json"))
            .mount(&server)
            .await;

        let translator = GoogleTranslator::with_base_url(server.uri());
        let result = translator
            .translate("Hello world", "en", "vi")
            .await
            .unwrap();
        assert_eq!(result, "Xin chào thế giới");
    }

    #[tokio::test]
    async fn google_translator_multi_sentence_wiremock() {
        let server = MockServer::start().await;

        let mock_body = r#"[[["Câu một. ","Sentence one. ",null,null,1],["Câu hai.","Sentence two.",null,null,1]],null,"en"]"#;
        Mock::given(method("GET"))
            .and(path("/translate_a/single"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(mock_body, "application/json"))
            .mount(&server)
            .await;

        let translator = GoogleTranslator::with_base_url(server.uri());
        let result = translator
            .translate("Sentence one. Sentence two.", "en", "vi")
            .await
            .unwrap();
        assert_eq!(result, "Câu một. Câu hai.");
    }

    #[tokio::test]
    async fn google_translator_api_error_wiremock() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/translate_a/single"))
            .respond_with(ResponseTemplate::new(503).set_body_string("Backend unavailable"))
            .mount(&server)
            .await;

        let translator = GoogleTranslator::with_base_url(server.uri());
        let err = translator.translate("Test", "en", "vi").await.unwrap_err();
        match err {
            TranslationError::Api { status, message } => {
                assert_eq!(status, 503);
                assert_eq!(message, "Backend unavailable");
            }
            _ => panic!("Expected Api error"),
        }
    }

    #[tokio::test]
    async fn openai_translator_success_with_wiremock() {
        let server = MockServer::start().await;

        let mock_response = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Xin chào thế giới"
                    }
                }
            ]
        });

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_response))
            .mount(&server)
            .await;

        let translator = OpenAiTranslator::builder()
            .base_url(server.uri())
            .model("gpt-4o-mini")
            .api_key("test-token")
            .build();

        let result = translator
            .translate("Hello world", "en", "vi")
            .await
            .unwrap();
        assert_eq!(result, "Xin chào thế giới");
    }

    #[tokio::test]
    async fn openai_translator_api_error_wiremock() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid API key"))
            .mount(&server)
            .await;

        let translator = OpenAiTranslator::builder()
            .base_url(server.uri())
            .model("gpt-4o-mini")
            .build();

        let err = translator
            .translate("Hello world", "en", "vi")
            .await
            .unwrap_err();
        match err {
            TranslationError::Api { status, message } => {
                assert_eq!(status, 401);
                assert_eq!(message, "Invalid API key");
            }
            _ => panic!("Expected Api error"),
        }
    }
}
