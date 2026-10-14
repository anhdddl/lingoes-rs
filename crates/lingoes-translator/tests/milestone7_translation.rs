use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use lingoes_translator::{
    GoogleTranslator, LanguageCatalog, OpenAiTranslator, ProxyConfig, RetryPolicy,
    TranslationError, TranslationRequest, Translator,
};

#[tokio::test]
async fn test_cancellation_cancels_in_flight_translation() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"[[["Xin chào","Hello",null,null,1]],null,"en"]"#)
                .set_delay(Duration::from_millis(500)),
        )
        .mount(&server)
        .await;

    let translator = GoogleTranslator::with_base_url(server.uri());
    let cancel = Arc::new(AtomicBool::new(false));

    let cancel_clone = Arc::clone(&cancel);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel_clone.store(true, Ordering::SeqCst);
    });

    let mut req = TranslationRequest::new("Hello", "en", "vi");
    req.cancellation_token = Some(cancel);
    req.timeout = Duration::from_secs(5);

    let err = translator.translate(req).await.unwrap_err();
    assert!(
        matches!(err, TranslationError::Cancelled),
        "Expected Cancelled, got {err:?}"
    );
}

#[tokio::test]
async fn test_timeout_triggers_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"[[["Xin chào","Hello",null,null,1]],null,"en"]"#)
                .set_delay(Duration::from_millis(500)),
        )
        .mount(&server)
        .await;

    let translator = GoogleTranslator::with_base_url(server.uri());
    let mut req = TranslationRequest::new("Hello", "en", "vi");
    req.timeout = Duration::from_millis(80);

    let err = translator.translate(req).await.unwrap_err();
    assert!(
        matches!(err, TranslationError::Timeout(_)),
        "Expected Timeout, got {err:?}"
    );
}

#[tokio::test]
async fn test_response_too_large_rejected() {
    let server = MockServer::start().await;
    let big_payload = "x".repeat(2048);
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(ResponseTemplate::new(200).set_body_string(big_payload))
        .mount(&server)
        .await;

    let translator = GoogleTranslator::with_base_url(server.uri());
    let mut req = TranslationRequest::new("Hello", "en", "vi");
    req.max_response_bytes = 256;

    let err = translator.translate(req).await.unwrap_err();
    assert!(
        matches!(err, TranslationError::ResponseTooLarge { limit: 256, .. }),
        "Expected ResponseTooLarge, got {err:?}"
    );
}

#[tokio::test]
async fn test_auth_failure_401_redacts_secret() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_string(r#"{"error":{"message":"Invalid secret-api-key-12345"}}"#),
        )
        .mount(&server)
        .await;

    let secret_key = "secret-api-key-12345";
    let translator = OpenAiTranslator::builder()
        .base_url(server.uri())
        .model("gpt-4o-mini")
        .api_key(secret_key)
        .build();

    let req = TranslationRequest::new("Hello", "en", "vi");
    let err = translator.translate(req).await.unwrap_err();

    let err_str = err.to_string();
    assert!(
        !err_str.contains(secret_key),
        "Error message leaked secret key: {err_str}"
    );
    assert!(
        matches!(err, TranslationError::AuthFailed { status: 401, .. }),
        "Expected AuthFailed, got {err:?}"
    );
}

#[tokio::test]
async fn test_rate_limited_429_returns_rate_limited_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "10")
                .set_body_string("Quota exceeded"),
        )
        .mount(&server)
        .await;

    let translator = OpenAiTranslator::builder()
        .base_url(server.uri())
        .model("gpt-4o-mini")
        .build();

    let req = TranslationRequest::new("Hello", "en", "vi");
    let err = translator.translate(req).await.unwrap_err();
    assert!(
        matches!(err, TranslationError::RateLimited { .. }),
        "Expected RateLimited, got {err:?}"
    );
}

#[tokio::test]
async fn test_server_503_retries_transient_failure_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"[[["Xin chào","Hello",null,null,1]],null,"en"]"#),
        )
        .mount(&server)
        .await;

    let translator = GoogleTranslator::with_base_url(server.uri());
    let mut req = TranslationRequest::new("Hello", "en", "vi");
    req.retry_policy = RetryPolicy {
        max_retries: 2,
        initial_backoff: Duration::from_millis(20),
        max_backoff: Duration::from_millis(50),
    };

    let result = translator.translate(req).await.unwrap();
    assert_eq!(result.text, "Xin chào");
    assert_eq!(result.provider, "Google Translate");
}

#[tokio::test]
async fn test_client_400_does_not_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/translate_a/single"))
        .respond_with(ResponseTemplate::new(400).set_body_string("Bad Request"))
        .expect(1)
        .mount(&server)
        .await;

    let translator = GoogleTranslator::with_base_url(server.uri());
    let mut req = TranslationRequest::new("Hello", "en", "vi");
    req.retry_policy = RetryPolicy {
        max_retries: 3,
        initial_backoff: Duration::from_millis(10),
        max_backoff: Duration::from_millis(50),
    };

    let err = translator.translate(req).await.unwrap_err();
    assert!(
        matches!(err, TranslationError::Api { status: 400, .. }),
        "Expected Api 400, got {err:?}"
    );
}

#[tokio::test]
async fn test_proxy_config_redacts_credentials_in_debug() {
    let proxy = ProxyConfig {
        url: "http://proxy.example.com:8080".into(),
        username: Some("my_secret_user".into()),
        password: Some("super_secret_password".into()),
    };

    let debug_str = format!("{proxy:?}");
    assert!(
        !debug_str.contains("super_secret_password"),
        "Proxy debug leaked password: {debug_str}"
    );
    assert!(debug_str.contains("[REDACTED]"));
}

#[tokio::test]
async fn test_language_fallback_when_source_equals_target() {
    let resolved = LanguageCatalog::resolve_target_lang("en", "en", Some("vi"));
    assert_eq!(resolved, "vi");

    let resolved2 = LanguageCatalog::resolve_target_lang("vi", "vi", Some("en"));
    assert_eq!(resolved2, "en");

    let resolved3 = LanguageCatalog::resolve_target_lang("en", "fr", Some("vi"));
    assert_eq!(resolved3, "fr");
}

#[tokio::test]
async fn test_provider_attribution_preserved() {
    let server = MockServer::start().await;
    let mock_response = serde_json::json!({
        "choices": [
            {
                "message": {
                    "role": "assistant",
                    "content": "Xin chào"
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
        .build();

    let req = TranslationRequest::new("Hello", "en", "vi");
    let result = translator.translate(req).await.unwrap();

    assert_eq!(result.text, "Xin chào");
    assert!(result.provider.contains("OpenAI"));
    assert_eq!(result.source_lang, "en");
    assert_eq!(result.target_lang, "vi");
    assert_eq!(result.model.as_deref(), Some("gpt-4o-mini"));
}
