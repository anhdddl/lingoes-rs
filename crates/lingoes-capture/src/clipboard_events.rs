use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct DelayDebounceGate {
    delay: Duration,
    debounce: Duration,
    pending: Option<(String, Instant)>,
    emitted: Option<(String, Instant)>,
}
impl DelayDebounceGate {
    pub fn new(delay: Duration, debounce: Duration) -> Self {
        Self {
            delay,
            debounce,
            pending: None,
            emitted: None,
        }
    }
    pub fn observe(&mut self, key: &str, now: Instant) -> bool {
        if self
            .pending
            .as_ref()
            .is_none_or(|(pending, _)| pending != key)
        {
            self.pending = Some((key.to_owned(), now));
            return self.delay.is_zero();
        }
        let stable = self
            .pending
            .as_ref()
            .is_some_and(|(_, since)| now.saturating_duration_since(*since) >= self.delay);
        if !stable {
            return false;
        }
        if self.emitted.as_ref().is_some_and(|(last, at)| {
            last == key && now.saturating_duration_since(*at) < self.debounce
        }) {
            return false;
        }
        self.emitted = Some((key.to_owned(), now));
        true
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardEventProcessor {
    coalesce: Duration,
    last: Option<(String, Instant)>,
    self_write: Option<String>,
}
impl ClipboardEventProcessor {
    pub fn new(coalesce: Duration) -> Self {
        Self {
            coalesce,
            last: None,
            self_write: None,
        }
    }
    pub fn tag_self_write(&mut self, text: impl Into<String>) {
        self.self_write = Some(text.into());
    }
    pub fn on_update(&mut self, text: &str, now: Instant) -> Option<String> {
        let value = text.trim();
        if value.is_empty() {
            return None;
        }
        if self.self_write.as_deref() == Some(value) {
            self.self_write = None;
            return None;
        }
        if self.last.as_ref().is_some_and(|(last, at)| {
            last == value && now.saturating_duration_since(*at) < self.coalesce
        }) {
            return None;
        }
        self.last = Some((value.to_owned(), now));
        Some(value.to_owned())
    }
}
