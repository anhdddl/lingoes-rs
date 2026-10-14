/// Identifies one popup content request.
///
/// Tokens are issued in strictly increasing order so asynchronous capture and
/// lookup completions can be matched to the request that is still current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaptureToken(u64);

impl CaptureToken {
    pub fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupState {
    #[default]
    Hidden,
    Pending,
    VisibleTransient,
    Pinned,
    Expanded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopupContent {
    headword: String,
    definition: String,
}

impl PopupContent {
    pub fn new(headword: impl Into<String>, definition: impl Into<String>) -> Self {
        Self {
            headword: headword.into(),
            definition: definition.into(),
        }
    }

    pub fn headword(&self) -> &str {
        &self.headword
    }

    pub fn definition(&self) -> &str {
        &self.definition
    }
}

/// Owns popup lifecycle state independently of the window toolkit.
///
/// The controller intentionally has no Slint or native-window dependency. A
/// later window adapter can translate its state into visibility and focus
/// policy without putting lifecycle rules in UI callbacks.
#[derive(Debug, Default)]
pub struct PopupController {
    state: PopupState,
    content: Option<PopupContent>,
    last_token: u64,
    active_token: Option<CaptureToken>,
    expanded_from_pinned: bool,
    preferred_pinned: bool,
}

impl PopupController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> PopupState {
        self.state
    }

    pub fn is_pinned(&self) -> bool {
        self.preferred_pinned
    }

    pub fn content(&self) -> Option<&PopupContent> {
        self.content.as_ref()
    }

    pub fn begin_capture(&mut self) -> CaptureToken {
        self.last_token = self
            .last_token
            .checked_add(1)
            .expect("capture token space exhausted");
        let token = CaptureToken(self.last_token);
        self.active_token = Some(token);

        if !matches!(self.state, PopupState::Pinned | PopupState::Expanded) {
            self.state = PopupState::Pending;
            self.content = None;
        }

        token
    }

    /// Applies a completion only when it belongs to the current request.
    /// Returns `false` for stale or cancelled requests.
    pub fn complete_capture(&mut self, token: CaptureToken, content: Option<PopupContent>) -> bool {
        if self.active_token != Some(token) {
            return false;
        }
        self.active_token = None;

        match content {
            Some(content) => {
                self.content = Some(content);
                if self.state == PopupState::Pending {
                    self.state = if self.preferred_pinned {
                        PopupState::Pinned
                    } else {
                        PopupState::VisibleTransient
                    };
                }
            }
            None if self.state == PopupState::Pending => self.hide(),
            None => {}
        }

        true
    }

    pub fn outside_click(&mut self) {
        if matches!(
            self.state,
            PopupState::Pending | PopupState::VisibleTransient
        ) {
            self.hide();
        }
    }

    pub fn escape(&mut self) {
        self.hide();
    }

    pub fn close(&mut self) {
        self.hide();
    }

    pub fn set_preferred_pinned(&mut self, pinned: bool) {
        self.preferred_pinned = pinned;
    }

    pub fn pin(&mut self) {
        self.preferred_pinned = true;
        if self.state == PopupState::VisibleTransient {
            self.state = PopupState::Pinned;
        } else if self.state == PopupState::Expanded {
            self.expanded_from_pinned = true;
        }
    }

    pub fn unpin(&mut self) {
        self.preferred_pinned = false;
        if self.state == PopupState::Pinned {
            self.state = PopupState::VisibleTransient;
        } else if self.state == PopupState::Expanded {
            self.expanded_from_pinned = false;
        }
    }

    pub fn expand(&mut self) {
        if matches!(
            self.state,
            PopupState::VisibleTransient | PopupState::Pinned
        ) {
            self.expanded_from_pinned = self.state == PopupState::Pinned;
            self.state = PopupState::Expanded;
        }
    }

    pub fn collapse(&mut self) {
        if self.state == PopupState::Expanded {
            self.state = if self.expanded_from_pinned {
                PopupState::Pinned
            } else {
                PopupState::VisibleTransient
            };
            self.expanded_from_pinned = false;
        }
    }

    fn hide(&mut self) {
        self.state = PopupState::Hidden;
        self.content = None;
        self.active_token = None;
        self.expanded_from_pinned = false;
    }
}
