use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TtsError {
    #[error("text is empty")]
    EmptyText,
    #[error("tts command failed: {0}")]
    CommandFailed(String),
    #[error("no supported tts engine available on system: {0}")]
    Unavailable(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("tts error: {0}")]
    Other(String),
}

pub trait TtsEngine: Send + Sync {
    fn speak(&self, text: &str, lang: &str) -> Result<(), TtsError>;
}

pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<bool, std::io::Error>;
}

#[derive(Debug, Default)]
pub struct DefaultCommandRunner;

impl CommandRunner for DefaultCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<bool, std::io::Error> {
        let status = std::process::Command::new(program).args(args).status()?;
        Ok(status.success())
    }
}

#[derive(Debug, Default, Clone)]
pub struct MockTtsEngine {
    spoken: Arc<Mutex<Vec<(String, String)>>>,
    simulated_error: Arc<Mutex<Option<String>>>,
}

impl MockTtsEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spoken_entries(&self) -> Vec<(String, String)> {
        self.spoken.lock().unwrap().clone()
    }

    pub fn last_spoken(&self) -> Option<(String, String)> {
        self.spoken.lock().unwrap().last().cloned()
    }

    pub fn set_error(&mut self, error: Option<String>) {
        *self.simulated_error.lock().unwrap() = error;
    }
}

impl TtsEngine for MockTtsEngine {
    fn speak(&self, text: &str, lang: &str) -> Result<(), TtsError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TtsError::EmptyText);
        }
        if let Some(err) = self.simulated_error.lock().unwrap().as_ref() {
            return Err(TtsError::Other(err.clone()));
        }
        self.spoken
            .lock()
            .unwrap()
            .push((text.to_owned(), lang.to_owned()));
        Ok(())
    }
}

pub struct SystemTtsEngine {
    runner: Arc<dyn CommandRunner>,
    candidates: Vec<String>,
}

impl Default for SystemTtsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTtsEngine {
    pub fn new() -> Self {
        Self::with_runner(Arc::new(DefaultCommandRunner))
    }

    pub fn with_runner(runner: Arc<dyn CommandRunner>) -> Self {
        Self {
            runner,
            candidates: Self::default_candidates(),
        }
    }

    pub fn with_runner_and_candidates(
        runner: Arc<dyn CommandRunner>,
        candidates: Vec<&str>,
    ) -> Self {
        Self {
            runner,
            candidates: candidates.into_iter().map(String::from).collect(),
        }
    }

    pub fn default_candidates() -> Vec<String> {
        #[cfg(target_os = "windows")]
        {
            vec!["powershell".into()]
        }
        #[cfg(target_os = "macos")]
        {
            vec!["say".into()]
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            vec!["spd-say".into(), "espeak-ng".into(), "espeak".into()]
        }
    }

    fn build_args<'a>(program: &str, text: &'a str, lang: &'a str) -> Vec<&'a str> {
        match program {
            "spd-say" => vec!["-l", lang, text],
            "espeak-ng" | "espeak" => vec!["-v", lang, text],
            "say" => vec!["-v", lang, text],
            _ => vec![text],
        }
    }
}

