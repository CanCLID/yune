use std::{
    ffi::{CStr, CString},
    mem, ptr,
    sync::{Mutex, MutexGuard, OnceLock},
};

use yune_rime_api::{
    rime_get_api, RimeCandidate, RimeCandidateListIterator, RimeCommit, RimeComposition,
    RimeContext, RimeMenu, RimeStatus, FALSE, TRUE,
};

fn empty_context() -> RimeContext {
    RimeContext {
        data_size: (mem::size_of::<RimeContext>() - mem::size_of::<i32>()) as i32,
        composition: RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        },
        menu: RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        },
        commit_text_preview: ptr::null_mut(),
        select_labels: ptr::null_mut(),
    }
}

fn empty_status() -> RimeStatus {
    RimeStatus {
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<i32>()) as i32,
        schema_id: ptr::null_mut(),
        schema_name: ptr::null_mut(),
        is_disabled: FALSE,
        is_composing: FALSE,
        is_ascii_mode: FALSE,
        is_full_shape: FALSE,
        is_simplified: FALSE,
        is_traditional: FALSE,
        is_ascii_punct: FALSE,
    }
}

fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<i32>()) as i32,
        text: ptr::null_mut(),
    }
}

fn empty_candidate_list_iterator() -> RimeCandidateListIterator {
    RimeCandidateListIterator {
        ptr: ptr::null_mut(),
        index: 0,
        candidate: RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        },
    }
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned")
}

