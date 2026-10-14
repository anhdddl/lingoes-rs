use std::fmt;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::error::TranslationError;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_millis(1000),
        }
    }
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self {
            max_retries: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        }
    }

    pub fn should_retry(&self, err: &TranslationError, attempt: u32) -> bool {
        if attempt >= self.max_retries {
            return false;
        }
        match err {
            TranslationError::ServerUnavailable { .. } => true,
            TranslationError::Http(_) => true,
            TranslationError::Api { status, .. } if *status >= 500 => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct TranslationRequest {
    pub text: String,
    pub from: String,
    pub to: String,
    pub fallback_to: Option<String>,
    pub timeout: Duration,
    pub max_response_bytes: usize,
    pub retry_policy: RetryPolicy,
    pub cancellation_token: Option<Arc<AtomicBool>>,
}

impl TranslationRequest {
    pub fn new(text: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            from: from.into(),
            to: to.into(),
            fallback_to: None,
            timeout: Duration::from_secs(15),
            max_response_bytes: 1024 * 1024,
            retry_policy: RetryPolicy::default(),
            cancellation_token: None,
        }
    }

    pub fn with_fallback(mut self, fallback: impl Into<String>) -> Self {
        self.fallback_to = Some(fallback.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_cancellation(mut self, token: Arc<AtomicBool>) -> Self {
        self.cancellation_token = Some(token);
        self
    }
}

impl From<(&str, &str, &str)> for TranslationRequest {
    fn from((text, from, to): (&str, &str, &str)) -> Self {
        Self::new(text, from, to)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslationResult {
    pub text: String,
    pub provider: String,
    pub source_lang: String,
    pub target_lang: String,
    pub detected_source: Option<String>,
    pub model: Option<String>,
}

#[derive(Clone)]
pub struct ProxyConfig {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("url", &self.url)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

impl ProxyConfig {
    pub fn to_reqwest_proxy(&self) -> Result<reqwest::Proxy, TranslationError> {
        let mut proxy =
            reqwest::Proxy::all(&self.url).map_err(|e| TranslationError::Proxy(e.to_string()))?;
        if let (Some(u), Some(p)) = (&self.username, &self.password) {
            proxy = proxy.basic_auth(u, p);
        }
        Ok(proxy)
    }
}

pub struct LanguageCatalog;

impl LanguageCatalog {
    pub fn resolve_target_lang(from: &str, to: &str, fallback_to: Option<&str>) -> String {
        let trimmed_from = from.trim();
        let trimmed_to = to.trim();
        if trimmed_from.eq_ignore_ascii_case(trimmed_to)
            && !trimmed_from.eq_ignore_ascii_case("auto")
        {
            if let Some(fb) = fallback_to {
                let trimmed_fb = fb.trim();
                if !trimmed_fb.is_empty() && !trimmed_fb.eq_ignore_ascii_case(trimmed_from) {
                    return trimmed_fb.to_string();
                }
            }
        }
        trimmed_to.to_string()
    }
}