impl TtsEngine for SystemTtsEngine {
    fn speak(&self, text: &str, lang: &str) -> Result<(), TtsError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(TtsError::EmptyText);
        }

        let mut last_error = None;
        for candidate in &self.candidates {
            let args = Self::build_args(candidate, text, lang);
            match self.runner.run(candidate, &args) {
                Ok(true) => return Ok(()),
                Ok(false) => {
                    last_error = Some(TtsError::CommandFailed(format!(
                        "{candidate} exited with non-zero status"
                    )));
                }
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                    continue;
                }
                Err(err) => {
                    last_error = Some(TtsError::CommandFailed(format!("{candidate} error: {err}")));
                }
            }
        }

        if let Some(err) = last_error {
            Err(err)
        } else {
            Err(TtsError::Unavailable(format!(
                "none of the TTS engines {:?} are available on this system",
                self.candidates
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn mock_tts_engine_records_spoken_text_and_language() {
        let mock = MockTtsEngine::new();
        mock.speak("Hello world", "en").unwrap();
        mock.speak("Xin chào", "vi").unwrap();

        let entries = mock.spoken_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], ("Hello world".into(), "en".into()));
        assert_eq!(entries[1], ("Xin chào".into(), "vi".into()));
        assert_eq!(mock.last_spoken(), Some(("Xin chào".into(), "vi".into())));
    }

    #[test]
    fn mock_tts_engine_rejects_empty_text() {
        let mock = MockTtsEngine::new();
        let err = mock.speak("   \t\n", "en").unwrap_err();
        assert!(matches!(err, TtsError::EmptyText));
    }

    #[test]
    fn mock_tts_engine_simulates_failure() {
        let mut mock = MockTtsEngine::new();
        mock.set_error(Some("Audio output busy".into()));

        let err = mock.speak("Test", "en").unwrap_err();
        match err {
            TtsError::Other(msg) => assert_eq!(msg, "Audio output busy"),
            _ => panic!("Expected Other error"),
        }
    }

    #[test]
    fn system_tts_engine_rejects_empty_text() {
        let engine = SystemTtsEngine::new();
        let err = engine.speak("", "en").unwrap_err();
        assert!(matches!(err, TtsError::EmptyText));
    }

    struct MockRunner {
        responses: Vec<(&'static str, Result<bool, std::io::ErrorKind>)>,
        call_count: AtomicUsize,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, _args: &[&str]) -> Result<bool, std::io::Error> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            if let Some((expected_prog, res)) = self.responses.get(idx) {
                assert_eq!(program, *expected_prog);
                match res {
                    Ok(success) => Ok(*success),
                    Err(kind) => Err(std::io::Error::new(*kind, "mock error")),
                }
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                ))
            }
        }
    }

    #[test]
    fn system_tts_engine_succeeds_with_first_available_command() {
        let runner = Arc::new(MockRunner {
            responses: vec![("spd-say", Ok(true))],
            call_count: AtomicUsize::new(0),
        });

        let engine = SystemTtsEngine::with_runner_and_candidates(runner, vec!["spd-say", "espeak"]);
        let result = engine.speak("Hello", "en");
        assert!(result.is_ok());
    }

    #[test]
    fn system_tts_engine_falls_back_when_first_command_missing() {
        let runner = Arc::new(MockRunner {
            responses: vec![
                ("spd-say", Err(std::io::ErrorKind::NotFound)),
                ("espeak", Ok(true)),
            ],
            call_count: AtomicUsize::new(0),
        });

        let engine = SystemTtsEngine::with_runner_and_candidates(runner, vec!["spd-say", "espeak"]);
        let result = engine.speak("Hello", "en");
        assert!(result.is_ok());
    }

    #[test]
    fn system_tts_engine_returns_unavailable_when_all_commands_missing() {
        let runner = Arc::new(MockRunner {
            responses: vec![
                ("spd-say", Err(std::io::ErrorKind::NotFound)),
                ("espeak", Err(std::io::ErrorKind::NotFound)),
            ],
            call_count: AtomicUsize::new(0),
        });

        let engine = SystemTtsEngine::with_runner_and_candidates(runner, vec!["spd-say", "espeak"]);
        let err = engine.speak("Hello", "en").unwrap_err();
        assert!(matches!(err, TtsError::Unavailable(_)));
    }

    #[test]
    fn system_tts_engine_handles_command_failure() {
        let runner = Arc::new(MockRunner {
            responses: vec![("spd-say", Ok(false)), ("espeak", Ok(false))],
            call_count: AtomicUsize::new(0),
        });

        let engine = SystemTtsEngine::with_runner_and_candidates(runner, vec!["spd-say", "espeak"]);
        let err = engine.speak("Hello", "en").unwrap_err();
        assert!(matches!(
            err,
            TtsError::CommandFailed(_) | TtsError::Unavailable(_)
        ));
    }
}
