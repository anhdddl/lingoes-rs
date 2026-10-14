use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::TranslationError;
use crate::types::{LanguageCatalog, TranslationRequest, TranslationResult};
use crate::{BoxFuture, Translator};

#[derive(Debug, Default, Clone)]
pub struct MockTranslator {
    canned: HashMap<(String, String, String), String>,
    simulated_error: Option<String>,
    simulated_delay: Option<Duration>,
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

    pub fn set_delay(&mut self, delay: Option<Duration>) {
        self.simulated_delay = delay;
    }

    pub fn recorded_requests(&self) -> Vec<(String, String, String)> {
        self.history.lock().unwrap().clone()
    }
}

impl Translator for MockTranslator {
    fn translate<'a>(
        &'a self,
        req: TranslationRequest,
    ) -> BoxFuture<'a, Result<TranslationResult, TranslationError>> {
        Box::pin(async move {
            let trimmed = req.text.trim();
            if trimmed.is_empty() {
                return Err(TranslationError::EmptyText);
            }

            if let Some(cancel) = &req.cancellation_token {
                if cancel.load(Ordering::SeqCst) {
                    return Err(TranslationError::Cancelled);
                }
            }

            if let Some(delay) = self.simulated_delay {
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

                let sleep_future = tokio::time::sleep(delay);

                tokio::select! {
                    biased;
                    _ = cancel_future => {
                        return Err(TranslationError::Cancelled);
                    }
                    _ = sleep_future => {}
                }
            }

            let effective_to = LanguageCatalog::resolve_target_lang(
                &req.from,
                &req.to,
                req.fallback_to.as_deref(),
            );

            self.history.lock().unwrap().push((
                req.text.clone(),
                req.from.clone(),
                effective_to.clone(),
            ));

            if let Some(err) = &self.simulated_error {
                return Err(TranslationError::Other(err.clone()));
            }

            if let Some(translation) =
                self.canned
                    .get(&(req.text.clone(), req.from.clone(), effective_to.clone()))
            {
                return Ok(TranslationResult {
                    text: translation.clone(),
                    provider: "Mock".into(),
                    source_lang: req.from,
                    target_lang: effective_to,
                    detected_source: None,
                    model: None,
                });
            }

            Ok(TranslationResult {
                text: format!("[{}->{}] {}", req.from, effective_to, req.text),
                provider: "Mock".into(),
                source_lang: req.from,
                target_lang: effective_to,
                detected_source: None,
                model: None,
            })
        })
    }
}
