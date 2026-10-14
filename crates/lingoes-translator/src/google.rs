use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::error::TranslationError;
use crate::types::{LanguageCatalog, TranslationRequest, TranslationResult};
use crate::{BoxFuture, Translator};

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

    async fn execute_attempt(
        &self,
        url: &str,
        req: &TranslationRequest,
        effective_to: &str,
    ) -> Result<TranslationResult, TranslationError> {
        let request = self.client.get(url).query(&[
            ("client", "gtx"),
            ("sl", req.from.as_str()),
            ("tl", effective_to),
            ("dt", "t"),
            ("q", req.text.as_str()),
        ]);

        let response = request.send().await.map_err(|e| {
            if e.is_connect() {
                TranslationError::Offline(e.to_string())
            } else {
                TranslationError::Http(e)
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            if status_code == 401 || status_code == 403 {
                return Err(TranslationError::AuthFailed {
                    status: status_code,
                    provider: "Google Translate".into(),
                });
            }
            if status_code == 429 {
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(Duration::from_secs);
                return Err(TranslationError::RateLimited { retry_after });
            }
            let message = response.text().await.unwrap_or_default();
            if status_code >= 500 {
                return Err(TranslationError::ServerUnavailable {
                    status: status_code,
                    message,
                });
            }
            return Err(TranslationError::Api {
                status: status_code,
                message,
            });
        }

        if let Some(content_len) = response.content_length() {
            if content_len as usize > req.max_response_bytes {
                return Err(TranslationError::ResponseTooLarge {
                    limit: req.max_response_bytes,
                    actual: Some(content_len as usize),
                });
            }
        }

        let bytes = response.bytes().await.map_err(TranslationError::Http)?;
        if bytes.len() > req.max_response_bytes {
            return Err(TranslationError::ResponseTooLarge {
                limit: req.max_response_bytes,
                actual: Some(bytes.len()),
            });
        }

        let json: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| TranslationError::InvalidResponse(e.to_string()))?;

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

        let detected_source = json
            .get(2)
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        Ok(TranslationResult {
            text: translated,
            provider: "Google Translate".into(),
            source_lang: req.from.clone(),
            target_lang: effective_to.to_string(),
            detected_source,
            model: None,
        })
    }
}

impl Default for GoogleTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl Translator for GoogleTranslator {
    fn translate<'a>(
        &'a self,
        req: TranslationRequest,
    ) -> BoxFuture<'a, Result<TranslationResult, TranslationError>> {
        Box::pin(async move {
            let trimmed = req.text.trim();
            if trimmed.is_empty() {
                return Err(TranslationError::EmptyText);
            }

            let effective_to = LanguageCatalog::resolve_target_lang(
                &req.from,
                &req.to,
                req.fallback_to.as_deref(),
            );
            let url = format!("{}/translate_a/single", self.base_url.trim_end_matches('/'));

            let mut attempt = 0;
            loop {
                if let Some(cancel) = &req.cancellation_token {
                    if cancel.load(Ordering::SeqCst) {
                        return Err(TranslationError::Cancelled);
                    }
                }

                let cancel_future = {
                    let cancel_tok = req.cancellation_token.clone();
                    async move {
                        if let Some(token) = cancel_tok {
                            while !token.load(Ordering::SeqCst) {
                                tokio::time::sleep(Duration::from_millis(10)).await;
                            }
                        } else {
                            std::future::pending::<()>().await;
                        }
                    }
                };

                let attempt_future = self.execute_attempt(&url, &req, &effective_to);

                let res = tokio::select! {
                    biased;
                    _ = cancel_future => {
                        return Err(TranslationError::Cancelled);
                    }
                    timed_res = tokio::time::timeout(req.timeout, attempt_future) => {
                        match timed_res {
                            Ok(inner) => inner,
                            Err(_) => Err(TranslationError::Timeout(req.timeout)),
                        }
                    }
                };

                match res {
                    Ok(val) => return Ok(val),
                    Err(err) => {
                        if req.retry_policy.should_retry(&err, attempt) {
                            attempt += 1;
                            let backoff = (req.retry_policy.initial_backoff * attempt)
                                .min(req.retry_policy.max_backoff);
                            tokio::time::sleep(backoff).await;
                            continue;
                        }
                        return Err(err);
                    }
                }
            }
        })
    }
}
