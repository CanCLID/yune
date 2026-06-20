use std::{
    ffi::{CStr, CString},
    fs, mem,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use yune_rime_api::{
    rime_get_api, yune_typeduck_cleanup, yune_typeduck_customize, yune_typeduck_delete_candidate,
    yune_typeduck_deploy, yune_typeduck_flip_page, yune_typeduck_free_response, yune_typeduck_init,
    yune_typeduck_process_key, yune_typeduck_response_handled, yune_typeduck_response_json,
    yune_typeduck_select_candidate, yune_typeduck_set_ai_enabled, yune_typeduck_set_option,
    yune_typeduck_stage_ai, Bool, RimeStringSlice, RimeTraits, FALSE, TRUE,
};

const SCHEMA_ID: &str = "typeduck_luna";
const TYPEDUCK_V112_COMMENTS: &str =
    include_str!("../../yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json");

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn empty_rime_traits() -> RimeTraits {
    RimeTraits {
        data_size: mem::size_of::<RimeTraits>() as i32,
        shared_data_dir: ptr::null(),
        user_data_dir: ptr::null(),
        distribution_name: ptr::null(),
        distribution_code_name: ptr::null(),
        distribution_version: ptr::null(),
        app_name: ptr::null(),
        modules: ptr::null(),
        min_log_level: 0,
        log_dir: ptr::null(),
        prebuilt_data_dir: ptr::null(),
        staging_dir: ptr::null(),
    }
}

fn state_label_text(label: *const std::os::raw::c_char) -> String {
    assert!(!label.is_null());
    unsafe { CStr::from_ptr(label) }
        .to_str()
        .expect("state label should be valid UTF-8")
        .to_owned()
}

fn state_label_slice(label: RimeStringSlice) -> String {
    assert!(!label.str.is_null());
    let bytes = unsafe { std::slice::from_raw_parts(label.str.cast::<u8>(), label.length) };
    std::str::from_utf8(bytes)
        .expect("state label slice should be valid UTF-8")
        .to_owned()
}

#[test]
fn typeduck_adapter_processes_keys_and_returns_json_state() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("process-json-state");
    runtime.write_schema();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let first = response_json(unsafe { yune_typeduck_process_key(state, 'b' as i32, 0) });
    assert_eq!(first["handled"], Value::Bool(true));
    assert_eq!(first["context"]["input"], Value::String("b".to_owned()));
    assert_eq!(
        first["status"]["schema_id"],
        Value::String(SCHEMA_ID.to_owned())
    );
    assert_eq!(first["status"]["is_composing"], Value::Bool(true));

    let second = response_json(unsafe { yune_typeduck_process_key(state, 'a' as i32, 0) });
    assert_eq!(second["handled"], Value::Bool(true));
    assert_eq!(second["context"]["input"], Value::String("ba".to_owned()));
    assert_eq!(second["context"]["page_size"], Value::from(2));
    assert_eq!(
        second["context"]["select_keys"],
        Value::String("AB".to_owned())
    );
    assert_eq!(
        second["context"]["select_labels"][0],
        Value::String("Alpha".to_owned())
    );
    assert_eq!(
        second["context"]["candidates"][0]["text"],
        Value::String("八".to_owned())
    );
    assert_eq!(
        second["context"]["candidates"][1]["text"],
        Value::String("吧".to_owned())
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_supports_page_candidate_actions_and_commits() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("candidate-actions");
    runtime.write_schema();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    drop(response_json(unsafe {
        yune_typeduck_process_key(state, 'b' as i32, 0)
    }));
    let composing = response_json(unsafe { yune_typeduck_process_key(state, 'a' as i32, 0) });
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("八".to_owned())
    );

    let next_page = response_json(unsafe { yune_typeduck_flip_page(state, FALSE) });
    assert_eq!(next_page["handled"], Value::Bool(true));
    assert_eq!(next_page["context"]["page_no"], Value::from(1));
    assert_eq!(
        next_page["context"]["candidates"][0]["text"],
        Value::String("爸".to_owned())
    );

    let previous_page = response_json(unsafe { yune_typeduck_flip_page(state, TRUE) });
    assert_eq!(previous_page["handled"], Value::Bool(true));
    assert_eq!(previous_page["context"]["page_no"], Value::from(0));

    let deleted = response_json(unsafe { yune_typeduck_delete_candidate(state, 0) });
    assert_eq!(deleted["handled"], Value::Bool(true));
    assert_eq!(
        deleted["context"]["candidates"][0]["text"],
        Value::String("吧".to_owned())
    );

    let selected = response_json(unsafe { yune_typeduck_select_candidate(state, 0) });
    assert_eq!(selected["handled"], Value::Bool(true));
    assert_eq!(
        selected["commits"],
        Value::Array(vec![Value::String("吧".to_owned())])
    );
    assert_eq!(selected["status"]["is_composing"], Value::Bool(false));

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_documents_browser_host_layout_constraints() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("browser-host-layout");

    assert!(runtime.shared.exists());
    assert!(runtime.user.exists());
    assert!(
        runtime.user.join("build").exists(),
        "browser host fixture must create user_data_dir/build before init"
    );

    let state_without_preloaded_assets = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        state_without_preloaded_assets.is_null(),
        "init without preloaded schema/dictionary assets must fail deterministically"
    );

    runtime.write_schema_with_dictionary("typeduck");
    runtime.write_dictionary("stray");
    let state_with_wrong_dictionary = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        state_with_wrong_dictionary.is_null(),
        "init must reject preloads that omit the selected schema dictionary"
    );

    let path_like_schema_id = CString::new("../typeduck_luna").expect("schema id should be valid");
    let state_with_path_like_schema_id = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            path_like_schema_id.as_ptr(),
        )
    };
    assert!(
        state_with_path_like_schema_id.is_null(),
        "init must reject path-like schema ids before probing assets"
    );

    runtime.write_dictionary("typeduck");
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let response = unsafe { yune_typeduck_process_key(state, 'b' as i32, 0) };
    let json = unsafe { yune_typeduck_response_json(response) };
    assert!(!json.is_null());
    let text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("adapter JSON should be valid UTF-8")
        .to_owned();
    unsafe { yune_typeduck_free_response(response) };
    let value: Value = serde_json::from_str(&text).expect("copied response should parse as JSON");
    assert_eq!(value["handled"], Value::Bool(true));

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_accepts_deployed_schema_dictionary_for_inherited_source_schema() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("deployed-dictionary");
    runtime.write_source_schema_with_deployed_dictionary("typeduck");
    runtime.write_dictionary("typeduck");

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        !state.is_null(),
        "init should accept browser preloads where the source schema inherits the dictionary and the deployed schema resolves it"
    );

    drop(response_json(unsafe {
        yune_typeduck_process_key(state, 'b' as i32, 0)
    }));
    let composing = response_json(unsafe { yune_typeduck_process_key(state, 'a' as i32, 0) });
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{516b}".to_owned())
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_composes_source_dictionary_with_mobile_schema_algebra() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("source-dictionary-mobile-algebra");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    drop(response_json(unsafe {
        yune_typeduck_process_key(state, 'n' as i32, 0)
    }));
    drop(response_json(unsafe {
        yune_typeduck_process_key(state, 'e' as i32, 0)
    }));
    let composing = response_json(unsafe { yune_typeduck_process_key(state, 'i' as i32, 0) });
    assert_eq!(
        composing["context"]["input"],
        Value::String("nei".to_owned())
    );
    assert_eq!(composing["context"]["select_keys"], Value::Null);
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_customized_sentence_mode_commits_multisyllable_phrase() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("mobile-sentence-customize");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_sentence_dictionary();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("typeduck_luna.schema").expect("config id should be valid");
    let key =
        CString::new("translator/enable_sentence").expect("custom key should be valid CString");
    let value = CString::new("true").expect("custom value should be valid CString");
    assert_eq!(
        unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);
    let deployed_schema: Value = serde_yaml::from_str(
        &fs::read_to_string(runtime.user.join("build").join("typeduck_luna.schema.yaml"))
            .expect("deployed sentence fixture schema should be readable"),
    )
    .expect("deployed sentence fixture schema should parse");
    assert_eq!(
        deployed_schema
            .pointer("/translator/enable_sentence")
            .and_then(config_bool_like),
        Some(true)
    );

    let mut composing = Value::Null;
    for key in "ngohaigo".chars() {
        composing = response_json(unsafe { yune_typeduck_process_key(state, key as i32, 0) });
    }
    assert_eq!(
        composing["context"]["input"],
        Value::String("ngohaigo".to_owned())
    );
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())
    );
    let has_raw_echo_candidate = composing["context"]["candidates"]
        .as_array()
        .expect("candidate list should be an array")
        .iter()
        .any(|candidate| {
            candidate["text"] == Value::String("ngohaig".to_owned())
                && candidate["comment"] == Value::String("echo".to_owned())
        });
    assert!(
        !has_raw_echo_candidate,
        "schemas without echo_translator must not leak a raw echo candidate for ngohaig"
    );

    let committed = response_json(unsafe { yune_typeduck_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    let mut composing = Value::Null;
    for key in "ngohaig".chars() {
        composing = response_json(unsafe { yune_typeduck_process_key(state, key as i32, 0) });
    }
    assert_eq!(
        composing["context"]["input"],
        Value::String("ngohaig".to_owned())
    );
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())
    );
    let has_raw_echo_candidate = composing["context"]["candidates"]
        .as_array()
        .expect("candidate list should be an array")
        .iter()
        .any(|candidate| {
            candidate["text"] == Value::String("ngohaig".to_owned())
                && candidate["comment"] == Value::String("echo".to_owned())
        });
    assert!(
        !has_raw_echo_candidate,
        "TypeDuck real-assets schema does not declare echo_translator, so ngohaig must not leak a raw echo candidate"
    );

    let committed = response_json(unsafe { yune_typeduck_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_deploy_and_customize_are_explicit() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("deploy-customize");
    runtime.write_schema();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);
    let config_id = CString::new("typeduck_luna.schema").expect("config id should be valid");
    let key = CString::new("schema/name").expect("custom key should be valid");
    let value = CString::new("TypeDuck Luna Web").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    let saved = fs::read_to_string(runtime.user.join("typeduck_luna.custom.yaml"))
        .expect("customized schema patch should be saved");
    assert!(saved.contains("schema/name"));
    assert!(saved.contains("TypeDuck Luna Web"));

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_set_option_updates_session_status() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("set-option");
    runtime.write_schema();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let ascii_mode = CString::new("ascii_mode").expect("option should be valid");
    assert_eq!(
        unsafe { yune_typeduck_set_option(state, ascii_mode.as_ptr(), TRUE) },
        TRUE
    );
    let ascii_enabled = response_json(unsafe { yune_typeduck_process_key(state, 'b' as i32, 0) });
    assert_eq!(ascii_enabled["status"]["is_ascii_mode"], Value::Bool(true));

    assert_eq!(
        unsafe { yune_typeduck_set_option(state, ascii_mode.as_ptr(), FALSE) },
        TRUE
    );
    let ascii_disabled = response_json(unsafe { yune_typeduck_process_key(state, 'a' as i32, 0) });
    assert_eq!(
        ascii_disabled["status"]["is_ascii_mode"],
        Value::Bool(false)
    );

    assert_eq!(
        unsafe { yune_typeduck_set_option(ptr::null_mut(), ascii_mode.as_ptr(), TRUE) },
        FALSE
    );
    assert_eq!(
        unsafe { yune_typeduck_set_option(state, ptr::null(), TRUE) },
        FALSE
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_stage_ai_is_default_off_and_second_pass_source_labeled() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("stage-ai-second-pass");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let classic = process_input(state, "nei");
    let classic_candidates = classic["context"]["candidates"]
        .as_array()
        .expect("classic candidates should be an array")
        .clone();
    assert_eq!(
        classic_candidates[0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    let default_off = response_json(unsafe { yune_typeduck_stage_ai(state) });
    assert_eq!(
        default_off["context"]["candidates"],
        Value::Array(classic_candidates.clone())
    );

    assert_eq!(unsafe { yune_typeduck_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_typeduck_stage_ai(state) });
    let staged_candidates = staged["context"]["candidates"]
        .as_array()
        .expect("staged candidates should be an array");
    assert_eq!(
        staged_candidates[0]["text"], classic_candidates[0]["text"],
        "classic top candidate must stay at index 0"
    );
    let ai_index = staged_candidates
        .iter()
        .position(|candidate| candidate["source"] == Value::String("ai:local".to_owned()))
        .expect("staged response should include a visible source-labeled AI candidate");
    assert!(
        staged_candidates.len() > classic_candidates.len(),
        "stage_ai should add a deterministic local AI row"
    );
    assert!(
        ai_index > 0,
        "AI rows must not displace the classic top candidate"
    );
    assert!(
        staged_candidates[..ai_index]
            .iter()
            .all(|candidate| candidate.get("source").is_none()),
        "classic rows before the AI row should remain unlabeled"
    );
    let ai_candidate = &staged_candidates[ai_index];
    assert_eq!(
        ai_candidate["text"],
        Value::String("\u{4f60}\u{554a}".to_owned())
    );
    let later_response = response_json(unsafe { yune_typeduck_flip_page(state, TRUE) });
    assert!(later_response["context"]["candidates"]
        .as_array()
        .expect("later response candidates should be an array")
        .iter()
        .any(|candidate| candidate["source"] == Value::String("ai:local".to_owned())));

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_disabling_ai_clears_staged_rows_for_current_input() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("stage-ai-disable-clears");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let classic = process_input(state, "nei");
    let classic_candidates = classic["context"]["candidates"].clone();
    assert_eq!(unsafe { yune_typeduck_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_typeduck_stage_ai(state) });
    assert!(staged["context"]["candidates"]
        .as_array()
        .expect("staged candidates should be an array")
        .iter()
        .any(|candidate| candidate["source"] == Value::String("ai:local".to_owned())));

    assert_eq!(unsafe { yune_typeduck_set_ai_enabled(state, FALSE) }, TRUE);
    let disabled = response_json(unsafe { yune_typeduck_stage_ai(state) });
    assert_eq!(disabled["context"]["candidates"], classic_candidates);
    assert!(
        disabled["context"]["candidates"]
            .as_array()
            .expect("disabled candidates should be an array")
            .iter()
            .all(|candidate| candidate.get("source").is_none()),
        "disabling AI must remove stale source-labeled rows immediately"
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_ai_rows_do_not_auto_commit_and_do_not_write_userdb() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("stage-ai-commit-safety");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let classic = process_input(state, "nei");
    let classic_top = classic["context"]["candidates"][0]["text"].clone();
    assert_eq!(unsafe { yune_typeduck_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_typeduck_stage_ai(state) });
    assert!(staged["context"]["candidates"]
        .as_array()
        .expect("staged candidates should be an array")
        .iter()
        .any(|candidate| candidate["source"] == Value::String("ai:local".to_owned())));

    let default_commit = response_json(unsafe { yune_typeduck_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        default_commit["commits"],
        Value::Array(vec![classic_top]),
        "Space/default confirm must commit the classic top row"
    );
    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();

    let runtime = TypeDuckRuntime::create("stage-ai-explicit-commit-safety");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
    process_input(state, "nei");
    assert_eq!(unsafe { yune_typeduck_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_typeduck_stage_ai(state) });
    let ai_index = staged["context"]["candidates"]
        .as_array()
        .expect("staged candidates should be an array")
        .iter()
        .position(|candidate| candidate["source"] == Value::String("ai:local".to_owned()))
        .expect("AI row should be selectable");
    let selected = response_json(unsafe { yune_typeduck_select_candidate(state, ai_index) });
    assert_eq!(
        selected["commits"],
        Value::Array(vec![Value::String("\u{4f60}\u{554a}".to_owned())])
    );
    assert!(
        !runtime.user.join("jyut6ping3.userdb").exists(),
        "explicit AI selection must not create or update the librime userdb"
    );
    assert!(
        !runtime.user.join("jyut6ping3.ai-memory").exists(),
        "sensitive browser default must suppress persisted AI-memory learning"
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_handles_null_inputs_and_response_freeing() {
    let _guard = test_guard();
    assert!(unsafe { yune_typeduck_init(ptr::null(), ptr::null(), ptr::null()) }.is_null());
    assert_eq!(unsafe { yune_typeduck_deploy(ptr::null_mut()) }, FALSE);
    assert_eq!(
        unsafe { yune_typeduck_customize(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null()) },
        FALSE
    );
    assert_eq!(
        unsafe { yune_typeduck_set_option(ptr::null_mut(), ptr::null(), TRUE) },
        FALSE
    );
    assert_eq!(
        unsafe { yune_typeduck_set_ai_enabled(ptr::null_mut(), TRUE) },
        FALSE
    );
    assert!(unsafe { yune_typeduck_process_key(ptr::null_mut(), 'a' as i32, 0) }.is_null());
    assert!(unsafe { yune_typeduck_stage_ai(ptr::null_mut()) }.is_null());
    assert!(unsafe { yune_typeduck_response_json(ptr::null()) }.is_null());
    assert_eq!(
        unsafe { yune_typeduck_response_handled(ptr::null()) },
        FALSE
    );
    unsafe { yune_typeduck_free_response(ptr::null_mut()) };
}

fn process_input(state: *mut yune_rime_api::YuneTypeDuckState, input: &str) -> Value {
    let mut response = Value::Null;
    for key in input.chars() {
        response = response_json(unsafe { yune_typeduck_process_key(state, key as i32, 0) });
    }
    response
}

fn response_json(response: *mut yune_rime_api::YuneTypeDuckResponse) -> Value {
    assert!(!response.is_null());
    let handled: Bool = unsafe { yune_typeduck_response_handled(response) };
    let json = unsafe { yune_typeduck_response_json(response) };
    assert!(!json.is_null());
    let text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("adapter JSON should be valid UTF-8")
        .to_owned();
    unsafe { yune_typeduck_free_response(response) };
    let value: Value = serde_json::from_str(&text).expect("adapter response should parse as JSON");
    assert_eq!(value["handled"].as_bool(), Some(handled == TRUE));
    value
}

fn config_bool_like(value: &Value) -> Option<bool> {
    value.as_bool().or_else(|| {
        value
            .as_str()
            .and_then(|value| match value.to_ascii_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            })
    })
}

struct TypeDuckRuntime {
    root: PathBuf,
    shared: PathBuf,
    user: PathBuf,
    shared_c: CString,
    user_c: CString,
    schema_id_c: CString,
}

impl TypeDuckRuntime {
    fn create(label: &str) -> Self {
        Self::create_with_schema(label, SCHEMA_ID)
    }

    fn create_with_schema(label: &str, schema_id: &str) -> Self {
        let root = unique_temp_dir(label);
        let shared = root.join("shared");
        let user = root.join("user");
        fs::create_dir_all(&shared).expect("shared dir should be created");
        fs::create_dir_all(user.join("build")).expect("staging dir should be created");
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
        let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
        let schema_id_c = CString::new(schema_id).expect("schema id should be valid");
        Self {
            root,
            shared,
            user,
            shared_c,
            user_c,
            schema_id_c,
        }
    }

    fn write_schema(&self) {
        self.write_schema_with_dictionary("typeduck");
        self.write_dictionary("typeduck");
    }

    fn write_schema_with_dictionary(&self, dictionary: &str) {
        let default_config =
            "config_version: typeduck-web\nschema_list:\n  - schema: typeduck_luna\n";
        let schema_config = format!(
            "\
schema:\n  schema_id: typeduck_luna\n  name: TypeDuck Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: {dictionary}\n"
        );
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("typeduck_luna.schema.yaml"), &schema_config)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("typeduck_luna.schema.yaml"), schema_config)
            .expect("shared schema config should be written");
    }

    fn write_source_schema_with_deployed_dictionary(&self, dictionary: &str) {
        let default_config =
            "config_version: typeduck-web\nschema_list:\n  - schema: typeduck_luna\n";
        let source_schema = "\
schema:\n  schema_id: typeduck_luna\n  name: TypeDuck Luna\n__include: template:/\n";
        let deployed_schema = format!(
            "\
schema:\n  schema_id: typeduck_luna\n  name: TypeDuck Luna\nmenu:\n  page_size: 2\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: {dictionary}\n"
        );
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("typeduck_luna.schema.yaml"), deployed_schema)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("typeduck_luna.schema.yaml"), source_schema)
            .expect("shared schema config should be written");
    }

    fn write_mobile_schema_with_dictionary(&self, dictionary: &str) {
        let default_config =
            "config_version: typeduck-web\nschema_list:\n  - schema: typeduck_luna\n";
        let schema_config = format!(
            "\
schema:\n  schema_id: typeduck_luna\n  name: TypeDuck Luna\nmenu:\n  page_size: 50\n  alternative_select_keys: \"\\x00\"\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  processors:\n    - speller\n    - express_editor\n  translators:\n    - script_translator\nspeller:\n  alphabet: zyxwvutsrqponmlkjihgfedcba\n  delimiter: \" '\"\n  algebra:\n    - \"derive/\\\\d//\"\ntranslator:\n  dictionary: {dictionary}\n  enable_completion: true\n  enable_sentence: false\n"
        );
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("typeduck_luna.schema.yaml"), &schema_config)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("typeduck_luna.schema.yaml"), schema_config)
            .expect("shared schema config should be written");
    }

    fn write_dictionary(&self, dictionary: &str) {
        fs::write(
            self.shared.join(format!("{dictionary}.dict.yaml")),
            format!(
                "\
---\nname: {dictionary}\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nba\t八\t10\nba\t吧\t9\nba\t爸\t8\nba\t巴\t7\nba\t把\t6\nba\t拔\t5\n"
            ),
        )
        .expect("dictionary should be written");
    }

    fn write_cantonese_dictionary(&self) {
        fs::write(
            self.shared.join("jyut6ping3.dict.yaml"),
            "---\nname: jyut6ping3\nversion: '1'\nsort: original\n...\n\n\u{4f60}\tnei5\t10\n\u{5462}\tnei1\t9\n",
        )
            .expect("dictionary should be written");
    }

    fn write_cantonese_sentence_dictionary(&self) {
        fs::write(
            self.shared.join("jyut6ping3.dict.yaml"),
            "---\nname: jyut6ping3\nversion: '1'\nsort: original\n...\n\n\u{6211}\tngo5\t10\n\u{4fc2}\thai6\t9\n\u{500b}\tgo3\t8\n\u{5605}\tge3\t7\n\u{5bb6}\tgaa1\t6\n",
        )
        .expect("dictionary should be written");
    }

    fn write_browser_real_assets(&self) {
        let oracle_root = typeduck_oracle_root();
        let oracle_schema_root = oracle_root.join("rime-shared");
        let schema_root = if oracle_schema_root.is_dir() {
            oracle_schema_root
        } else {
            browser_app_schema_root()
        };
        copy_asset(&schema_root, &self.shared, "default.yaml");
        copy_asset(&schema_root, &self.shared, "jyut6ping3_mobile.schema.yaml");
        copy_asset(&schema_root, &self.shared, "jyut6ping3.dict.yaml");
        let oracle_build_root = oracle_root.join("rime-user/build");
        let build_root = if oracle_build_root.is_dir() {
            oracle_build_root.clone()
        } else {
            schema_root.join("build")
        };
        let staging = self.user.join("build");
        for file_name in ["default.yaml", "jyut6ping3_mobile.schema.yaml"] {
            fs::copy(build_root.join(file_name), staging.join(file_name))
                .expect("browser preloaded build asset should be copied");
        }
        for file_name in [
            "default.custom.yaml",
            "common.yaml",
            "common.custom.yaml",
            "include.yaml",
            "template.yaml",
            "jyut6ping3.schema.yaml",
            "jyut6ping3_mobile_longpress.schema.yaml",
            "jyut6ping3_mobile_10keys.schema.yaml",
            "jyut6ping3_scolar.schema.yaml",
            "jyut6ping3_scolar.dict.yaml",
            "luna_pinyin.schema.yaml",
            "luna_pinyin.dict.yaml",
            "loengfan.schema.yaml",
            "loengfan.dict.yaml",
            "loengfan_longpress.schema.yaml",
            "cangjie3.schema.yaml",
            "cangjie3.dict.yaml",
            "cangjie5.schema.yaml",
            "cangjie5.dict.yaml",
            "opencc/hk2s.json",
            "opencc/HKVariantsRev.ocd2",
            "opencc/HKVariantsRevPhrases.ocd2",
            "opencc/TSCharacters.ocd2",
            "opencc/TSPhrases.ocd2",
        ] {
            copy_asset(&schema_root, &self.shared, file_name);
        }
        if oracle_build_root.is_dir() {
            for file_name in [
                "jyut6ping3.table.bin",
                "jyut6ping3.reverse.bin",
                "jyut6ping3_mobile.prism.bin",
                "jyut6ping3_scolar.table.bin",
                "jyut6ping3_scolar.reverse.bin",
                "jyut6ping3_scolar.prism.bin",
            ] {
                copy_asset(&oracle_build_root, &self.shared, file_name);
            }
        }
    }

    fn write_browser_app_assets(&self) {
        let schema_root = browser_app_schema_root();
        copy_asset(&schema_root, &self.shared, "default.yaml");
        copy_asset(&schema_root, &self.shared, "jyut6ping3_mobile.schema.yaml");
        copy_asset(&schema_root, &self.shared, "jyut6ping3.dict.yaml");
        let staging = self.user.join("build");
        for file_name in ["default.yaml", "jyut6ping3_mobile.schema.yaml"] {
            fs::copy(
                schema_root.join("build").join(file_name),
                staging.join(file_name),
            )
            .expect("browser app preloaded build asset should be copied");
        }
        for file_name in [
            "default.custom.yaml",
            "common.yaml",
            "common.custom.yaml",
            "include.yaml",
            "template.yaml",
            "jyut6ping3.schema.yaml",
            "jyut6ping3_mobile_longpress.schema.yaml",
            "jyut6ping3_mobile_10keys.schema.yaml",
            "jyut6ping3_scolar.schema.yaml",
            "jyut6ping3_scolar.dict.yaml",
            "luna_pinyin.schema.yaml",
            "luna_pinyin.dict.yaml",
            "loengfan.schema.yaml",
            "loengfan.dict.yaml",
            "loengfan_longpress.schema.yaml",
            "cangjie3.schema.yaml",
            "cangjie3.dict.yaml",
            "cangjie5.schema.yaml",
            "cangjie5.dict.yaml",
            "opencc/hk2s.json",
            "opencc/HKVariantsRev.ocd2",
            "opencc/HKVariantsRevPhrases.ocd2",
            "opencc/TSCharacters.ocd2",
            "opencc/TSPhrases.ocd2",
        ] {
            copy_asset(&schema_root, &self.shared, file_name);
        }
        for file_name in [
            "jyut6ping3.table.bin",
            "jyut6ping3.reverse.bin",
            "jyut6ping3_mobile.prism.bin",
            "jyut6ping3_scolar.table.bin",
            "jyut6ping3_scolar.reverse.bin",
            "jyut6ping3_scolar.prism.bin",
        ] {
            copy_asset(&schema_root, &self.shared, file_name);
        }
    }

    fn remove(self) {
        reset_rime();
        fs::remove_dir_all(self.root).expect("temp dir should be removed");
    }
}