#[test]
fn frontend_style_api_table_can_drive_basic_composition_flow() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    assert_eq!(
        api.data_size,
        (mem::size_of_val(api) - mem::size_of::<i32>()) as i32
    );

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let find_session = api.find_session.expect("frontend requires find_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_status = api.get_status.expect("frontend requires get_status");
    let free_status = api.free_status.expect("frontend requires free_status");
    let get_context = api.get_context.expect("frontend requires get_context");
    let free_context = api.free_context.expect("frontend requires free_context");
    let select_candidate_on_current_page = api
        .select_candidate_on_current_page
        .expect("frontend requires select_candidate_on_current_page");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(find_session(session_id), TRUE);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let input = get_input(session_id);
    assert!(!input.is_null());
    let input = unsafe { CStr::from_ptr(input) };
    assert_eq!(input.to_str(), Ok("ni"));

    let mut status = empty_status();
    assert_eq!(unsafe { get_status(session_id, &mut status) }, TRUE);
    assert_eq!(status.is_composing, TRUE);
    let schema_id = unsafe { CStr::from_ptr(status.schema_id) };
    assert_eq!(schema_id.to_str(), Ok("default"));
    assert_eq!(unsafe { free_status(&mut status) }, TRUE);

    let mut context = empty_context();
    assert_eq!(unsafe { get_context(session_id, &mut context) }, TRUE);
    assert_eq!(context.composition.length, 2);
    assert_eq!(context.menu.page_size, 5);
    assert_eq!(context.menu.num_candidates, 1);
    assert_eq!(context.menu.highlighted_candidate_index, 0);
    let first_candidate = unsafe { *context.menu.candidates };
    let first_candidate_text = unsafe { CStr::from_ptr(first_candidate.text) };
    assert_eq!(first_candidate_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_context(&mut context) }, TRUE);

    assert_eq!(select_candidate_on_current_page(session_id, 0), TRUE);

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, FALSE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_simulate_key_sequences() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let simulate_key_sequence = api
        .simulate_key_sequence
        .expect("frontend requires simulate_key_sequence");
    let get_commit = api.get_commit.expect("frontend requires get_commit");
    let free_commit = api.free_commit.expect("frontend requires free_commit");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let sequence = CString::new("ni{space}").expect("literal should not contain NUL");
    assert_eq!(
        unsafe { simulate_key_sequence(session_id, sequence.as_ptr()) },
        TRUE
    );

    let mut commit = empty_commit();
    assert_eq!(unsafe { get_commit(session_id, &mut commit) }, TRUE);
    let commit_text = unsafe { CStr::from_ptr(commit.text) };
    assert_eq!(commit_text.to_str(), Ok("ni"));
    assert_eq!(unsafe { free_commit(&mut commit) }, TRUE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_edit_input_and_caret() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let get_version = api.get_version.expect("frontend requires get_version");
    let version = get_version();
    assert!(!version.is_null());
    let version = unsafe { CStr::from_ptr(version) };
    assert_eq!(version.to_str(), Ok("yune-rime-api 0.1.0"));

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let get_input = api.get_input.expect("frontend requires get_input");
    let get_caret_pos = api.get_caret_pos.expect("frontend requires get_caret_pos");
    let set_caret_pos = api.set_caret_pos.expect("frontend requires set_caret_pos");
    let set_input = api.set_input.expect("frontend requires set_input");

    assert!(get_input(0).is_null());
    assert_eq!(get_caret_pos(0), 0);

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let input = CString::new("nihao").expect("literal should not contain NUL");
    assert_eq!(unsafe { set_input(session_id, input.as_ptr()) }, TRUE);
    assert_eq!(get_caret_pos(session_id), 5);

    let current_input = get_input(session_id);
    assert!(!current_input.is_null());
    let current_input = unsafe { CStr::from_ptr(current_input) };
    assert_eq!(current_input.to_str(), Ok("nihao"));

    set_caret_pos(session_id, 2);
    assert_eq!(get_caret_pos(session_id), 2);
    set_caret_pos(session_id, 99);
    assert_eq!(get_caret_pos(session_id), 5);

    assert_eq!(unsafe { set_input(session_id, ptr::null()) }, FALSE);
    assert_eq!(unsafe { set_input(session_id + 1, input.as_ptr()) }, FALSE);

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_iterate_candidates() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let candidate_list_begin = api
        .candidate_list_begin
        .expect("frontend requires candidate_list_begin");
    let candidate_list_next = api
        .candidate_list_next
        .expect("frontend requires candidate_list_next");
    let candidate_list_end = api
        .candidate_list_end
        .expect("frontend requires candidate_list_end");

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let mut iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_begin(session_id, &mut iterator) },
        TRUE
    );
    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, TRUE);

    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    let comment = unsafe { CStr::from_ptr(iterator.candidate.comment) };
    assert_eq!(comment.to_str(), Ok("echo"));

    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, FALSE);
    assert_eq!(iterator.index, 1);
    let preserved_text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(preserved_text.to_str(), Ok("ni"));

    unsafe { candidate_list_end(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert!(iterator.candidate.text.is_null());
    assert!(iterator.candidate.comment.is_null());

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}

#[test]
fn frontend_style_api_table_can_iterate_candidates_from_index() {
    let _guard = test_guard();
    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };

    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    cleanup_all_sessions();

    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let process_key = api.process_key.expect("frontend requires process_key");
    let candidate_list_from_index = api
        .candidate_list_from_index
        .expect("frontend requires candidate_list_from_index");
    let candidate_list_next = api
        .candidate_list_next
        .expect("frontend requires candidate_list_next");
    let candidate_list_end = api
        .candidate_list_end
        .expect("frontend requires candidate_list_end");

    let session_id = create_session();
    assert_ne!(session_id, 0);

    let mut empty_iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut empty_iterator, 0) },
        FALSE
    );
    assert!(empty_iterator.ptr.is_null());

    assert_eq!(process_key(session_id, 'n' as i32, 0), TRUE);
    assert_eq!(process_key(session_id, 'i' as i32, 0), TRUE);

    let mut iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut iterator, 0) },
        TRUE
    );
    assert_eq!(iterator.index, -1);
    assert_eq!(unsafe { candidate_list_next(&mut iterator) }, TRUE);

    let text = unsafe { CStr::from_ptr(iterator.candidate.text) };
    assert_eq!(text.to_str(), Ok("ni"));
    unsafe { candidate_list_end(&mut iterator) };

    let mut past_end_iterator = empty_candidate_list_iterator();
    assert_eq!(
        unsafe { candidate_list_from_index(session_id, &mut past_end_iterator, 1) },
        TRUE
    );
    assert_eq!(past_end_iterator.index, 0);
    assert_eq!(
        unsafe { candidate_list_next(&mut past_end_iterator) },
        FALSE
    );
    assert_eq!(past_end_iterator.index, 1);
    assert!(past_end_iterator.candidate.text.is_null());
    unsafe { candidate_list_end(&mut past_end_iterator) };

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
}
