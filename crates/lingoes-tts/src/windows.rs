use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

use windows::Foundation::{
    AsyncOperationCompletedHandler, AsyncStatus, IAsyncOperation, TypedEventHandler,
};
use windows::Media::Core::MediaSource;
use windows::Media::Playback::{MediaPlayer, MediaPlayerFailedEventArgs};
use windows::Media::SpeechSynthesis::{SpeechSynthesisStream, SpeechSynthesizer, VoiceInformation};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::core::{HSTRING, IInspectable};

use crate::state::SpeechState;
use crate::{
    SpeechEvent, SpeechOptions, TtsError, UtteranceId, Voice, VoiceSelection, select_voice,
};

type Reply<T> = mpsc::SyncSender<Result<T, TtsError>>;

enum Command {
    Voices(Reply<Vec<Voice>>),
    Speak {
        utterance: UtteranceId,
        text: String,
        language: String,
        voice: VoiceSelection,
        options: SpeechOptions,
    },
    SynthesisFinished {
        utterance: UtteranceId,
        result: Result<SpeechSynthesisStream, String>,
    },
    Stop(Reply<()>),
    Shutdown,
}

pub(crate) struct WinRtTtsEngine {
    sender: mpsc::Sender<Command>,
    worker: Mutex<Option<JoinHandle<()>>>,
    generation: Mutex<u64>,
    state: Arc<SpeechState>,
}

impl WinRtTtsEngine {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let state = Arc::new(SpeechState::default());
        let worker_state = Arc::clone(&state);
        let callback_sender = sender.clone();
        let worker = thread::Builder::new()
            .name("lingoes-winrt-tts".into())
            .spawn(move || worker_loop(receiver, callback_sender, worker_state))
            .expect("failed to start WinRT TTS worker");
        Self {
            sender,
            worker: Mutex::new(Some(worker)),
            generation: Mutex::new(0),
            state,
        }
    }

    pub(crate) fn voices(&self) -> Result<Vec<Voice>, TtsError> {
        request(&self.sender, Command::Voices)
    }

    pub(crate) fn speak_async(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        options: SpeechOptions,
    ) -> Result<UtteranceId, TtsError> {
        if text.trim().is_empty() {
            return Err(TtsError::EmptyText);
        }
        let utterance = self.next_utterance();
        self.sender
            .send(Command::Speak {
                utterance,
                text: text.into(),
                language: language.into(),
                voice,
                options,
            })
            .map_err(|_| worker_stopped())?;
        Ok(utterance)
    }

    pub(crate) fn stop(&self) -> Result<(), TtsError> {
        request(&self.sender, Command::Stop)
    }

    pub(crate) fn drain_events(&self) -> Vec<SpeechEvent> {
        self.state.drain_events()
    }

    pub(crate) fn speak_blocking(
        &self,
        text: &str,
        language: &str,
        voice: VoiceSelection,
        options: SpeechOptions,
    ) -> Result<(), TtsError> {
        let utterance = self.speak_async(text, language, voice, options)?;
        self.state.wait_terminal(utterance)
    }

    fn next_utterance(&self) -> UtteranceId {
        let mut generation = self.generation.lock().unwrap();
        *generation += 1;
        UtteranceId(*generation)
    }
}

impl Drop for WinRtTtsEngine {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Shutdown);
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
}

fn request<T>(
    sender: &mpsc::Sender<Command>,
    command: impl FnOnce(Reply<T>) -> Command,
) -> Result<T, TtsError> {
    let (reply, result) = mpsc::sync_channel(1);
    sender.send(command(reply)).map_err(|_| worker_stopped())?;
    result.recv().map_err(|_| worker_stopped())?
}

fn worker_stopped() -> TtsError {
    TtsError::Unavailable("WinRT TTS worker stopped".into())
}

fn winrt_error(context: &str, error: windows::core::Error) -> TtsError {
    TtsError::Other(format!("{context}: {error}"))
}

struct Runtime {
    synthesizer: SpeechSynthesizer,
    player: MediaPlayer,
    state: Arc<SpeechState>,
    synthesis: Option<(UtteranceId, IAsyncOperation<SpeechSynthesisStream>)>,
    ended_token: Option<windows::Foundation::EventRegistrationToken>,
    failed_token: Option<windows::Foundation::EventRegistrationToken>,
}

impl Runtime {
    fn new(state: Arc<SpeechState>) -> Result<Self, TtsError> {
        let synthesizer = SpeechSynthesizer::new()
            .map_err(|error| winrt_error("create speech synthesizer", error))?;
        let player =
            MediaPlayer::new().map_err(|error| winrt_error("create media player", error))?;
        Ok(Self {
            synthesizer,
            player,
            state,
            synthesis: None,
            ended_token: None,
            failed_token: None,
        })
    }

