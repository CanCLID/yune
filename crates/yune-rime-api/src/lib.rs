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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeComposition {
    pub length: c_int,
    pub cursor_pos: c_int,
    pub sel_start: c_int,
    pub sel_end: c_int,
    pub preedit: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeCandidate {
    pub text: *mut c_char,
    pub comment: *mut c_char,
    pub reserved: *mut std::ffi::c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeMenu {
    pub page_size: c_int,
    pub page_no: c_int,
    pub is_last_page: Bool,
    pub highlighted_candidate_index: c_int,
    pub num_candidates: c_int,
    pub candidates: *mut RimeCandidate,
    pub select_keys: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RimeContext {
    pub data_size: c_int,
    pub composition: RimeComposition,
    pub menu: RimeMenu,
    pub commit_text_preview: *mut c_char,
    pub select_labels: *mut *mut c_char,
}

const XK_BACKSPACE: c_int = 0xff08;
const XK_RETURN: c_int = 0xff0d;
const DEFAULT_PAGE_SIZE: usize = 5;

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

/// Copies the current composition and first candidate page into caller storage.
///
/// # Safety
///
/// `context` must be either null or a valid, writable pointer to a
/// `RimeContext` initialized with a positive `data_size`. When this function
/// returns `TRUE`, the caller must release nested strings and candidate memory
/// by passing the same context object to `RimeFreeContext`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetContext(
    session_id: RimeSessionId,
    context: *mut RimeContext,
) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    clear_context(context);

    let registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.sessions.get(&session_id) else {
        return FALSE;
    };

    let snapshot = session.engine.snapshot();
    let composition = snapshot.context.composition;
    if !composition.input.is_empty() {
        let Ok(preedit) = CString::new(composition.preedit) else {
            return FALSE;
        };
        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `preedit` is converted into owned C storage for the caller.
        unsafe {
            (*context).composition.length = composition.input.len() as c_int;
            (*context).composition.cursor_pos = composition.caret as c_int;
            (*context).composition.sel_start = 0;
            (*context).composition.sel_end = composition.input.len() as c_int;
            (*context).composition.preedit = preedit.into_raw();
        }
    }

    let candidates = snapshot.context.candidates;
    if !candidates.is_empty() {
        let highlighted = snapshot.context.highlighted;
        let page_no = highlighted / DEFAULT_PAGE_SIZE;
        let page_start = page_no * DEFAULT_PAGE_SIZE;
        let page_end = (page_start + DEFAULT_PAGE_SIZE).min(candidates.len());
        let page_candidates = &candidates[page_start..page_end];

        let mut rime_candidates = Vec::with_capacity(page_candidates.len());
        for candidate in page_candidates {
            let Ok(text) = CString::new(candidate.text.as_str()) else {
                free_rime_candidates(&mut rime_candidates);
                return FALSE;
            };
            let comment = if candidate.comment.is_empty() {
                ptr::null_mut()
            } else {
                let Ok(comment) = CString::new(candidate.comment.as_str()) else {
                    free_rime_candidates(&mut rime_candidates);
                    return FALSE;
                };
                comment.into_raw()
            };
            rime_candidates.push(RimeCandidate {
                text: text.into_raw(),
                comment,
                reserved: ptr::null_mut(),
            });
        }

        let num_candidates = rime_candidates.len();
        let candidates_ptr = rime_candidates.as_mut_ptr();
        std::mem::forget(rime_candidates);

        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `candidates_ptr` owns `num_candidates` initialized entries.
        unsafe {
            (*context).menu.page_size = DEFAULT_PAGE_SIZE as c_int;
            (*context).menu.page_no = page_no as c_int;
            (*context).menu.is_last_page = bool_from(page_end == candidates.len());
            (*context).menu.highlighted_candidate_index =
                (highlighted - page_start).min(num_candidates.saturating_sub(1)) as c_int;
            (*context).menu.num_candidates = num_candidates as c_int;
            (*context).menu.candidates = candidates_ptr;
        }
    }

    TRUE
}

/// Frees nested allocations populated by `RimeGetContext`.
///
/// # Safety
///
/// `context` must be either null or a valid, writable pointer to a
/// `RimeContext`. Nested pointers, when non-null, must have been returned by
/// `RimeGetContext` and not already freed.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeContext(context: *mut RimeContext) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    free_context_fields(context);
    clear_context(context);
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

fn clear_context(context: *mut RimeContext) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size`.
    unsafe {
        (*context).composition = RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        };
        (*context).menu = RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        };
        (*context).commit_text_preview = ptr::null_mut();
        (*context).select_labels = ptr::null_mut();
    }
}

fn free_context_fields(context: *mut RimeContext) {
    // SAFETY: `context` is non-null and nested pointers are owned by this API
    // when populated by `RimeGetContext`.
    unsafe {
        if !(*context).composition.preedit.is_null() {
            drop(CString::from_raw((*context).composition.preedit));
        }
        if !(*context).menu.candidates.is_null() && (*context).menu.num_candidates > 0 {
            let num_candidates = (*context).menu.num_candidates as usize;
            let mut candidates =
                Vec::from_raw_parts((*context).menu.candidates, num_candidates, num_candidates);
            free_rime_candidates(&mut candidates);
        }
        if !(*context).menu.select_keys.is_null() {
            drop(CString::from_raw((*context).menu.select_keys));
        }
        if !(*context).commit_text_preview.is_null() {
            drop(CString::from_raw((*context).commit_text_preview));
        }
        if !(*context).select_labels.is_null() {
            let page_size = (*context).menu.page_size.max(0) as usize;
            let labels = Vec::from_raw_parts((*context).select_labels, page_size, page_size);
            for label in labels {
                if !label.is_null() {
                    drop(CString::from_raw(label));
                }
            }
        }
    }
}

fn free_rime_candidates(candidates: &mut Vec<RimeCandidate>) {
    for candidate in candidates.drain(..) {
        if !candidate.text.is_null() {
            // SAFETY: candidate text pointers were returned by CString::into_raw
            // while populating a RimeContext.
            unsafe {
                drop(CString::from_raw(candidate.text));
            }
        }
        if !candidate.comment.is_null() {
            // SAFETY: candidate comment pointers were returned by
            // CString::into_raw while populating a RimeContext.
            unsafe {
                drop(CString::from_raw(candidate.comment));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CStr;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    use super::{
        bool_from, RimeCleanupAllSessions, RimeCommit, RimeContext, RimeCreateSession,
        RimeDestroySession, RimeFindSession, RimeFreeCommit, RimeFreeContext, RimeGetCommit,
        RimeGetContext, RimeProcessKey, FALSE, TRUE,
    };

    fn test_guard() -> MutexGuard<'static, ()> {
        static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("test lock should not be poisoned")
    }

    fn empty_context() -> RimeContext {
        RimeContext {
            data_size: (std::mem::size_of::<RimeContext>() - std::mem::size_of::<i32>()) as i32,
            composition: super::RimeComposition {
                length: 0,
                cursor_pos: 0,
                sel_start: 0,
                sel_end: 0,
                preedit: std::ptr::null_mut(),
            },
            menu: super::RimeMenu {
                page_size: 0,
                page_no: 0,
                is_last_page: FALSE,
                highlighted_candidate_index: 0,
                num_candidates: 0,
                candidates: std::ptr::null_mut(),
                select_keys: std::ptr::null_mut(),
            },
            commit_text_preview: std::ptr::null_mut(),
            select_labels: std::ptr::null_mut(),
        }
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
    fn returns_context_with_preedit_and_candidate_page() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut context = empty_context();

        assert_eq!(RimeProcessKey(session_id, 'n' as i32, 0), TRUE);
        assert_eq!(RimeProcessKey(session_id, 'i' as i32, 0), TRUE);

        // SAFETY: `context` points to valid writable storage initialized with a
        // positive `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        assert_eq!(context.composition.length, 2);
        assert_eq!(context.composition.cursor_pos, 2);
        assert_eq!(context.composition.sel_start, 0);
        assert_eq!(context.composition.sel_end, 2);
        // SAFETY: `RimeGetContext` returned true and populated owned C strings.
        let preedit = unsafe { CStr::from_ptr(context.composition.preedit) };
        assert_eq!(preedit.to_str(), Ok("ni"));

        assert_eq!(context.menu.page_size, 5);
        assert_eq!(context.menu.page_no, 0);
        assert_eq!(context.menu.is_last_page, TRUE);
        assert_eq!(context.menu.highlighted_candidate_index, 0);
        assert_eq!(context.menu.num_candidates, 1);
        assert!(!context.menu.candidates.is_null());
        // SAFETY: `context.menu.candidates` points to one initialized candidate.
        let candidate = unsafe { *context.menu.candidates };
        // SAFETY: candidate strings are valid NUL-terminated strings owned by
        // the context object.
        let candidate_text = unsafe { CStr::from_ptr(candidate.text) };
        assert_eq!(candidate_text.to_str(), Ok("ni"));
        // SAFETY: the echo candidate includes a non-null comment.
        let candidate_comment = unsafe { CStr::from_ptr(candidate.comment) };
        assert_eq!(candidate_comment.to_str(), Ok("echo"));

        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        assert!(context.composition.preedit.is_null());
        assert!(context.menu.candidates.is_null());
        assert_eq!(context.menu.num_candidates, 0);

        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    #[test]
    fn rejects_invalid_context_requests() {
        let _guard = test_guard();
        RimeCleanupAllSessions();
        let session_id = RimeCreateSession();
        let mut context = empty_context();
        context.data_size = 0;

        // SAFETY: `context` points to writable storage but has invalid
        // librime-style data_size metadata.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, FALSE);
        // SAFETY: null pointers are explicitly rejected.
        assert_eq!(
            unsafe { RimeGetContext(session_id, std::ptr::null_mut()) },
            FALSE
        );
        // SAFETY: `context` points to writable storage but has invalid
        // librime-style data_size metadata.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, FALSE);

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
