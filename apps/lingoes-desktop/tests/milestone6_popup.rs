use lingoes_desktop::popup::{PopupContent, PopupController, PopupState};

fn content(headword: &str) -> PopupContent {
    PopupContent::new(headword, format!("definition of {headword}"))
}

#[test]
fn capture_moves_hidden_through_pending_to_transient_with_content() {
    let mut popup = PopupController::new();

    assert_eq!(popup.state(), PopupState::Hidden);
    assert_eq!(popup.content(), None);

    let token = popup.begin_capture();
    assert_eq!(popup.state(), PopupState::Pending);

    let expected = content("keyboard");
    assert!(popup.complete_capture(token, Some(expected.clone())));
    assert_eq!(popup.state(), PopupState::VisibleTransient);
    assert_eq!(popup.content(), Some(&expected));
}

#[test]
fn capture_tokens_increase_and_stale_completions_are_rejected() {
    let mut popup = PopupController::new();

    let stale = popup.begin_capture();
    let current = popup.begin_capture();
    assert!(current > stale);

    assert!(!popup.complete_capture(stale, Some(content("stale"))));
    assert_eq!(popup.state(), PopupState::Pending);
    assert_eq!(popup.content(), None);

    let expected = content("current");
    assert!(popup.complete_capture(current, Some(expected.clone())));
    assert_eq!(popup.content(), Some(&expected));
}

#[test]
fn capture_without_content_returns_pending_popup_to_hidden() {
    let mut popup = PopupController::new();
    let token = popup.begin_capture();

    assert!(popup.complete_capture(token, None));
    assert_eq!(popup.state(), PopupState::Hidden);
    assert_eq!(popup.content(), None);
}

#[test]
fn outside_click_dismisses_pending_and_transient_but_not_pinned_or_expanded() {
    let mut pending = PopupController::new();
    let pending_token = pending.begin_capture();
    pending.outside_click();
    assert_eq!(pending.state(), PopupState::Hidden);
    assert!(!pending.complete_capture(pending_token, Some(content("late"))));

    let mut transient = visible_popup("transient");
    transient.outside_click();
    assert_eq!(transient.state(), PopupState::Hidden);
    assert_eq!(transient.content(), None);

    let mut pinned = visible_popup("pinned");
    pinned.pin();
    pinned.outside_click();
    assert_eq!(pinned.state(), PopupState::Pinned);
    assert_eq!(pinned.content(), Some(&content("pinned")));

    let mut expanded = visible_popup("expanded");
    expanded.expand();
    expanded.outside_click();
    assert_eq!(expanded.state(), PopupState::Expanded);
    assert_eq!(expanded.content(), Some(&content("expanded")));
}

#[test]
fn escape_hides_every_state_and_rejects_pending_completion() {
    let mut pending = PopupController::new();
    let token = pending.begin_capture();
    pending.escape();
    assert_eq!(pending.state(), PopupState::Hidden);
    assert!(!pending.complete_capture(token, Some(content("late"))));

    for mut popup in [
        visible_popup("transient"),
        pinned_popup("pinned"),
        expanded_popup("expanded"),
    ] {
        popup.escape();
        assert_eq!(popup.state(), PopupState::Hidden);
        assert_eq!(popup.content(), None);
    }
}

#[test]
fn persisted_pin_preference_applies_to_the_next_completed_capture() {
    let mut popup = PopupController::new();
    popup.set_preferred_pinned(true);
    let token = popup.begin_capture();

    assert!(popup.complete_capture(token, Some(content("keyboard"))));
    assert_eq!(popup.state(), PopupState::Pinned);
}

#[test]
fn pinning_keeps_popup_visible_and_unpinning_restores_transient_behavior() {
    let mut popup = visible_popup("keyboard");

    popup.pin();
    assert_eq!(popup.state(), PopupState::Pinned);
    popup.outside_click();
    assert_eq!(popup.state(), PopupState::Pinned);

    popup.unpin();
    assert_eq!(popup.state(), PopupState::VisibleTransient);
    popup.outside_click();
    assert_eq!(popup.state(), PopupState::Hidden);
}

#[test]
fn expansion_and_collapse_preserve_content() {
    let mut popup = visible_popup("keyboard");
    let expected = content("keyboard");

    popup.expand();
    assert_eq!(popup.state(), PopupState::Expanded);
    assert_eq!(popup.content(), Some(&expected));

    popup.collapse();
    assert_eq!(popup.state(), PopupState::VisibleTransient);
    assert_eq!(popup.content(), Some(&expected));

    popup.pin();
    popup.expand();
    assert!(popup.is_pinned());
    popup.unpin();
    assert!(!popup.is_pinned());
    popup.collapse();
    assert_eq!(popup.state(), PopupState::VisibleTransient);

    popup.pin();
    popup.expand();
    popup.collapse();
    assert_eq!(popup.state(), PopupState::Pinned);
}

#[test]
fn pinned_and_expanded_popups_update_without_disappearing_during_capture() {
    let mut pinned = pinned_popup("old-pinned");
    let pinned_token = pinned.begin_capture();
    assert_eq!(pinned.state(), PopupState::Pinned);
    assert_eq!(pinned.content(), Some(&content("old-pinned")));
    assert!(pinned.complete_capture(pinned_token, Some(content("new-pinned"))));
    assert_eq!(pinned.state(), PopupState::Pinned);
    assert_eq!(pinned.content(), Some(&content("new-pinned")));

    let mut expanded = expanded_popup("old-expanded");
    let expanded_token = expanded.begin_capture();
    assert_eq!(expanded.state(), PopupState::Expanded);
    assert!(expanded.complete_capture(expanded_token, Some(content("new-expanded"))));
    assert_eq!(expanded.state(), PopupState::Expanded);
    assert_eq!(expanded.content(), Some(&content("new-expanded")));
}

fn visible_popup(headword: &str) -> PopupController {
    let mut popup = PopupController::new();
    let token = popup.begin_capture();
    assert!(popup.complete_capture(token, Some(content(headword))));
    popup
}

fn pinned_popup(headword: &str) -> PopupController {
    let mut popup = visible_popup(headword);
    popup.pin();
    popup
}

fn expanded_popup(headword: &str) -> PopupController {
    let mut popup = visible_popup(headword);
    popup.expand();
    popup
}
