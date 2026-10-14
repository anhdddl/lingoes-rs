use lingoes_tts::{
    MockTtsEngine, SpeechEventKind, SpeechOptions, TtsEngine, TtsError, Voice, VoiceSelection,
    select_voice,
};

fn voice(id: &str, language: &str) -> Voice {
    Voice {
        id: id.into(),
        name: id.into(),
        language: language.into(),
    }
}

#[test]
fn speech_options_validate_finite_ranges() {
    assert_eq!(SpeechOptions::default().rate(), 1.0);
    assert_eq!(SpeechOptions::default().pitch(), 1.0);
    assert_eq!(SpeechOptions::default().volume(), 1.0);
    assert!(SpeechOptions::new(0.5, 0.5, 0.0).is_ok());
    assert!(SpeechOptions::new(2.0, 2.0, 1.0).is_ok());

    for (rate, pitch, volume) in [
        (f32::NAN, 1.0, 1.0),
        (1.0, f32::INFINITY, 1.0),
        (1.0, 1.0, f32::NEG_INFINITY),
        (0.49, 1.0, 1.0),
        (2.01, 1.0, 1.0),
        (1.0, 0.49, 1.0),
        (1.0, 2.01, 1.0),
        (1.0, 1.0, -0.01),
        (1.0, 1.0, 1.01),
    ] {
        assert!(matches!(
            SpeechOptions::new(rate, pitch, volume),
            Err(TtsError::InvalidOptions(_))
        ));
    }
}

#[test]
fn voice_policy_prefers_id_then_exact_script_then_primary_language() {
    let voices = vec![
        voice("zh-tw", "zh-Hant-TW"),
        voice("zh-cn", "zh-Hans-CN"),
        voice("en-us", "en-US"),
        voice("en-gb", "en-GB"),
        voice("vi", "vi-VN"),
    ];

    assert_eq!(
        select_voice(&voices, "vi-vn", &VoiceSelection::Id("en-gb".into()))
            .unwrap()
            .id,
        "en-gb"
    );
    assert_eq!(
        select_voice(&voices, "EN_us", &VoiceSelection::Automatic)
            .unwrap()
            .id,
        "en-us"
    );
    assert_eq!(
        select_voice(&voices, "zh-Hans-SG", &VoiceSelection::Automatic)
            .unwrap()
            .id,
        "zh-cn"
    );
    assert_eq!(
        select_voice(&voices, "en-AU", &VoiceSelection::Automatic)
            .unwrap()
            .id,
        "en-us"
    );
    for malformed in ["", "  ", "-en", "en-", "en--US", "e", "en-!"] {
        assert!(select_voice(&voices, malformed, &VoiceSelection::Automatic).is_none());
    }
    assert!(select_voice(&voices, "fr-FR", &VoiceSelection::Id("missing".into())).is_none());
}

#[cfg(not(windows))]
#[test]
fn command_backend_explicitly_rejects_untracked_async_operations() {
    let engine = lingoes_tts::SystemTtsEngine::new();
    assert!(matches!(
        engine.speak_async(
            "hello",
            "en-US",
            VoiceSelection::Automatic,
            SpeechOptions::default()
        ),
        Err(TtsError::Unsupported(_))
    ));
    assert!(matches!(engine.stop(), Err(TtsError::Unsupported(_))));
}

#[test]
fn mock_stop_emits_generation_scoped_events_for_repeated_cycles() {
    let mock = MockTtsEngine::with_voices(vec![voice("en-us", "en-US")]);
    let options = SpeechOptions::default();

    for generation in 1..=100 {
        let id = mock
            .speak_async("hello", "en-US", VoiceSelection::Automatic, options)
            .unwrap();
        assert_eq!(id.generation(), generation);
        mock.stop().unwrap();
    }

    let events = mock.drain_events();
    assert_eq!(events.len(), 200);
    for (index, pair) in events.chunks_exact(2).enumerate() {
        let generation = index as u64 + 1;
        assert_eq!(pair[0].utterance.generation(), generation);
        assert_eq!(pair[0].kind, SpeechEventKind::Started);
        assert_eq!(pair[1].utterance.generation(), generation);
        assert_eq!(pair[1].kind, SpeechEventKind::Stopped);
    }
}

#[test]
fn legacy_speak_is_a_documented_blocking_compatibility_call() {
    let mock = MockTtsEngine::new();
    mock.speak("Hello", "en").unwrap();
    assert_eq!(mock.last_spoken(), Some(("Hello".into(), "en".into())));

    mock.speak_blocking(
        "Xin chào",
        "vi",
        VoiceSelection::Automatic,
        SpeechOptions::default(),
    )
    .unwrap();
    assert_eq!(mock.last_spoken(), Some(("Xin chào".into(), "vi".into())));
}
