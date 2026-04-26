use std::{
    collections::HashMap,
    ffi::CString,
    os::raw::{c_char, c_int},
    ptr,
    sync::{Mutex, OnceLock},
};

use yune_core::{Engine, KeyCode, KeyEvent, KeyModifiers};

pub type RimeSessionId = usize;
pub type Bool = c_int;

pub const FALSE: Bool = 0;
pub const TRUE: Bool = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeTraits {
    pub data_size: c_int,
    pub shared_data_dir: *const c_char,
    pub user_data_dir: *const c_char,
    pub distribution_name: *const c_char,
    pub distribution_code_name: *const c_char,
    pub distribution_version: *const c_char,
    pub app_name: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCommit {
    pub data_size: c_int,
    pub text: *mut c_char,
}

const XK_BACKSPACE: c_int = 0xff08;
const XK_RETURN: c_int = 0xff0d;

#[derive(Default)]
struct SessionRegistry {
    next_id: RimeSessionId,
    sessions: HashMap<RimeSessionId, SessionState>,
}

impl SessionRegistry {
    fn create_session(&mut self) -> RimeSessionId {
        self.next_id = self.next_id.saturating_add(1).max(1);
        let session_id = self.next_id;
        self.sessions.insert(session_id, SessionState::default());
        session_id
    }
}

#[derive(Default)]
struct SessionState {
    engine: Engine,
    unread_commit: Option<String>,
}

fn sessions() -> &'static Mutex<SessionRegistry> {
    static SESSIONS: OnceLock<Mutex<SessionRegistry>> = OnceLock::new();
    SESSIONS.get_or_init(|| Mutex::new(SessionRegistry::default()))
}

#[must_use]
pub const fn bool_from(value: bool) -> Bool {
    if value {
        TRUE
    } else {
        FALSE
    }
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
    bool_from(
        session_id != 0
            && sessions()
                .lock()
                .expect("session registry should not be poisoned")
                .sessions
                .contains_key(&session_id),
    )
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
pub extern "C" fn RimeProcessKey(session_id: RimeSessionId, keycode: c_int, mask: c_int) -> Bool {
    if session_id == 0 || mask != 0 {
        return FALSE;
    }
    let Some(key_event) = key_event_from_rime_keycode(keycode) else {
        return FALSE;
    };

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };

    let was_composing = !session.engine.context().composition.input.is_empty();
    if let Some(commit) = session.engine.process_key_event(key_event) {
        session.unread_commit = Some(commit);
        return TRUE;
    }

    bool_from(matches!(key_event.code, KeyCode::Character(ch) if ch != ' ') || was_composing)
}

/// Copies the unread commit text for a session into a caller-provided commit.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// When this function returns `TRUE`, the caller must release `commit.text` by
/// passing the same commit object to `RimeFreeCommit`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCommit(session_id: RimeSessionId, commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }

    clear_commit(commit);

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get_mut(&session_id) else {
        return FALSE;
    };
    let Some(text) = session.unread_commit.take() else {
        return FALSE;
    };
    let Ok(text) = CString::new(text) else {
        return FALSE;
    };

    // SAFETY: `commit` is non-null and points to caller-owned writable storage.
    unsafe {
        (*commit).data_size = std::mem::size_of::<RimeCommit>() as c_int;
        (*commit).text = text.into_raw();
    }
    TRUE
}

/// Frees a commit object populated by `RimeGetCommit`.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// If `commit.text` is non-null, it must be a pointer previously returned by
/// `RimeGetCommit` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeCommit(commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }

    // SAFETY: `commit` is non-null and any non-null `text` pointer is owned by
    // this API because it was returned from `CString::into_raw` in `RimeGetCommit`.
    unsafe {
        if !(*commit).text.is_null() {
            drop(CString::from_raw((*commit).text));
        }
    }
    clear_commit(commit);
    TRUE
}

fn key_event_from_rime_keycode(keycode: c_int) -> Option<KeyEvent> {
    let code = match keycode {
        XK_BACKSPACE => KeyCode::Backspace,
        XK_RETURN => KeyCode::Return,
        0x20..=0x7e => KeyCode::Character(char::from_u32(keycode as u32)?),
        _ => return None,
    };

    Some(KeyEvent {
        code,
        modifiers: KeyModifiers::default(),
    })
}

fn clear_commit(commit: *mut RimeCommit) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain integers/pointers and assigning null mirrors librime's clear macro.
    unsafe {
        (*commit).data_size = 0;
        (*commit).text = ptr::null_mut();
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use super::{
        bool_from, RimeCleanupAllSessions, RimeCommit, RimeCreateSession, RimeDestroySession,
        RimeFindSession, RimeFreeCommit, RimeGetCommit, RimeProcessKey, FALSE, TRUE,
    };

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("test lock should not be poisoned")
    }

    #[test]
    fn maps_bool_to_rime_bool() {
        assert_eq!(bool_from(true), TRUE);
        assert_eq!(bool_from(false), FALSE);
    }

    #[test]
    fn creates_finds_and_destroys_sessions() {
        let _guard = test_guard();
        RimeCleanupAllSessions();

        let session_id = RimeCreateSession();

        assert_ne!(session_id, 0);
        assert_eq!(RimeFindSession(session_id), TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        assert_eq!(RimeFindSession(session_id), FALSE);
    }

    #[test]
    fn processes_ascii_keys_and_returns_unread_commit_once() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut commit = RimeCommit {
            data_size: 0,
            text: std::ptr::null_mut(),
        };

        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);
        assert_eq!(RimeProcessKey(session_id, ' ' as i32, 0), TRUE);
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, TRUE);
        // SAFETY: `RimeGetCommit` returned true and populated `text` with a
        // valid NUL-terminated C string owned by the commit object.
        let text = unsafe { CStr::from_ptr(commit.text) };
        assert_eq!(text.to_str(), Ok("ni"));
        // SAFETY: `commit.text` was returned by `RimeGetCommit` above.
        assert_eq!(unsafe { RimeFreeCommit(&mut commit) }, TRUE);
        assert!(commit.text.is_null());
        // SAFETY: `commit` points to valid writable storage for this test.
        assert_eq!(unsafe { RimeGetCommit(session_id, &mut commit) }, FALSE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn rejects_unknown_sessions_and_modified_keys() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();

        assert_eq!(RimeProcessKey(0, 'a' as i32, 0), FALSE);
        assert_eq!(RimeProcessKey(session_id + 1, 'a' as i32, 0), FALSE);
        assert_eq!(RimeProcessKey(session_id, 'a' as i32, 1), FALSE);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }
}
