use std::{
    collections::HashMap,
    ffi::CString,
    os::raw::c_int,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use yune_core::{Engine, KeyEvent};

use crate::{
    apply_schema_to_session, bool_from, deployed_schema_list_entries, userdb, AffixSegmentor,
    AsciiModeSwitchStyle, Bool, ChordComposerProcessor, EditorBindingAction, EditorCharHandler,
    EditorProcessor, KeyBinderProcessor, MatcherSegmentor, NavigatorBindings,
    NavigatorSyllableJumpPosition, PunctSegmentor, PunctuationProcessor, RecognizerProcessor,
    RimeSessionId, SelectorBindings, SpellerProcessor, FALSE,
};

pub(crate) const SESSION_LIFESPAN_SECS: u64 = 5 * 60;

#[derive(Default)]
pub(crate) struct SessionRegistry {
    pub(crate) next_id: RimeSessionId,
    pub(crate) sessions: HashMap<RimeSessionId, SessionState>,
}

impl SessionRegistry {
    pub(crate) fn create_session(&mut self) -> RimeSessionId {
        if !service_started().load(Ordering::SeqCst) {
            return 0;
        }

        self.next_id = self.next_id.saturating_add(1).max(1);
        let session_id = self.next_id;
        let mut session = SessionState::new();
        apply_initial_schema_to_session(&mut session);
        self.sessions.insert(session_id, session);
        session_id
    }

    pub(crate) fn get_session_mut(
        &mut self,
        session_id: RimeSessionId,
    ) -> Option<&mut SessionState> {
        if session_id == 0 || !service_started().load(Ordering::SeqCst) {
            return None;
        }

        let session = self.sessions.get_mut(&session_id)?;
        session.activate();
        Some(session)
    }

    pub(crate) fn find_session(&mut self, session_id: RimeSessionId) -> bool {
        self.get_session_mut(session_id).is_some()
    }

    pub(crate) fn cleanup_stale_sessions(&mut self) {
        let now = session_activity_now();
        self.sessions.retain(|_, session| {
            now.saturating_sub(session.last_active_time) <= SESSION_LIFESPAN_SECS
        });
    }
}

fn apply_initial_schema_to_session(session: &mut SessionState) {
    if let Some((schema_id, _)) = deployed_schema_list_entries().into_iter().next() {
        apply_schema_to_session(session, &schema_id);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RemainingGearDeferral {
    pub(crate) gear: String,
    pub(crate) observed_librime_role: String,
    pub(crate) current_yune_behavior: String,
    pub(crate) scope_decision: String,
    pub(crate) target_phase: String,
}

pub(crate) struct SessionState {
    pub(crate) engine: Engine,
    pub(crate) unread_commit: Option<String>,
    pub(crate) input_buffer: Option<CString>,
    pub(crate) key_binder: Option<KeyBinderProcessor>,
    pub(crate) speller: Option<SpellerProcessor>,
    pub(crate) editor_processor: Option<EditorProcessor>,
    pub(crate) editor_bindings: HashMap<KeyEvent, EditorBindingAction>,
    pub(crate) editor_char_handler: Option<EditorCharHandler>,
    pub(crate) chord_composer: Option<ChordComposerProcessor>,
    pub(crate) ascii_composer_enabled: bool,
    pub(crate) ascii_composer_switch_bindings: HashMap<c_int, AsciiModeSwitchStyle>,
    pub(crate) ascii_composer_pressed_switch_key: Option<c_int>,
    pub(crate) ascii_composer_inline_ascii: bool,
    pub(crate) ascii_segmentor_enabled: bool,
    pub(crate) punctuation_processor: Option<PunctuationProcessor>,
    pub(crate) recognizer_processor: Option<RecognizerProcessor>,
    pub(crate) selector_bindings: SelectorBindings,
    pub(crate) navigator_bindings: NavigatorBindings,
    pub(crate) navigator_delimiters: String,
    pub(crate) navigator_syllable_jump_position: NavigatorSyllableJumpPosition,
    pub(crate) base_segment_tags: Vec<String>,
    pub(crate) punct_segmentor: Option<PunctSegmentor>,
    pub(crate) affix_segmentors: Vec<AffixSegmentor>,
    pub(crate) matcher_segmentor: Option<MatcherSegmentor>,
    pub(crate) fallback_segmentor_enabled: bool,
    pub(crate) remaining_gear_deferrals: Vec<RemainingGearDeferral>,
    pub(crate) paging: bool,
    pub(crate) user_dict_name: Option<String>,
    pub(crate) last_active_time: u64,
}

impl SessionState {
    fn new() -> Self {
        Self {
            engine: Engine::default(),
            unread_commit: None,
            input_buffer: None,
            key_binder: None,
            speller: None,
            editor_processor: None,
            editor_bindings: HashMap::new(),
            editor_char_handler: None,
            chord_composer: None,
            ascii_composer_enabled: false,
            ascii_composer_switch_bindings: HashMap::new(),
            ascii_composer_pressed_switch_key: None,
            ascii_composer_inline_ascii: false,
            ascii_segmentor_enabled: false,
            punctuation_processor: None,
            recognizer_processor: None,
            selector_bindings: SelectorBindings::default(),
            navigator_bindings: NavigatorBindings::default(),
            navigator_delimiters: " ".to_owned(),
            navigator_syllable_jump_position: NavigatorSyllableJumpPosition::AfterDelimiter,
            base_segment_tags: vec!["abc".to_owned()],
            punct_segmentor: None,
            affix_segmentors: Vec::new(),
            matcher_segmentor: None,
            fallback_segmentor_enabled: false,
            remaining_gear_deferrals: Vec::new(),
            paging: false,
            user_dict_name: None,
            last_active_time: session_activity_now(),
        }
    }

    pub(crate) fn set_user_dict_name(&mut self, dict_name: impl Into<String>) {
        self.user_dict_name = Some(dict_name.into());
    }

    pub(crate) fn clear_user_dict_name(&mut self) {
        self.user_dict_name = None;
        self.reload_userdb_from_store();
    }

    pub(crate) fn reload_userdb_from_store(&mut self) {
        let Some(dict_name) = self.user_dict_name.as_deref() else {
            self.engine.set_userdb(Default::default());
            return;
        };
        match userdb::load_runtime_userdb(dict_name) {
            Ok(userdb) => self.engine.set_userdb(userdb),
            Err(_) => self.engine.set_userdb(Default::default()),
        }
    }

    pub(crate) fn persist_pending_userdb_learning(&mut self) {
        let Some(event) = self.engine.take_pending_userdb_learning() else {
            return;
        };
        let Some(dict_name) = self.user_dict_name.as_deref() else {
            return;
        };
        if let Ok(userdb) = userdb::record_runtime_commit(dict_name, &event) {
            self.engine.set_userdb(userdb);
        }
    }

    fn activate(&mut self) {
        self.last_active_time = session_activity_now();
    }
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn sessions() -> &'static Mutex<SessionRegistry> {
    static SESSIONS: OnceLock<Mutex<SessionRegistry>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(SessionRegistry::default()))
}

pub(crate) fn service_started() -> &'static AtomicBool {
    static SERVICE_STARTED: AtomicBool = AtomicBool::new(false);
    &SERVICE_STARTED
}

#[no_mangle]
pub extern "C" fn RimeCreateSession() -> RimeSessionId {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .create_session()
}

#[no_mangle]
pub extern "C" fn RimeFindSession(session_id: RimeSessionId) -> Bool {
    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    bool_from(registry.find_session(session_id))
}

#[no_mangle]
pub extern "C" fn RimeDestroySession(session_id: RimeSessionId) -> Bool {
    bool_from(
        session_id != 0
            && sessions()
                .lock()
                .expect("session registry should not be poisoned")
                .sessions
                .remove(&session_id)
                .is_some(),
    )
}

#[no_mangle]
pub extern "C" fn RimeCleanupAllSessions() {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .sessions
        .clear();
}

#[no_mangle]
pub extern "C" fn RimeCleanupStaleSessions() {
    sessions()
        .lock()
        .expect("session registry should not be poisoned")
        .cleanup_stale_sessions();
}

pub(crate) fn with_session(
    session_id: RimeSessionId,
    action: impl FnOnce(&mut SessionState) -> bool,
) -> Bool {
    if session_id == 0 {
        return FALSE;
    }

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };

    bool_from(action(session))
}

pub(crate) fn session_candidates_snapshot(
    session_id: RimeSessionId,
) -> Option<Vec<yune_core::Candidate>> {
    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let session = registry.get_session_mut(session_id)?;
    Some(session.engine.context().candidates.clone())
}

pub(crate) fn session_inspector_snapshot(
    session_id: RimeSessionId,
) -> Option<(
    yune_core::EngineInspectorSnapshot,
    Vec<yune_core::Candidate>,
)> {
    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let session = registry.get_session_mut(session_id)?;
    Some((
        session.engine.inspector_snapshot(),
        session.engine.context().candidates.clone(),
    ))
}

#[cfg(test)]
pub(crate) fn remaining_gear_deferrals_snapshot(
    session_id: RimeSessionId,
) -> Option<Vec<RemainingGearDeferral>> {
    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let session = registry.get_session_mut(session_id)?;
    Some(session.remaining_gear_deferrals.clone())
}

pub(crate) fn session_activity_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
