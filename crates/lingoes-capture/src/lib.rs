use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClipboardError {
    #[error("clipboard io error: {0}")]
    Io(String),
    #[error("clipboard unavailable: {0}")]
    Unavailable(String),
    #[error("empty clipboard")]
    Empty,
}

pub trait ClipboardReader: Send + Sync {
    fn get_text(&mut self) -> Result<String, ClipboardError>;
    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError>;
}

#[derive(Debug, Default, Clone)]
pub struct MockClipboardReader {
    content: Arc<Mutex<Option<String>>>,
    error: Arc<Mutex<Option<String>>>,
}

impl MockClipboardReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_text(text: impl Into<String>) -> Self {
        let mock = Self::new();
        *mock.content.lock().unwrap() = Some(text.into());
        mock
    }

    pub fn set_error(&mut self, error: Option<String>) {
        *self.error.lock().unwrap() = error;
    }
}

impl ClipboardReader for MockClipboardReader {
    fn get_text(&mut self) -> Result<String, ClipboardError> {
        if let Some(err) = self.error.lock().unwrap().as_ref() {
            return Err(ClipboardError::Io(err.clone()));
        }
        match self.content.lock().unwrap().as_ref() {
            Some(text) => Ok(text.clone()),
            None => Err(ClipboardError::Empty),
        }
    }

    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        if let Some(err) = self.error.lock().unwrap().as_ref() {
            return Err(ClipboardError::Io(err.clone()));
        }
        *self.content.lock().unwrap() = Some(text.to_owned());
        Ok(())
    }
}

pub struct SystemClipboardReader {
    clipboard: Option<arboard::Clipboard>,
}

impl SystemClipboardReader {
    pub fn new() -> Result<Self, ClipboardError> {
        match arboard::Clipboard::new() {
            Ok(clipboard) => Ok(Self {
                clipboard: Some(clipboard),
            }),
            Err(err) => Err(ClipboardError::Unavailable(err.to_string())),
        }
    }
}

impl ClipboardReader for SystemClipboardReader {
    fn get_text(&mut self) -> Result<String, ClipboardError> {
        let clipboard = self
            .clipboard
            .as_mut()
            .ok_or_else(|| ClipboardError::Unavailable("clipboard not initialized".into()))?;
        clipboard
            .get_text()
            .map_err(|err| ClipboardError::Io(err.to_string()))
    }

    fn set_text(&mut self, text: &str) -> Result<(), ClipboardError> {
        let clipboard = self
            .clipboard
            .as_mut()
            .ok_or_else(|| ClipboardError::Unavailable("clipboard not initialized".into()))?;
        clipboard
            .set_text(text)
            .map_err(|err| ClipboardError::Io(err.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct ClipboardWatcherConfig {
    pub poll_interval: Duration,
    pub min_length: usize,
    pub max_length: usize,
}

impl Default for ClipboardWatcherConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(200),
            min_length: 1,
            max_length: 2000,
        }
    }
}

pub struct ClipboardWatcher {
    config: ClipboardWatcherConfig,
    last_text: Option<String>,
}

impl Default for ClipboardWatcher {
    fn default() -> Self {
        Self::new(ClipboardWatcherConfig::default())
    }
}

impl ClipboardWatcher {
    pub fn new(config: ClipboardWatcherConfig) -> Self {
        Self {
            config,
            last_text: None,
        }
    }

    pub fn config(&self) -> &ClipboardWatcherConfig {
        &self.config
    }

    pub fn set_last_text(&mut self, text: Option<String>) {
        self.last_text = text;
    }

    pub fn poll_step<R: ClipboardReader>(&mut self, reader: &mut R) -> Option<String> {
        let raw = match reader.get_text() {
            Ok(text) => text,
            Err(_) => return None,
        };

        let trimmed = raw.trim();
        let char_count = trimmed.chars().count();

        if char_count < self.config.min_length || char_count > self.config.max_length {
            return None;
        }

        if let Some(ref last) = self.last_text {
            if last == trimmed {
                return None;
            }
        }

        let result = trimmed.to_string();
        self.last_text = Some(result.clone());
        Some(result)
    }

