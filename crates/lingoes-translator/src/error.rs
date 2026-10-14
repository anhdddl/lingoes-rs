use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranslationError {
    #[error("empty input text")]
    EmptyText,

    #[error("translation request was cancelled")]
    Cancelled,

    #[error("translation timed out after {0:?}")]
    Timeout(Duration),

    #[error("response too large: exceeds limit of {limit} bytes (received {actual:?})")]
    ResponseTooLarge { limit: usize, actual: Option<usize> },

    #[error("authentication failed with provider {provider} (status {status})")]
    AuthFailed { status: u16, provider: String },

    #[error("rate limit exceeded (retry after {retry_after:?})")]
    RateLimited { retry_after: Option<Duration> },

    #[error("translation server unavailable (status {status}): {message}")]
    ServerUnavailable { status: u16, message: String },

    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("api returned error (status {status}): {message}")]
    Api { status: u16, message: String },

    #[error("failed to parse response: {0}")]
    InvalidResponse(String),

    #[error("proxy configuration error: {0}")]
    Proxy(String),

    #[error("network offline or host unreachable: {0}")]
    Offline(String),

    #[error("translation error: {0}")]
    Other(String),
}

pub fn redact_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        return text.to_string();
    }
    text.replace(secret, "[REDACTED]")
}
