use std::collections::VecDeque;
use std::sync::{Condvar, Mutex};

use crate::{SpeechEvent, SpeechEventKind, TtsError, UtteranceId};

#[allow(dead_code)]
pub(crate) const EVENT_CAPACITY: usize = 256;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    Synthesizing,
    Playing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Active {
    utterance: UtteranceId,
    phase: Phase,
}

#[derive(Debug, Default)]
struct Inner {
    active: Option<Active>,
    events: VecDeque<SpeechEvent>,
}

#[derive(Debug, Default)]
pub(crate) struct SpeechState {
    inner: Mutex<Inner>,
    changed: Condvar,
}

impl SpeechState {
    pub(crate) fn begin(&self, utterance: UtteranceId) {
        let mut inner = self.inner.lock().unwrap();
        terminate_active(&mut inner, SpeechEventKind::Stopped);
        inner.active = Some(Active {
            utterance,
            phase: Phase::Synthesizing,
        });
        push_event(&mut inner.events, utterance, SpeechEventKind::Started);
        self.changed.notify_all();
    }

    #[allow(dead_code)]
    pub(crate) fn mark_playing(&self, utterance: UtteranceId) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if let Some(active) = inner.active.as_mut()
            && active.utterance == utterance
            && active.phase == Phase::Synthesizing
        {
            active.phase = Phase::Playing;
            return true;
        }
        false
    }

    pub(crate) fn complete(&self, utterance: UtteranceId) -> bool {
        self.terminate(utterance, SpeechEventKind::Completed)
    }

    #[allow(dead_code)]
    pub(crate) fn fail(&self, utterance: UtteranceId, message: String) -> bool {
        self.terminate(utterance, SpeechEventKind::Failed(message))
    }

    pub(crate) fn stop(&self) -> Option<UtteranceId> {
        let mut inner = self.inner.lock().unwrap();
        let utterance = inner.active.map(|active| active.utterance);
        terminate_active(&mut inner, SpeechEventKind::Stopped);
        self.changed.notify_all();
        utterance
    }

    fn terminate(&self, utterance: UtteranceId, kind: SpeechEventKind) -> bool {
        let mut inner = self.inner.lock().unwrap();
        if inner.active.map(|active| active.utterance) != Some(utterance) {
            return false;
        }
        inner.active = None;
        push_event(&mut inner.events, utterance, kind);
        self.changed.notify_all();
        true
    }

    #[allow(dead_code)]
    pub(crate) fn is_active(&self, utterance: UtteranceId) -> bool {
        self.inner
            .lock()
            .unwrap()
            .active
            .is_some_and(|active| active.utterance == utterance)
    }

    #[allow(dead_code)]
    pub(crate) fn wait_terminal(&self, utterance: UtteranceId) -> Result<(), TtsError> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if let Some(kind) = inner.events.iter().find_map(|event| {
                (event.utterance == utterance && is_terminal(&event.kind))
                    .then(|| event.kind.clone())
            }) {
                return terminal_result(kind);
            }
            if inner
                .active
                .is_none_or(|active| active.utterance != utterance)
            {
                return Err(TtsError::Other(
                    "speech terminal event was no longer available".into(),
                ));
            }
            inner = self.changed.wait(inner).unwrap();
        }
    }

    pub(crate) fn drain_events(&self) -> Vec<SpeechEvent> {
        self.inner.lock().unwrap().events.drain(..).collect()
    }
}

fn terminate_active(inner: &mut Inner, kind: SpeechEventKind) {
    if let Some(active) = inner.active.take() {
        push_event(&mut inner.events, active.utterance, kind);
    }
}

fn push_event(events: &mut VecDeque<SpeechEvent>, utterance: UtteranceId, kind: SpeechEventKind) {
    if events.len() == EVENT_CAPACITY {
        events.pop_front();
    }
    events.push_back(SpeechEvent { utterance, kind });
}

#[allow(dead_code)]
fn is_terminal(kind: &SpeechEventKind) -> bool {
    matches!(
        kind,
        SpeechEventKind::Completed | SpeechEventKind::Stopped | SpeechEventKind::Failed(_)
    )
}

#[allow(dead_code)]
fn terminal_result(kind: SpeechEventKind) -> Result<(), TtsError> {
    match kind {
        SpeechEventKind::Completed => Ok(()),
        SpeechEventKind::Stopped => Err(TtsError::Stopped),
        SpeechEventKind::Failed(message) => Err(TtsError::Other(message)),
        SpeechEventKind::Started => unreachable!("Started is not terminal"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(generation: u64) -> UtteranceId {
        UtteranceId(generation)
    }

    #[test]
    fn stop_during_pending_synthesis_is_terminal() {
        let state = SpeechState::default();
        state.begin(id(1));
        assert_eq!(state.stop(), Some(id(1)));
        assert_eq!(
            state
                .drain_events()
                .iter()
                .map(|event| &event.kind)
                .collect::<Vec<_>>(),
            vec![&SpeechEventKind::Started, &SpeechEventKind::Stopped]
        );
    }

    #[test]
    fn stale_completion_is_suppressed() {
        let state = SpeechState::default();
        state.begin(id(1));
        state.begin(id(2));
        assert!(!state.complete(id(1)));
        assert!(state.complete(id(2)));
        let events = state.drain_events();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.utterance == id(1))
                .count(),
            2
        );
    }

    #[test]
    fn setup_failure_rolls_back_with_one_terminal_after_started() {
        let state = SpeechState::default();
        state.begin(id(1));
        assert!(state.fail(id(1), "setup failed".into()));
        assert!(!state.fail(id(1), "duplicate".into()));
        assert!(!state.is_active(id(1)));
        let events = state.drain_events();
        assert_eq!(events[0].kind, SpeechEventKind::Started);
        assert_eq!(
            events[1].kind,
            SpeechEventKind::Failed("setup failed".into())
        );
    }

    #[test]
    fn queued_replacement_stops_old_before_starting_new() {
        let state = SpeechState::default();
        state.begin(id(1));
        state.begin(id(2));
        let events = state.drain_events();
        assert_eq!(
            events[1],
            SpeechEvent {
                utterance: id(1),
                kind: SpeechEventKind::Stopped
            }
        );
        assert_eq!(
            events[2],
            SpeechEvent {
                utterance: id(2),
                kind: SpeechEventKind::Started
            }
        );
    }

    #[test]
    fn events_are_bounded() {
        let state = SpeechState::default();
        for generation in 1..=(EVENT_CAPACITY as u64 * 2) {
            state.begin(id(generation));
            state.complete(id(generation));
        }
        assert_eq!(state.drain_events().len(), EVENT_CAPACITY);
    }

    #[test]
    fn one_hundred_cycles_have_one_terminal_each() {
        let state = SpeechState::default();
        for generation in 1..=100 {
            state.begin(id(generation));
            state.stop();
        }
        let events = state.drain_events();
        for generation in 1..=100 {
            let utterance_events: Vec<_> = events
                .iter()
                .filter(|event| event.utterance == id(generation))
                .collect();
            assert_eq!(utterance_events.len(), 2);
            assert_eq!(utterance_events[0].kind, SpeechEventKind::Started);
            assert!(matches!(utterance_events[1].kind, SpeechEventKind::Stopped));
        }
    }
}