    pub fn spawn<R: ClipboardReader + 'static>(
        mut self,
        mut reader: R,
    ) -> (
        crossbeam_channel::Receiver<String>,
        Arc<AtomicBool>,
        std::thread::JoinHandle<()>,
    ) {
        let (tx, rx) = crossbeam_channel::unbounded();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        let handle = std::thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                if let Some(captured) = self.poll_step(&mut reader) {
                    if tx.send(captured).is_err() {
                        break;
                    }
                }
                std::thread::sleep(self.config.poll_interval);
            }
        });

        (rx, running, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_reader_can_store_and_retrieve_text() {
        let mut reader = MockClipboardReader::new();
        reader.set_text("hello").unwrap();
        assert_eq!(reader.get_text().unwrap(), "hello");
    }

    #[test]
    fn mock_reader_simulates_error() {
        let mut reader = MockClipboardReader::new();
        reader.set_error(Some("access denied".into()));
        let err = reader.get_text().unwrap_err();
        match err {
            ClipboardError::Io(msg) => assert_eq!(msg, "access denied"),
            _ => panic!("unexpected error variant"),
        }
    }

    #[test]
    fn watcher_filters_empty_and_whitespace_only() {
        let mut watcher = ClipboardWatcher::default();
        let mut reader = MockClipboardReader::with_text("   \n\t  ");
        assert_eq!(watcher.poll_step(&mut reader), None);
    }

    #[test]
    fn watcher_filters_below_min_length() {
        let config = ClipboardWatcherConfig {
            min_length: 3,
            max_length: 100,
            ..Default::default()
        };
        let mut watcher = ClipboardWatcher::new(config);

        let mut reader = MockClipboardReader::with_text("hi");
        assert_eq!(watcher.poll_step(&mut reader), None);

        reader.set_text("cat").unwrap();
        assert_eq!(watcher.poll_step(&mut reader), Some("cat".to_string()));
    }

    #[test]
    fn watcher_filters_above_max_length() {
        let config = ClipboardWatcherConfig {
            min_length: 1,
            max_length: 5,
            ..Default::default()
        };
        let mut watcher = ClipboardWatcher::new(config);

        let mut reader = MockClipboardReader::with_text("123456");
        assert_eq!(watcher.poll_step(&mut reader), None);

        reader.set_text("12345").unwrap();
        assert_eq!(watcher.poll_step(&mut reader), Some("12345".to_string()));
    }

    #[test]
    fn watcher_trims_whitespace_before_emitting() {
        let mut watcher = ClipboardWatcher::default();
        let mut reader = MockClipboardReader::with_text("  bonjour  \n");
        assert_eq!(watcher.poll_step(&mut reader), Some("bonjour".to_string()));
    }

    #[test]
    fn watcher_debounces_identical_clips() {
        let mut watcher = ClipboardWatcher::default();
        let mut reader = MockClipboardReader::with_text("hello");

        assert_eq!(watcher.poll_step(&mut reader), Some("hello".to_string()));
        // Second poll with same text -> None
        assert_eq!(watcher.poll_step(&mut reader), None);
        // Third poll with same text plus spaces -> None
        reader.set_text("  hello  ").unwrap();
        assert_eq!(watcher.poll_step(&mut reader), None);
    }

    #[test]
    fn watcher_emits_when_content_changes_back() {
        let mut watcher = ClipboardWatcher::default();
        let mut reader = MockClipboardReader::with_text("apple");

        assert_eq!(watcher.poll_step(&mut reader), Some("apple".to_string()));
        reader.set_text("banana").unwrap();
        assert_eq!(watcher.poll_step(&mut reader), Some("banana".to_string()));
        reader.set_text("apple").unwrap();
        assert_eq!(watcher.poll_step(&mut reader), Some("apple".to_string()));
    }

    #[test]
    fn watcher_handles_reader_errors_gracefully() {
        let mut watcher = ClipboardWatcher::default();
        let mut reader = MockClipboardReader::new();
        reader.set_error(Some("system busy".into()));

        assert_eq!(watcher.poll_step(&mut reader), None);

        reader.set_error(None);
        reader.set_text("recovered").unwrap();
        assert_eq!(
            watcher.poll_step(&mut reader),
            Some("recovered".to_string())
        );
    }

    #[test]
    fn watcher_background_thread_lifecycle_and_channel_delivery() {
        let config = ClipboardWatcherConfig {
            poll_interval: Duration::from_millis(10),
            min_length: 1,
            max_length: 50,
        };
        let watcher = ClipboardWatcher::new(config);
        let reader = MockClipboardReader::new();
        let reader_clone = reader.clone();

        let (rx, running, handle) = watcher.spawn(reader);

        // Feed word 1
        let mut mut_clone = reader_clone.clone();
        mut_clone.set_text("first").unwrap();
        let received = rx
            .recv_timeout(Duration::from_millis(500))
            .expect("should receive first event");
        assert_eq!(received, "first");

        // Feed word 2
        mut_clone.set_text("second").unwrap();
        let received = rx
            .recv_timeout(Duration::from_millis(500))
            .expect("should receive second event");
        assert_eq!(received, "second");

        // Stop watcher
        running.store(false, Ordering::Relaxed);
        handle.join().expect("thread join");
    }
}