    fn voices(&self) -> Result<Vec<Voice>, TtsError> {
        installed_voices().map(|voices| voices.into_iter().map(|(_, voice)| voice).collect())
    }

    fn start(
        &mut self,
        sender: &mpsc::Sender<Command>,
        utterance: UtteranceId,
        text: &str,
        language: &str,
        selection: &VoiceSelection,
        options: SpeechOptions,
    ) {
        let _ = self.stop();
        self.state.begin(utterance);
        if let Err(error) = self.start_inner(sender, utterance, text, language, selection, options)
        {
            let _ = self.cleanup_resources();
            self.state.fail(utterance, error.to_string());
        }
    }

    fn start_inner(
        &mut self,
        sender: &mpsc::Sender<Command>,
        utterance: UtteranceId,
        text: &str,
        language: &str,
        selection: &VoiceSelection,
        options: SpeechOptions,
    ) -> Result<(), TtsError> {
        let voices = installed_voices()?;
        let metadata: Vec<_> = voices.iter().map(|(_, voice)| voice.clone()).collect();
        let selected = select_voice(&metadata, language, selection)
            .ok_or_else(|| TtsError::VoiceUnavailable(language.into()))?;
        let native = voices
            .iter()
            .find(|(_, voice)| voice.id == selected.id)
            .map(|(native, _)| native)
            .ok_or_else(|| TtsError::VoiceUnavailable(selected.id.clone()))?;
        self.synthesizer
            .SetVoice(native)
            .map_err(|error| winrt_error("select voice", error))?;
        let synth_options = self
            .synthesizer
            .Options()
            .map_err(|error| winrt_error("get speech options", error))?;
        synth_options
            .SetSpeakingRate(options.rate() as f64)
            .and_then(|_| synth_options.SetAudioPitch(options.pitch() as f64))
            .and_then(|_| synth_options.SetAudioVolume(options.volume() as f64))
            .map_err(|error| winrt_error("set speech options", error))?;

        let operation = self
            .synthesizer
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))
            .map_err(|error| winrt_error("start speech synthesis", error))?;
        let completed_sender = sender.clone();
        operation
            .SetCompleted(&AsyncOperationCompletedHandler::new(
                move |operation, status| {
                    let result = match (operation, status) {
                        (Some(operation), AsyncStatus::Completed) => operation
                            .GetResults()
                            .map_err(|error| format!("synthesize speech: {error}")),
                        (Some(operation), _) => {
                            let message = operation
                                .ErrorCode()
                                .map(|code| code.to_string())
                                .unwrap_or_else(|error| error.to_string());
                            Err(format!("synthesize speech: {message}"))
                        }
                        (None, _) => Err("synthesize speech: missing async operation".into()),
                    };
                    let _ = completed_sender.send(Command::SynthesisFinished { utterance, result });
                    Ok(())
                },
            ))
            .map_err(|error| winrt_error("register synthesis completion", error))?;
        self.synthesis = Some((utterance, operation));
        Ok(())
    }

    fn synthesis_finished(
        &mut self,
        utterance: UtteranceId,
        result: Result<SpeechSynthesisStream, String>,
    ) {
        if !self.state.is_active(utterance) {
            return;
        }
        self.synthesis.take();
        if let Err(error) = self.play(utterance, result) {
            let _ = self.cleanup_resources();
            self.state.fail(utterance, error.to_string());
        }
    }

    fn play(
        &mut self,
        utterance: UtteranceId,
        stream: Result<SpeechSynthesisStream, String>,
    ) -> Result<(), TtsError> {
        let stream = stream.map_err(TtsError::Other)?;
        let content_type = stream
            .ContentType()
            .map_err(|error| winrt_error("read speech content type", error))?;
        let source = MediaSource::CreateFromStream(&stream, &content_type)
            .map_err(|error| winrt_error("create speech media source", error))?;
        self.install_handlers(utterance)?;
        self.player
            .SetSource(&source)
            .and_then(|_| self.player.Play())
            .map_err(|error| winrt_error("play synthesized speech", error))?;
        self.state.mark_playing(utterance);
        Ok(())
    }

    fn install_handlers(&mut self, utterance: UtteranceId) -> Result<(), TtsError> {
        let ended_state = Arc::clone(&self.state);
        let ended = self
            .player
            .MediaEnded(&TypedEventHandler::<MediaPlayer, IInspectable>::new(
                move |_, _| {
                    ended_state.complete(utterance);
                    Ok(())
                },
            ))
            .map_err(|error| winrt_error("register media-ended handler", error))?;
        self.ended_token = Some(ended);

        let failed_state = Arc::clone(&self.state);
        match self.player.MediaFailed(
            &TypedEventHandler::<MediaPlayer, MediaPlayerFailedEventArgs>::new(move |_, args| {
                let message = args
                    .as_ref()
                    .and_then(|args| args.ErrorMessage().ok())
                    .map(|message| message.to_string_lossy())
                    .unwrap_or_else(|| "media playback failed".into());
                failed_state.fail(utterance, message);
                Ok(())
            }),
        ) {
            Ok(token) => {
                self.failed_token = Some(token);
                Ok(())
            }
            Err(error) => {
                let _ = self.player.RemoveMediaEnded(ended);
                self.ended_token = None;
                Err(winrt_error("register media-failed handler", error))
            }
        }
    }

    fn stop(&mut self) -> Result<(), TtsError> {
        let result = self.cleanup_resources();
        self.state.stop();
        result
    }

    fn cleanup_resources(&mut self) -> Result<(), TtsError> {
        let mut errors = Vec::new();
        if let Some((_, operation)) = self.synthesis.take() {
            if let Err(error) = operation.Cancel() {
                errors.push(format!("cancel synthesis: {error}"));
            }
            if let Err(error) = operation.Close() {
                errors.push(format!("close synthesis: {error}"));
            }
        }
        if let Err(error) = self.player.Pause() {
            errors.push(format!("pause speech playback: {error}"));
        }
        if let Some(token) = self.ended_token.take()
            && let Err(error) = self.player.RemoveMediaEnded(token)
        {
            errors.push(format!("remove media-ended handler: {error}"));
        }
        if let Some(token) = self.failed_token.take()
            && let Err(error) = self.player.RemoveMediaFailed(token)
        {
            errors.push(format!("remove media-failed handler: {error}"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(TtsError::Other(errors.join("; ")))
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.stop();
        let _ = self.player.Close();
        let _ = self.synthesizer.Close();
    }
}

fn installed_voices() -> Result<Vec<(VoiceInformation, Voice)>, TtsError> {
    let installed = SpeechSynthesizer::AllVoices()
        .map_err(|error| winrt_error("enumerate installed voices", error))?;
    let size = installed
        .Size()
        .map_err(|error| winrt_error("count installed voices", error))?;
    (0..size)
        .map(|index| {
            let native = installed
                .GetAt(index)
                .map_err(|error| winrt_error("read installed voice", error))?;
            let voice = Voice {
                id: native
                    .Id()
                    .map_err(|error| winrt_error("read voice id", error))?
                    .to_string_lossy(),
                name: native
                    .DisplayName()
                    .map_err(|error| winrt_error("read voice name", error))?
                    .to_string_lossy(),
                language: native
                    .Language()
                    .map_err(|error| winrt_error("read voice language", error))?
                    .to_string_lossy(),
            };
            Ok((native, voice))
        })
        .collect()
}

fn worker_loop(
    receiver: mpsc::Receiver<Command>,
    sender: mpsc::Sender<Command>,
    state: Arc<SpeechState>,
) {
    let com_result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.ok();
    let mut runtime = match com_result {
        Ok(()) => Runtime::new(Arc::clone(&state)),
        Err(ref error) => Err(TtsError::Other(format!("initialize COM: {error}"))),
    };
    while let Ok(command) = receiver.recv() {
        match command {
            Command::Voices(reply) => {
                let result = runtime
                    .as_ref()
                    .map_err(clone_error)
                    .and_then(Runtime::voices);
                let _ = reply.send(result);
            }
            Command::Speak {
                utterance,
                text,
                language,
                voice,
                options,
            } => match runtime.as_mut() {
                Ok(runtime) => runtime.start(&sender, utterance, &text, &language, &voice, options),
                Err(error) => {
                    state.begin(utterance);
                    state.fail(utterance, error.to_string());
                }
            },
            Command::SynthesisFinished { utterance, result } => {
                if let Ok(runtime) = runtime.as_mut() {
                    runtime.synthesis_finished(utterance, result);
                }
            }
            Command::Stop(reply) => {
                let result = runtime
                    .as_mut()
                    .map_err(|error| clone_error(&*error))
                    .and_then(Runtime::stop);
                let _ = reply.send(result);
            }
            Command::Shutdown => {
                if let Ok(runtime) = runtime.as_mut() {
                    let _ = runtime.stop();
                } else {
                    state.stop();
                }
                break;
            }
        }
    }
    drop(runtime);
    if com_result.is_ok() {
        unsafe { CoUninitialize() };
    }
}

fn clone_error(error: &TtsError) -> TtsError {
    TtsError::Other(error.to_string())
}