#[test]
fn typeduck_adapter_deploys_browser_real_assets_after_init() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create_with_schema("browser-real-deploy", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_real_assets_prefix_fallback_keeps_raw_tail() {
    let _guard = test_guard();
    let runtime =
        TypeDuckRuntime::create_with_schema("browser-real-prefix-fallback", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    let key = CString::new("page_size").expect("custom key should be valid");
    let value = CString::new("6").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);

    let composing = process_input(state, "nri");
    let candidates = composing["context"]["candidates"]
        .as_array()
        .expect("nri prefix fallback candidates should be an array");
    let top_texts = candidates
        .iter()
        .take(5)
        .map(|candidate| candidate["text"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert_eq!(
        top_texts,
        vec!["\u{6211}", "\u{4f60}", "\u{5916}", "\u{80fd}", "\u{5167}"]
    );

    let committed = response_json(unsafe { yune_typeduck_select_candidate(state, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}ri".to_owned())])
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_real_assets_correction_enabled_reorders_nri() {
    let _guard = test_guard();
    let runtime =
        TypeDuckRuntime::create_with_schema("browser-real-correction-enabled", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    let key = CString::new("translator/enable_correction").expect("custom key should be valid");
    let value = CString::new("true").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);

    let composing = process_input(state, "nri");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    let committed = response_json(unsafe { yune_typeduck_select_candidate(state, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{4f60}".to_owned())])
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_real_assets_browser_defaults_keep_correction_nri_first() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create_with_schema(
        "browser-real-correction-browser-defaults",
        "jyut6ping3_mobile",
    );
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    for (key, value) in [
        ("page_size", "6"),
        ("translator/enable_completion", "true"),
        ("translator/enable_correction", "true"),
        ("translator/enable_sentence", "true"),
        ("translator/enable_user_dict", "true"),
        ("translator/encode_commit_history", "true"),
        ("translator/combine_candidates", "true"),
        ("translator/prediction_never_first", "true"),
        ("translator/prediction_weight_threshold", "0"),
        ("cangjie/dictionary", "cangjie5"),
    ] {
        let key = CString::new(key).expect("custom key should be valid");
        let value = CString::new(value).expect("custom value should be valid");
        assert_eq!(
            unsafe {
                yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr())
            },
            TRUE
        );
    }
    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);

    let composing = process_input(state, "nri");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_real_assets_preserve_typeduck_full_shape_state_labels() {
    let _guard = test_guard();
    let runtime =
        TypeDuckRuntime::create_with_schema("browser-real-state-labels", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let api = rime_get_api();
    assert!(!api.is_null());
    let api = unsafe { &*api };
    let setup = api.setup.expect("frontend requires setup");
    let initialize = api.initialize.expect("frontend requires initialize");
    let cleanup_all_sessions = api
        .cleanup_all_sessions
        .expect("frontend requires cleanup_all_sessions");
    let create_session = api
        .create_session
        .expect("frontend requires create_session");
    let destroy_session = api
        .destroy_session
        .expect("frontend requires destroy_session");
    let select_schema = api.select_schema.expect("frontend requires select_schema");
    let get_state_label = api
        .get_state_label
        .expect("frontend requires get_state_label");
    let get_state_label_abbreviated = api
        .get_state_label_abbreviated
        .expect("frontend requires get_state_label_abbreviated");

    cleanup_all_sessions();
    let mut traits = empty_rime_traits();
    traits.shared_data_dir = runtime.shared_c.as_ptr();
    traits.user_data_dir = runtime.user_c.as_ptr();
    unsafe { setup(&traits) };
    unsafe { initialize(&traits) };

    let session_id = create_session();
    assert_ne!(session_id, 0);
    assert_eq!(
        unsafe { select_schema(session_id, runtime.schema_id_c.as_ptr()) },
        TRUE
    );

    let full_shape = CString::new("full_shape").expect("option name should be valid");
    let half_label =
        state_label_text(unsafe { get_state_label(session_id, full_shape.as_ptr(), FALSE) });
    let full_label =
        state_label_text(unsafe { get_state_label(session_id, full_shape.as_ptr(), TRUE) });
    assert_eq!(half_label, "\u{534a}\u{5f62}");
    assert_eq!(full_label, "\u{5168}\u{5f62}");

    let half_abbrev =
        unsafe { get_state_label_abbreviated(session_id, full_shape.as_ptr(), FALSE, TRUE) };
    assert_eq!(half_abbrev.length, "\u{534a}".len());
    assert_eq!(state_label_slice(half_abbrev), "\u{534a}");
    let full_abbrev =
        unsafe { get_state_label_abbreviated(session_id, full_shape.as_ptr(), TRUE, TRUE) };
    assert_eq!(full_abbrev.length, "\u{5168}".len());
    assert_eq!(state_label_slice(full_abbrev), "\u{5168}");

    assert_eq!(destroy_session(session_id), TRUE);
    cleanup_all_sessions();
    runtime.remove();
}

#[test]
fn typeduck_adapter_deploys_browser_real_assets_after_customize() {
    let _guard = test_guard();
    let runtime =
        TypeDuckRuntime::create_with_schema("browser-real-customize-deploy", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    let key = CString::new("page_size").expect("custom key should be valid");
    let value = CString::new("6").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_real_assets_sentence_mode_commits_multisyllable_phrase() {
    let _guard = test_guard();
    let runtime =
        TypeDuckRuntime::create_with_schema("browser-real-sentence-mode", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    let key =
        CString::new("translator/enable_sentence").expect("custom key should be valid CString");
    let value = CString::new("true").expect("custom value should be valid CString");
    assert_eq!(
        unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);
    let deployed_schema: Value = serde_yaml::from_str(
        &fs::read_to_string(
            runtime
                .user
                .join("build")
                .join("jyut6ping3_mobile.schema.yaml"),
        )
        .expect("deployed real-assets schema should be readable"),
    )
    .expect("deployed real-assets schema should parse");
    assert_eq!(
        deployed_schema
            .pointer("/translator/enable_sentence")
            .and_then(config_bool_like),
        Some(true)
    );

    let mut composing = Value::Null;
    for key in "ngohaigo".chars() {
        composing = response_json(unsafe { yune_typeduck_process_key(state, key as i32, 0) });
    }
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())
    );

    let committed = response_json(unsafe { yune_typeduck_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    let mut composing = Value::Null;
    for key in "ngohaig".chars() {
        composing = response_json(unsafe { yune_typeduck_process_key(state, key as i32, 0) });
    }
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())
    );

    let committed = response_json(unsafe { yune_typeduck_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

#[test]
fn typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create_with_schema(
        "browser-real-dictionary-comments",
        "jyut6ping3_mobile",
    );
    if let Err(reason) = rich_dictionary_comment_oracle_build_status() {
        eprintln!(
            "SKIP typeduck_adapter_real_assets_emit_oracle_dictionary_panel_comments: {reason}. \
             This integration test does not pass against the degraded three-column fallback. \
             Clean-checkout byte parity is covered by `cargo test -p yune-core --test cantonese_parity`."
        );
        runtime.remove();
        return;
    }
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let mut composing = Value::Null;
    for key in "nei".chars() {
        composing = response_json(unsafe { yune_typeduck_process_key(state, key as i32, 0) });
    }
    let oracle: Value = serde_json::from_str(TYPEDUCK_V112_COMMENTS)
        .expect("TypeDuck v1.1.2 comments fixture should parse");
    let expected_comment = oracle["cases"]
        .as_array()
        .expect("oracle cases should be an array")
        .iter()
        .find(|case| case["input"] == "nei")
        .expect("nei should be captured by the v1.1.2 oracle")["selected_candidates"][0]["comment"]
        .as_str()
        .expect("oracle candidate comment should be a string");

    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );
    assert_eq!(
        composing["context"]["candidates"][0]["comment"],
        Value::String(expected_comment.to_owned())
    );

    unsafe { yune_typeduck_cleanup(state) };
    runtime.remove();
}

fn typeduck_oracle_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/typeduck-oracle/v1.1.2")
}

fn browser_app_schema_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/typeduck-web/source/public/schema")
}

fn rich_dictionary_comment_oracle_build_status() -> Result<PathBuf, String> {
    let build_root = typeduck_oracle_root().join("rime-user/build");
    let required_files = [
        "jyut6ping3.table.bin",
        "jyut6ping3.reverse.bin",
        "jyut6ping3_mobile.prism.bin",
        "jyut6ping3_scolar.table.bin",
        "jyut6ping3_scolar.reverse.bin",
        "jyut6ping3_scolar.prism.bin",
    ];
    let missing = required_files
        .into_iter()
        .filter(|file_name| !build_root.join(file_name).is_file())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(build_root)
    } else {
        Err(format!(
            "missing local TypeDuck v1.1.2 oracle build assets at {} ({})",
            build_root.display(),
            missing.join(", ")
        ))
    }
}

fn reset_rime() {
    let api = rime_get_api();
    if api.is_null() {
        return;
    }
    let api = unsafe { &*api };
    if let Some(cleanup_all_sessions) = api.cleanup_all_sessions {
        cleanup_all_sessions();
    }
    if let Some(finalize) = api.finalize {
        finalize();
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "yune-rime-api-typeduck-web-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn copy_asset(schema_root: &Path, destination_root: &Path, relative_path: &str) {
    let source = schema_root.join(relative_path);
    let destination = destination_root.join(relative_path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).expect("asset parent directory should be created");
    }
    fs::copy(source, destination).expect("browser preloaded asset should be copied");
}
