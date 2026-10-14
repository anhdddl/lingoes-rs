use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::error::{TranslationError, redact_secret};
use crate::types::{LanguageCatalog, ProxyConfig, TranslationRequest, TranslationResult};
use crate::{BoxFuture, Translator};

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
    proxy: Option<ProxyConfig>,
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

    pub fn proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn build(self) -> OpenAiTranslator {
        let mut client_builder = reqwest::Client::builder();
        if let Some(proxy_cfg) = self.proxy {
            if let Ok(proxy) = proxy_cfg.to_reqwest_proxy() {
                client_builder = client_builder.proxy(proxy);
            }
        }
        let client = self
            .client
            .unwrap_or_else(|| client_builder.build().unwrap_or_default());

        OpenAiTranslator {
            base_url: self
                .base_url
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model: self.model.unwrap_or_else(|| "gpt-4o-mini".into()),
            api_key: self.api_key,
            client,
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

    async fn execute_attempt(
        &self,
        url: &str,
        req: &TranslationRequest,
        effective_to: &str,
    ) -> Result<TranslationResult, TranslationError> {
        let mut request = self
            .client
            .post(url)
            .header("Content-Type", "application/json");

        if let Some(token) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let system_prompt = format!(
            "You are a professional translator. Translate the text from {} to {effective_to}. Output only the translation without any explanation or extra text.",
            req.from
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
                    "content": req.text
                }
            ]
        });

        let response = request.json(&body).send().await.map_err(|e| {
            let msg = if let Some(key) = &self.api_key {
                redact_secret(&e.to_string(), key)
            } else {
                e.to_string()
            };
            if e.is_connect() {
                TranslationError::Offline(msg)
            } else {
                TranslationError::Other(msg)
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .map(Duration::from_secs);
            let raw_msg = response.text().await.unwrap_or_default();
            let sanitized_msg = if let Some(key) = &self.api_key {
                redact_secret(&raw_msg, key)
            } else {
                raw_msg
            };

            if status_code == 401 || status_code == 403 {
                return Err(TranslationError::AuthFailed {
                    status: status_code,
                    provider: format!("OpenAI ({})", self.model),
                });
            }
            if status_code == 429 {
                return Err(TranslationError::RateLimited { retry_after });
            }
            if status_code >= 500 {
                return Err(TranslationError::ServerUnavailable {
                    status: status_code,
                    message: sanitized_msg,
                });
            }
            return Err(TranslationError::Api {
                status: status_code,
                message: sanitized_msg,
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

        let bytes = response.bytes().await.map_err(|e| {
            let msg = if let Some(key) = &self.api_key {
                redact_secret(&e.to_string(), key)
            } else {
                e.to_string()
            };
            TranslationError::Other(msg)
        })?;

        if bytes.len() > req.max_response_bytes {
            return Err(TranslationError::ResponseTooLarge {
                limit: req.max_response_bytes,
                actual: Some(bytes.len()),
            });
        }

        let json: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| TranslationError::InvalidResponse(e.to_string()))?;

        let translation = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                TranslationError::InvalidResponse("missing choices[0].message.content".into())
            })?;

        Ok(TranslationResult {
            text: translation.trim().to_owned(),
            provider: format!("OpenAI ({})", self.model),
            source_lang: req.from.clone(),
            target_lang: effective_to.to_string(),
            detected_source: None,
            model: Some(self.model.clone()),
        })
    }
}

impl Translator for OpenAiTranslator {
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
            let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

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
