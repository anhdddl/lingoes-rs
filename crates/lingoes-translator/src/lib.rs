use std::future::Future;
use std::pin::Pin;

pub mod error;
pub mod google;
pub mod mock;
pub mod openai;
pub mod types;

pub use error::{TranslationError, redact_secret};
pub use google::GoogleTranslator;
pub use mock::MockTranslator;
pub use openai::{OpenAiTranslator, OpenAiTranslatorBuilder};
pub use types::{LanguageCatalog, ProxyConfig, RetryPolicy, TranslationRequest, TranslationResult};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Translator: Send + Sync {
    fn translate<'a>(
        &'a self,
        req: TranslationRequest,
    ) -> BoxFuture<'a, Result<TranslationResult, TranslationError>>;

    fn translate_text<'a>(
        &'a self,
        text: &'a str,
        from: &'a str,
        to: &'a str,
    ) -> BoxFuture<'a, Result<String, TranslationError>> {
        Box::pin(async move {
            let req = TranslationRequest::new(text, from, to);
            let res = self.translate(req).await?;
            Ok(res.text)
        })
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

        let res1 = mock.translate_text("Hello", "en", "vi").await.unwrap();
        assert_eq!(res1, "Xin chào");

        let res2 = mock.translate_text("Goodbye", "en", "vi").await.unwrap();
        assert_eq!(res2, "[en->vi] Goodbye");
    }

    #[tokio::test]
    async fn mock_translator_error_simulation() {
        let mut mock = MockTranslator::new();
        mock.set_error(Some("Service unavailable".into()));

        let err = mock.translate_text("Hello", "en", "vi").await.unwrap_err();
        match err {
            TranslationError::Other(msg) => assert_eq!(msg, "Service unavailable"),
            _ => panic!("Expected Other error, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn mock_translator_records_history() {
        let mock = MockTranslator::new();
        mock.translate_text("One", "en", "fr").await.unwrap();
        mock.translate_text("Two", "en", "de").await.unwrap();

        let history = mock.recorded_requests();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0], ("One".into(), "en".into(), "fr".into()));
        assert_eq!(history[1], ("Two".into(), "en".into(), "de".into()));
    }

    #[tokio::test]
    async fn mock_translator_rejects_empty_text() {
        let mock = MockTranslator::new();
        let err = mock.translate_text("   ", "en", "vi").await.unwrap_err();
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
            .translate_text("Hello world", "en", "vi")
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
            .translate_text("Sentence one. Sentence two.", "en", "vi")
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
        let mut req = TranslationRequest::new("Test", "en", "vi");
        req.retry_policy = RetryPolicy::none();

        let err = translator.translate(req).await.unwrap_err();
        match err {
            TranslationError::ServerUnavailable { status, message } => {
                assert_eq!(status, 503);
                assert_eq!(message, "Backend unavailable");
            }
            _ => panic!("Expected ServerUnavailable error, got {err:?}"),
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
            .translate_text("Hello world", "en", "vi")
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

        let req = TranslationRequest::new("Hello world", "en", "vi");
        let err = translator.translate(req).await.unwrap_err();
        match err {
            TranslationError::AuthFailed { status, provider } => {
                assert_eq!(status, 401);
                assert!(provider.contains("OpenAI"));
            }
            _ => panic!("Expected AuthFailed error, got {err:?}"),
        }
    }
}
