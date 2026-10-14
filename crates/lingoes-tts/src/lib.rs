use std::sync::{Arc, Mutex};
use thiserror::Error;

mod state;

#[cfg(target_os = "windows")]
mod windows;

#[derive(Debug, Error)]
pub enum TtsError {
    #[error("text is empty")]
    EmptyText,
    #[error("invalid speech options: {0}")]
    InvalidOptions(String),
    #[error("voice is unavailable: {0}")]
    VoiceUnavailable(String),
    #[error("unsupported TTS operation: {0}")]
    Unsupported(String),
    #[error("speech was stopped")]
    Stopped,
    #[error("tts command failed: {0}")]
    CommandFailed(String),
    #[error("no supported tts engine available on system: {0}")]
    Unavailable(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("tts error: {0}")]
    Other(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeechOptions {
    rate: f32,
    pitch: f32,
    volume: f32,
}

impl Default for SpeechOptions {
    fn default() -> Self {
        Self {
            rate: 1.0,
            pitch: 1.0,
            volume: 1.0,
        }
    }
}

impl SpeechOptions {
    pub fn new(rate: f32, pitch: f32, volume: f32) -> Result<Self, TtsError> {
        validate_parameter("rate", rate, 0.5..=2.0)?;
        validate_parameter("pitch", pitch, 0.5..=2.0)?;
        validate_parameter("volume", volume, 0.0..=1.0)?;
        Ok(Self {
            rate,
            pitch,
            volume,
        })
    }

    pub fn rate(self) -> f32 {
        self.rate
    }

    pub fn pitch(self) -> f32 {
        self.pitch
    }

    pub fn volume(self) -> f32 {
        self.volume
    }
}

fn validate_parameter(
    name: &str,
    value: f32,
    range: std::ops::RangeInclusive<f32>,
) -> Result<(), TtsError> {
    if value.is_finite() && range.contains(&value) {
        Ok(())
    } else {
        Err(TtsError::InvalidOptions(format!(
            "{name} must be finite and in {}..={}",
            range.start(),
            range.end()
        )))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Voice {
    pub id: String,
    pub name: String,
    pub language: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum VoiceSelection {
    #[default]
    Automatic,
    Id(String),
}

/// Selects an explicit id first; automatic selection prefers an exact BCP-47
/// language tag, then preserves language plus script, then uses the primary language.
pub fn select_voice<'a>(
    voices: &'a [Voice],
    language: &str,
    selection: &VoiceSelection,
) -> Option<&'a Voice> {
    if let VoiceSelection::Id(id) = selection {
        return (!id.trim().is_empty())
            .then(|| {
                voices
                    .iter()
                    .find(|voice| voice.id.eq_ignore_ascii_case(id))
            })
            .flatten();
    }

    let requested = LanguageTag::parse(language)?;
    let candidates: Vec<_> = voices
        .iter()
        .filter_map(|voice| LanguageTag::parse(&voice.language).map(|tag| (voice, tag)))
        .collect();
    candidates
        .iter()
        .find(|(_, tag)| tag.normalized == requested.normalized)
        .map(|(voice, _)| *voice)
        .or_else(|| {
            let script = requested.script.as_ref()?;
            candidates
                .iter()
                .find(|(_, tag)| {
                    tag.primary == requested.primary && tag.script.as_ref() == Some(script)
                })
                .map(|(voice, _)| *voice)
        })
        .or_else(|| {
            candidates
                .iter()
                .find(|(_, tag)| tag.primary == requested.primary)
                .map(|(voice, _)| *voice)
        })
}

struct LanguageTag {
    normalized: String,
    primary: String,
    script: Option<String>,
}

impl LanguageTag {
    fn parse(tag: &str) -> Option<Self> {
        let normalized = tag.trim().replace('_', "-").to_ascii_lowercase();
        let parts: Vec<_> = normalized.split('-').collect();
        let primary = *parts.first()?;
        if !(2..=8).contains(&primary.len())
            || !primary.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            return None;
        }
        if parts.iter().skip(1).any(|part| {
            part.is_empty()
                || part.len() > 8
                || !part.bytes().all(|byte| byte.is_ascii_alphanumeric())
        }) {
            return None;
        }
        let primary = primary.to_owned();
        let script = parts
            .iter()
            .skip(1)
            .find(|part| part.len() == 4 && part.bytes().all(|byte| byte.is_ascii_alphabetic()))
            .map(|part| (*part).to_owned());
        Some(Self {
            normalized,
            primary,
            script,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UtteranceId(u64);

impl UtteranceId {
    pub fn generation(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SpeechEventKind {
    Started,
    Completed,
    Stopped,
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeechEvent {
    pub utterance: UtteranceId,
    pub kind: SpeechEventKind,
}

pub trait TtsEngine: Send + Sync {
    fn voices(&self) -> Result<Vec<Voice>, TtsError> {
        Ok(Vec::new())
    }

    fn speak_async(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        options: SpeechOptions,
    ) -> Result<UtteranceId, TtsError>;

    fn stop(&self) -> Result<(), TtsError>;

    fn drain_events(&self) -> Vec<SpeechEvent> {
        Vec::new()
    }

    /// Blocking compatibility API retained for existing callers.
    fn speak(&self, text: &str, language: &str) -> Result<(), TtsError> {
        self.speak_blocking(
            text,
            language,
            VoiceSelection::Automatic,
            SpeechOptions::default(),
        )
    }

    /// Compatibility behavior that may block until the platform command exits.
    fn speak_blocking(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        options: SpeechOptions,
    ) -> Result<(), TtsError>;
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

#[derive(Debug, Default)]
struct MockState {
    spoken: Vec<(String, String)>,
    simulated_error: Option<String>,
    voices: Vec<Voice>,
    generation: u64,
    state: state::SpeechState,
}

#[derive(Debug, Default, Clone)]
pub struct MockTtsEngine {
    state: Arc<Mutex<MockState>>,
}

impl MockTtsEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_voices(voices: Vec<Voice>) -> Self {
        Self {
            state: Arc::new(Mutex::new(MockState {
                voices,
                ..MockState::default()
            })),
        }
    }

    pub fn spoken_entries(&self) -> Vec<(String, String)> {
        self.state.lock().unwrap().spoken.clone()
    }

    pub fn last_spoken(&self) -> Option<(String, String)> {
        self.state.lock().unwrap().spoken.last().cloned()
    }

    pub fn set_error(&mut self, error: Option<String>) {
        self.state.lock().unwrap().simulated_error = error;
    }
}

impl TtsEngine for MockTtsEngine {
    fn voices(&self) -> Result<Vec<Voice>, TtsError> {
        Ok(self.state.lock().unwrap().voices.clone())
    }

    fn speak_async(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        _options: SpeechOptions,
    ) -> Result<UtteranceId, TtsError> {
        if text.trim().is_empty() {
            return Err(TtsError::EmptyText);
        }
        let mut state = self.state.lock().unwrap();
        if let Some(error) = &state.simulated_error {
            return Err(TtsError::Other(error.clone()));
        }
        if !state.voices.is_empty() && select_voice(&state.voices, language, &voice).is_none() {
            return Err(TtsError::VoiceUnavailable(language.into()));
        }
        state.generation += 1;
        let utterance = UtteranceId(state.generation);
        state.spoken.push((text.into(), language.into()));
        state.state.begin(utterance);
        Ok(utterance)
    }

    fn stop(&self) -> Result<(), TtsError> {
        self.state.lock().unwrap().state.stop();
        Ok(())
    }

    fn drain_events(&self) -> Vec<SpeechEvent> {
        self.state.lock().unwrap().state.drain_events()
    }

    fn speak_blocking(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        options: SpeechOptions,
    ) -> Result<(), TtsError> {
        let utterance = self.speak_async(text, language, voice, options)?;
        let state = Arc::clone(&self.state);
        state.lock().unwrap().state.complete(utterance);
        Ok(())
    }
}

pub struct SystemTtsEngine {
    #[cfg(not(target_os = "windows"))]
    runner: Arc<dyn CommandRunner>,
    #[cfg(not(target_os = "windows"))]
    candidates: Vec<String>,
    #[cfg(target_os = "windows")]
    backend: windows::WinRtTtsEngine,
}

impl Default for SystemTtsEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTtsEngine {
    pub fn new() -> Self {
        #[cfg(target_os = "windows")]
        {
            Self {
                backend: windows::WinRtTtsEngine::new(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            Self::with_runner(Arc::new(DefaultCommandRunner))
        }
    }

    #[cfg(not(target_os = "windows"))]
    pub fn with_runner(runner: Arc<dyn CommandRunner>) -> Self {
        Self {
            runner,
            candidates: Self::default_candidates(),
        }
    }

    #[cfg(not(target_os = "windows"))]
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
            Vec::new()
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

    #[cfg(not(target_os = "windows"))]
    fn build_args<'a>(program: &str, text: &'a str, language: &'a str) -> Vec<&'a str> {
        match program {
            "spd-say" => vec!["-l", language, text],
            "espeak-ng" | "espeak" => vec!["-v", language, text],
            "say" => vec!["-v", language, text],
            _ => vec![text],
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn run_command(&self, text: &str, language: &str) -> Result<(), TtsError> {
        if text.trim().is_empty() {
            return Err(TtsError::EmptyText);
        }
        let mut last_error = None;
        for candidate in &self.candidates {
            let args = Self::build_args(candidate, text, language);
            match self.runner.run(candidate, &args) {
                Ok(true) => return Ok(()),
                Ok(false) => {
                    last_error = Some(TtsError::CommandFailed(format!(
                        "{candidate} exited with non-zero status"
                    )));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    last_error = Some(TtsError::CommandFailed(format!(
                        "{candidate} error: {error}"
                    )));
                }
            }
        }
        Err(last_error.unwrap_or_else(|| {
            TtsError::Unavailable(format!(
                "none of the TTS engines {:?} are available on this system",
                self.candidates
            ))
        }))
    }
}

impl TtsEngine for SystemTtsEngine {
    fn voices(&self) -> Result<Vec<Voice>, TtsError> {
        #[cfg(target_os = "windows")]
        return self.backend.voices();
        #[cfg(not(target_os = "windows"))]
        Ok(Vec::new())
    }

    fn speak_async(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        options: SpeechOptions,
    ) -> Result<UtteranceId, TtsError> {
        #[cfg(target_os = "windows")]
        return self.backend.speak_async(text, language, voice, options);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (language, voice, options);
            if text.trim().is_empty() {
                return Err(TtsError::EmptyText);
            }
            Err(TtsError::Unsupported(
                "asynchronous speech is unavailable for command-based backends".into(),
            ))
        }
    }

    fn stop(&self) -> Result<(), TtsError> {
        #[cfg(target_os = "windows")]
        return self.backend.stop();
        #[cfg(not(target_os = "windows"))]
        Err(TtsError::Unsupported(
            "stop is unavailable for command-based backends".into(),
        ))
    }

    fn drain_events(&self) -> Vec<SpeechEvent> {
        #[cfg(target_os = "windows")]
        return self.backend.drain_events();
        #[cfg(not(target_os = "windows"))]
        Vec::new()
    }

    fn speak_blocking(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        options: SpeechOptions,
    ) -> Result<(), TtsError> {
        #[cfg(target_os = "windows")]
        return self.backend.speak_blocking(text, language, voice, options);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (voice, options);
            self.run_command(text, language)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockRunner {
        responses: Vec<(&'static str, Result<bool, std::io::ErrorKind>)>,
        call_count: AtomicUsize,
    }

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, _args: &[&str]) -> Result<bool, std::io::Error> {
            let index = self.call_count.fetch_add(1, Ordering::SeqCst);
            if let Some((expected, response)) = self.responses.get(index) {
                assert_eq!(program, *expected);
                response.map_err(|kind| std::io::Error::new(kind, "mock error"))
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not found",
                ))
            }
        }
    }

    #[test]
    fn command_backend_falls_back_and_preserves_blocking_compatibility() {
        let runner = Arc::new(MockRunner {
            responses: vec![
                ("spd-say", Err(std::io::ErrorKind::NotFound)),
                ("espeak", Ok(true)),
            ],
            call_count: AtomicUsize::new(0),
        });
        let engine = SystemTtsEngine::with_runner_and_candidates(runner, vec!["spd-say", "espeak"]);
        engine.speak("Hello", "en").unwrap();
    }

    #[test]
    fn mock_engine_bounds_events_at_capacity() {
        let mock = MockTtsEngine::new();
        for generation in 1..=(state::EVENT_CAPACITY as u64 * 2) {
            let id = mock
                .speak_async(
                    "hello",
                    "en-US",
                    VoiceSelection::Automatic,
                    SpeechOptions::default(),
                )
                .unwrap();
            assert_eq!(id.generation(), generation);
        }
        assert_eq!(mock.drain_events().len(), state::EVENT_CAPACITY);
    }

    #[test]
    fn command_backend_reports_missing_commands() {
        let runner = Arc::new(MockRunner {
            responses: vec![
                ("spd-say", Err(std::io::ErrorKind::NotFound)),
                ("espeak", Err(std::io::ErrorKind::NotFound)),
            ],
            call_count: AtomicUsize::new(0),
        });
        let engine = SystemTtsEngine::with_runner_and_candidates(runner, vec!["spd-say", "espeak"]);
        assert!(matches!(
            engine.speak("Hello", "en"),
            Err(TtsError::Unavailable(_))
        ));
    }
}
