use std::{
    ffi::{CStr, CString},
    fs, mem,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};
use yune_core::{
    m37_metrics_enable, m37_metrics_reset, m37_metrics_snapshot, RimeDictArtifactStatus,
};
use yune_rime_api::{
    rime_get_api, workspace_dictionary_rebuild_reports, yune_web_cleanup, yune_web_customize,
    yune_web_delete_candidate, yune_web_deploy, yune_web_flip_page, yune_web_free_response,
    yune_web_init, yune_web_process_key, yune_web_response_handled, yune_web_response_json,
    yune_web_select_candidate, yune_web_set_ai_enabled, yune_web_set_option, yune_web_stage_ai,
    Bool, RimeDeploySchema, RimeDeployerInitialize, RimeRunTask, RimeStringSlice, RimeTraits,
    FALSE, TRUE,
};

const SCHEMA_ID: &str = "yune_web_luna";
const WEB03_COMPILED_SCHEMA_ASSETS: &[&str] = &[
    "jyut6ping3.table.bin",
    "jyut6ping3.reverse.bin",
    "jyut6ping3_mobile.prism.bin",
    "jyut6ping3_scolar.table.bin",
    "jyut6ping3_scolar.reverse.bin",
    "jyut6ping3_scolar.prism.bin",
    "luna_pinyin_yune_reverse.table.bin",
    "luna_pinyin_yune_reverse.reverse.bin",
    "luna_pinyin_yune_reverse.prism.bin",
    "cangjie5.table.bin",
    "cangjie5.reverse.bin",
    "cangjie5.prism.bin",
    "luna_pinyin.table.bin",
    "luna_pinyin.reverse.bin",
    "luna_pinyin.prism.bin",
];
const TYPEDUCK_V112_COMMENTS: &str =
    include_str!("../../yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-mobile-comments.json");
const TYPEDUCK_V112_M28_PARTIAL_SELECTION: &str = include_str!(
    "../../yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json"
);
const M28_UPSTREAM_JYUTPING_COMPOSITION: &str = include_str!(
    "../../yune-core/tests/fixtures/upstream-jyutping/jyutping-m28-followup-composition.json"
);

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

fn m28_oracle_continuation_components(fixture: &Value) -> Vec<String> {
    let comment = fixture["captured_next_candidates"][0]["comment"]
        .as_str()
        .expect("M28 fixture should capture continuation composition comment");
    comment
        .split('\r')
        .filter_map(|record| {
            record
                .strip_prefix('1')
                .or_else(|| record.strip_prefix('0'))
        })
        .filter_map(|record| record.split(',').nth(1))
        .skip(1)
        .map(str::to_owned)
        .collect()
}

#[test]
fn yune_web_adapter_processes_keys_and_returns_json_state() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("process-json-state");
    runtime.write_schema();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let first = response_json(unsafe { yune_web_process_key(state, 'b' as i32, 0) });
    assert_eq!(first["handled"], Value::Bool(true));
    assert_eq!(first["context"]["input"], Value::String("b".to_owned()));
    assert_eq!(
        first["status"]["schema_id"],
        Value::String(SCHEMA_ID.to_owned())
    );
    assert_eq!(first["status"]["is_composing"], Value::Bool(true));

    let second = response_json(unsafe { yune_web_process_key(state, 'a' as i32, 0) });
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

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_supports_page_candidate_actions_and_commits() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("candidate-actions");
    runtime.write_schema();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    drop(response_json(unsafe {
        yune_web_process_key(state, 'b' as i32, 0)
    }));
    let composing = response_json(unsafe { yune_web_process_key(state, 'a' as i32, 0) });
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("八".to_owned())
    );

    let next_page = response_json(unsafe { yune_web_flip_page(state, FALSE) });
    assert_eq!(next_page["handled"], Value::Bool(true));
    assert_eq!(next_page["context"]["page_no"], Value::from(1));
    assert_eq!(
        next_page["context"]["candidates"][0]["text"],
        Value::String("爸".to_owned())
    );

    let previous_page = response_json(unsafe { yune_web_flip_page(state, TRUE) });
    assert_eq!(previous_page["handled"], Value::Bool(true));
    assert_eq!(previous_page["context"]["page_no"], Value::from(0));

    let deleted = response_json(unsafe { yune_web_delete_candidate(state, 0) });
    assert_eq!(deleted["handled"], Value::Bool(true));
    assert_eq!(
        deleted["context"]["candidates"][0]["text"],
        Value::String("吧".to_owned())
    );

    let selected = response_json(unsafe { yune_web_select_candidate(state, 0) });
    assert_eq!(selected["handled"], Value::Bool(true));
    assert_eq!(
        selected["commits"],
        Value::Array(vec![Value::String("吧".to_owned())])
    );
    assert_eq!(selected["status"]["is_composing"], Value::Bool(false));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_documents_browser_host_layout_constraints() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("browser-host-layout");

    assert!(runtime.shared.exists());
    assert!(runtime.user.exists());
    assert!(
        runtime.user.join("build").exists(),
        "browser host fixture must create user_data_dir/build before init"
    );

    let state_without_preloaded_assets = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        state_without_preloaded_assets.is_null(),
        "init without preloaded schema/dictionary assets must fail deterministically"
    );

    runtime.write_schema_with_dictionary("yune_web");
    runtime.write_dictionary("stray");
    let state_with_wrong_dictionary = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        state_with_wrong_dictionary.is_null(),
        "init must reject preloads that omit the selected schema dictionary"
    );

    let path_like_schema_id = CString::new("../yune_web_luna").expect("schema id should be valid");
    let state_with_path_like_schema_id = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            path_like_schema_id.as_ptr(),
        )
    };
    assert!(
        state_with_path_like_schema_id.is_null(),
        "init must reject path-like schema ids before probing assets"
    );

    runtime.write_dictionary("yune_web");
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let response = unsafe { yune_web_process_key(state, 'b' as i32, 0) };
    let json = unsafe { yune_web_response_json(response) };
    assert!(!json.is_null());
    let text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("adapter JSON should be valid UTF-8")
        .to_owned();
    unsafe { yune_web_free_response(response) };
    let value: Value = serde_json::from_str(&text).expect("copied response should parse as JSON");
    assert_eq!(value["handled"], Value::Bool(true));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_accepts_deployed_schema_dictionary_for_inherited_source_schema() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("deployed-dictionary");
    runtime.write_source_schema_with_deployed_dictionary("yune_web");
    runtime.write_dictionary("yune_web");

    let state = unsafe {
        yune_web_init(
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
        yune_web_process_key(state, 'b' as i32, 0)
    }));
    let composing = response_json(unsafe { yune_web_process_key(state, 'a' as i32, 0) });
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{516b}".to_owned())
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_composes_source_dictionary_with_mobile_schema_algebra() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("source-dictionary-mobile-algebra");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    drop(response_json(unsafe {
        yune_web_process_key(state, 'n' as i32, 0)
    }));
    drop(response_json(unsafe {
        yune_web_process_key(state, 'e' as i32, 0)
    }));
    let composing = response_json(unsafe { yune_web_process_key(state, 'i' as i32, 0) });
    assert_eq!(
        composing["context"]["input"],
        Value::String("nei".to_owned())
    );
    assert_eq!(composing["context"]["select_keys"], Value::Null);
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_customized_sentence_mode_commits_multisyllable_phrase() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("mobile-sentence-customize");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_sentence_dictionary();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("yune_web_luna.schema").expect("config id should be valid");
    let key =
        CString::new("translator/enable_sentence").expect("custom key should be valid CString");
    let value = CString::new("true").expect("custom value should be valid CString");
    assert_eq!(
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);
    let deployed_schema: Value = serde_yaml::from_str(
        &fs::read_to_string(runtime.user.join("build").join("yune_web_luna.schema.yaml"))
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
        composing = response_json(unsafe { yune_web_process_key(state, key as i32, 0) });
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

    let committed = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    let mut composing = Value::Null;
    for key in "ngohaig".chars() {
        composing = response_json(unsafe { yune_web_process_key(state, key as i32, 0) });
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
        "Real-assets schema does not declare echo_translator, so ngohaig must not leak a raw echo candidate"
    );

    let committed = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_deploy_and_customize_are_explicit() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("deploy-customize");
    runtime.write_schema();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);
    let config_id = CString::new("yune_web_luna.schema").expect("config id should be valid");
    let key = CString::new("schema/name").expect("custom key should be valid");
    let value = CString::new("Yune Web Luna Web").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    let saved = fs::read_to_string(runtime.user.join("yune_web_luna.custom.yaml"))
        .expect("customized schema patch should be saved");
    assert!(saved.contains("schema/name"));
    assert!(saved.contains("Yune Web Luna Web"));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_customizes_dictionary_exclude_as_yaml_list() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("dictionary-exclude-customize");
    runtime.write_schema();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);
    let before = process_input(state, "ba");
    assert_eq!(
        before["context"]["candidates"][0]["text"],
        Value::String("八".to_owned())
    );

    let config_id = CString::new("yune_web_luna.schema").expect("config id should be valid");
    let key = CString::new("translator/dictionary_exclude").expect("custom key should be valid");
    let value = CString::new(r#"["八"]"#).expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );

    let saved = fs::read_to_string(runtime.user.join("yune_web_luna.custom.yaml"))
        .expect("customized schema patch should be saved");
    let saved_yaml: serde_yaml::Value =
        serde_yaml::from_str(&saved).expect("customized schema patch should be valid YAML");
    assert_eq!(
        saved_yaml["patch"]["translator/dictionary_exclude"],
        serde_yaml::Value::Sequence(vec![serde_yaml::Value::String("八".to_owned())])
    );

    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);
    let after = process_input(state, "ba");
    assert_eq!(
        after["context"]["candidates"][0]["text"],
        Value::String("吧".to_owned())
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_set_option_updates_session_status() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("set-option");
    runtime.write_schema();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let ascii_mode = CString::new("ascii_mode").expect("option should be valid");
    assert_eq!(
        unsafe { yune_web_set_option(state, ascii_mode.as_ptr(), TRUE) },
        TRUE
    );
    let ascii_enabled = response_json(unsafe { yune_web_process_key(state, 'b' as i32, 0) });
    assert_eq!(ascii_enabled["status"]["is_ascii_mode"], Value::Bool(true));

    assert_eq!(
        unsafe { yune_web_set_option(state, ascii_mode.as_ptr(), FALSE) },
        TRUE
    );
    let ascii_disabled = response_json(unsafe { yune_web_process_key(state, 'a' as i32, 0) });
    assert_eq!(
        ascii_disabled["status"]["is_ascii_mode"],
        Value::Bool(false)
    );

    assert_eq!(
        unsafe { yune_web_set_option(ptr::null_mut(), ascii_mode.as_ptr(), TRUE) },
        FALSE
    );
    assert_eq!(
        unsafe { yune_web_set_option(state, ptr::null(), TRUE) },
        FALSE
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_inspector_is_opt_in_and_preserves_classic_candidate_output() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("inspector-opt-in");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let inspector = CString::new("yune_inspector").expect("option should be valid");
    let inspector_off = process_input(state, "nei");
    assert!(inspector_off["context"].get("debug").is_none());
    assert!(inspector_off["context"]["candidates"]
        .as_array()
        .expect("off candidates should be an array")
        .iter()
        .all(|candidate| candidate.get("quality").is_none()));
    let off_full_bytes =
        serde_json::to_vec(&inspector_off).expect("inspector-off response should serialize");
    let off_classic_bytes = serde_json::to_vec(&classic_candidate_projection(&inspector_off))
        .expect("classic projection should serialize");

    unsafe { yune_web_cleanup(state) };

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
    assert_eq!(
        unsafe { yune_web_set_option(state, inspector.as_ptr(), FALSE) },
        TRUE
    );

    let inspector_explicitly_off = process_input(state, "nei");
    let explicitly_off_full_bytes = serde_json::to_vec(&inspector_explicitly_off)
        .expect("explicitly disabled inspector response should serialize");
    assert_eq!(
        explicitly_off_full_bytes, off_full_bytes,
        "explicitly disabled inspector must preserve the full classic response"
    );

    unsafe { yune_web_cleanup(state) };

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
    assert_eq!(
        unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
        TRUE
    );

    let inspector_on = process_input(state, "nei");
    let on_classic_bytes = serde_json::to_vec(&classic_candidate_projection(&inspector_on))
        .expect("classic projection should serialize");
    assert_eq!(
        on_classic_bytes, off_classic_bytes,
        "inspector must not change classic candidate text/comment output"
    );
    assert_eq!(
        inspector_on["context"]["candidates"][0]["source"],
        Value::String("table".to_owned())
    );
    assert!(
        inspector_on["context"]["candidates"][0]["quality"].is_number(),
        "inspector candidates should expose debug quality only when opted in"
    );
    assert_eq!(
        inspector_on["context"]["debug"]["segment_tags"],
        Value::Array(vec![Value::String("abc".to_owned())])
    );
    assert_eq!(
        inspector_on["context"]["debug"]["ai_staging"]["state"],
        Value::String("off".to_owned())
    );
    assert!(inspector_on["context"]["debug"]["spelling_algebra"]
        .as_array()
        .expect("spelling algebra debug should be an array")
        .iter()
        .any(|algebra| algebra["expanded_codes"]
            .as_array()
            .is_some_and(|codes| codes.contains(&Value::String("nei".to_owned())))));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_stage_ai_is_default_off_and_second_pass_source_labeled() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("stage-ai-second-pass");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_web_init(
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

    let default_off = response_json(unsafe { yune_web_stage_ai(state) });
    assert_eq!(
        default_off["context"]["candidates"],
        Value::Array(classic_candidates.clone())
    );

    assert_eq!(unsafe { yune_web_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_web_stage_ai(state) });
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
    let later_response = response_json(unsafe { yune_web_flip_page(state, TRUE) });
    assert!(later_response["context"]["candidates"]
        .as_array()
        .expect("later response candidates should be an array")
        .iter()
        .any(|candidate| candidate["source"] == Value::String("ai:local".to_owned())));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_disabling_ai_clears_staged_rows_for_current_input() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("stage-ai-disable-clears");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let classic = process_input(state, "nei");
    let classic_candidates = classic["context"]["candidates"].clone();
    assert_eq!(unsafe { yune_web_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_web_stage_ai(state) });
    assert!(staged["context"]["candidates"]
        .as_array()
        .expect("staged candidates should be an array")
        .iter()
        .any(|candidate| candidate["source"] == Value::String("ai:local".to_owned())));

    assert_eq!(unsafe { yune_web_set_ai_enabled(state, FALSE) }, TRUE);
    let disabled = response_json(unsafe { yune_web_stage_ai(state) });
    assert_eq!(disabled["context"]["candidates"], classic_candidates);
    assert!(
        disabled["context"]["candidates"]
            .as_array()
            .expect("disabled candidates should be an array")
            .iter()
            .all(|candidate| candidate.get("source").is_none()),
        "disabling AI must remove stale source-labeled rows immediately"
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_ai_rows_do_not_auto_commit_and_do_not_write_userdb() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("stage-ai-commit-safety");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let classic = process_input(state, "nei");
    let classic_top = classic["context"]["candidates"][0]["text"].clone();
    assert_eq!(unsafe { yune_web_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_web_stage_ai(state) });
    assert!(staged["context"]["candidates"]
        .as_array()
        .expect("staged candidates should be an array")
        .iter()
        .any(|candidate| candidate["source"] == Value::String("ai:local".to_owned())));

    let default_commit = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        default_commit["commits"],
        Value::Array(vec![classic_top]),
        "Space/default confirm must commit the classic top row"
    );
    unsafe { yune_web_cleanup(state) };
    runtime.remove();

    let runtime = YuneWebRuntime::create("stage-ai-explicit-commit-safety");
    runtime.write_mobile_schema_with_dictionary("jyut6ping3");
    runtime.write_cantonese_dictionary();
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
    process_input(state, "nei");
    assert_eq!(unsafe { yune_web_set_ai_enabled(state, TRUE) }, TRUE);
    let staged = response_json(unsafe { yune_web_stage_ai(state) });
    let ai_index = staged["context"]["candidates"]
        .as_array()
        .expect("staged candidates should be an array")
        .iter()
        .position(|candidate| candidate["source"] == Value::String("ai:local".to_owned()))
        .expect("AI row should be selectable");
    let selected = response_json(unsafe { yune_web_select_candidate(state, ai_index) });
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

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_classic_commit_writes_userdb() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create("classic-userdb-learning");
    runtime.write_mobile_schema_with_reverse_dictionary();
    runtime.write_cantonese_dictionary();
    runtime.write_dictionary("cangjie5");
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let composing = process_input(state, "nei");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    let committed = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{4f60}".to_owned())])
    );

    let store_path = runtime.user.join("jyut6ping3.userdb");
    let stored = fs::read_to_string(&store_path).expect("classic commit should write userdb");
    assert!(stored.contains("/db_name\tjyut6ping3\n"));
    assert!(stored.contains("\t\u{4f60}\tc="), "{stored}");
    assert!(
        !runtime.user.join("cangjie5.userdb").exists(),
        "reverse-lookup dictionaries must not own classic userdb learning"
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_handles_null_inputs_and_response_freeing() {
    let _guard = test_guard();
    assert!(unsafe { yune_web_init(ptr::null(), ptr::null(), ptr::null()) }.is_null());
    assert_eq!(unsafe { yune_web_deploy(ptr::null_mut()) }, FALSE);
    assert_eq!(
        unsafe { yune_web_customize(ptr::null_mut(), ptr::null(), ptr::null(), ptr::null()) },
        FALSE
    );
    assert_eq!(
        unsafe { yune_web_set_option(ptr::null_mut(), ptr::null(), TRUE) },
        FALSE
    );
    assert_eq!(
        unsafe { yune_web_set_ai_enabled(ptr::null_mut(), TRUE) },
        FALSE
    );
    assert!(unsafe { yune_web_process_key(ptr::null_mut(), 'a' as i32, 0) }.is_null());
    assert!(unsafe { yune_web_stage_ai(ptr::null_mut()) }.is_null());
    assert!(unsafe { yune_web_response_json(ptr::null()) }.is_null());
    assert_eq!(unsafe { yune_web_response_handled(ptr::null()) }, FALSE);
    unsafe { yune_web_free_response(ptr::null_mut()) };
}

fn process_input(state: *mut yune_rime_api::YuneWebState, input: &str) -> Value {
    let mut response = Value::Null;
    for key in input.chars() {
        response = response_json(unsafe { yune_web_process_key(state, key as i32, 0) });
    }
    response
}

fn response_json(response: *mut yune_rime_api::YuneWebResponse) -> Value {
    assert!(!response.is_null());
    let handled: Bool = unsafe { yune_web_response_handled(response) };
    let json = unsafe { yune_web_response_json(response) };
    assert!(!json.is_null());
    let text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("adapter JSON should be valid UTF-8")
        .to_owned();
    unsafe { yune_web_free_response(response) };
    let value: Value = serde_json::from_str(&text).expect("adapter response should parse as JSON");
    assert_eq!(value["handled"].as_bool(), Some(handled == TRUE));
    value
}

fn classic_candidate_projection(response: &Value) -> Value {
    Value::Array(
        response["context"]["candidates"]
            .as_array()
            .expect("candidate list should be an array")
            .iter()
            .map(|candidate| {
                serde_json::json!({
                    "text": candidate["text"],
                    "comment": candidate["comment"],
                })
            })
            .collect(),
    )
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

fn patch_browser_app_keyboard_profile(runtime: &YuneWebRuntime) {
    for relative_path in [
        "template.yaml",
        "jyut6ping3.schema.yaml",
        "jyut6ping3_mobile.schema.yaml",
        "build/jyut6ping3.schema.yaml",
        "build/jyut6ping3_mobile.schema.yaml",
    ] {
        let path = if relative_path.starts_with("build/") {
            runtime.user.join(relative_path)
        } else {
            runtime.shared.join(relative_path)
        };
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path).expect("schema config should be readable");
        let text = patch_dictionary_lookup_records_opt_out(&text);
        let text = patch_namespace_lookup_records_opt_out(&text, "translator");
        let text = patch_namespace_lookup_records_opt_out(&text, "luna_pinyin");
        let text = patch_namespace_bool_opt_out(&text, "luna_pinyin", "load_translator", false);
        fs::write(&path, text).expect("keyboard profile config patch should be written");
    }
}

fn patch_browser_app_comments_intact_keyboard_profile(runtime: &YuneWebRuntime) {
    for relative_path in [
        "template.yaml",
        "jyut6ping3.schema.yaml",
        "jyut6ping3_mobile.schema.yaml",
        "build/jyut6ping3.schema.yaml",
        "build/jyut6ping3_mobile.schema.yaml",
    ] {
        let path = if relative_path.starts_with("build/") {
            runtime.user.join(relative_path)
        } else {
            runtime.shared.join(relative_path)
        };
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path).expect("schema config should be readable");
        let text = patch_namespace_bool_opt_out(&text, "luna_pinyin", "load_translator", false);
        fs::write(&path, text)
            .expect("comments-intact keyboard profile config patch should be written");
    }
}

fn patch_dictionary_lookup_records_opt_out(text: &str) -> String {
    if text.contains("dictionary_lookup_filter/load_lookup_records") {
        return text.to_owned();
    }
    if text.contains("dictionary_lookup_filter:\n  dictionary: jyut6ping3_scolar\n  load_lookup_records: false\n")
    {
        return text.to_owned();
    }
    text.replace(
        "dictionary_lookup_filter:\n  dictionary: jyut6ping3_scolar\n",
        "dictionary_lookup_filter:\n  dictionary: jyut6ping3_scolar\n  load_lookup_records: false\n",
    )
}

fn patch_namespace_lookup_records_opt_out(text: &str, namespace: &str) -> String {
    patch_namespace_bool_opt_out(text, namespace, "load_lookup_records", false)
}

fn patch_namespace_bool_opt_out(text: &str, namespace: &str, key: &str, value: bool) -> String {
    let marker = format!("{namespace}:\n");
    let Some(block_start) = text.find(&marker).map(|index| index + marker.len()) else {
        return text.to_owned();
    };
    let block_end = text[block_start..]
        .find("\n\n")
        .map(|offset| block_start + offset)
        .unwrap_or(text.len());
    if text[block_start..block_end].contains(&format!("{key}:")) {
        return text.to_owned();
    }
    let insertion = format!("{marker}  {key}: {value}\n");
    text.replacen(&marker, &insertion, 1)
}

struct YuneWebRuntime {
    root: PathBuf,
    shared: PathBuf,
    user: PathBuf,
    shared_c: CString,
    user_c: CString,
    schema_id_c: CString,
}

impl YuneWebRuntime {
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
        self.write_schema_with_dictionary("yune_web");
        self.write_dictionary("yune_web");
    }

    fn write_schema_with_dictionary(&self, dictionary: &str) {
        let default_config = "config_version: yune-web\nschema_list:\n  - schema: yune_web_luna\n";
        let schema_config = format!(
            "\
schema:\n  schema_id: yune_web_luna\n  name: Yune Web Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: {dictionary}\n"
        );
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("yune_web_luna.schema.yaml"), &schema_config)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("yune_web_luna.schema.yaml"), schema_config)
            .expect("shared schema config should be written");
    }

    fn write_source_schema_with_deployed_dictionary(&self, dictionary: &str) {
        let default_config = "config_version: yune-web\nschema_list:\n  - schema: yune_web_luna\n";
        let source_schema = "\
schema:\n  schema_id: yune_web_luna\n  name: Yune Web Luna\n__include: template:/\n";
        let deployed_schema = format!(
            "\
schema:\n  schema_id: yune_web_luna\n  name: Yune Web Luna\nmenu:\n  page_size: 2\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: {dictionary}\n"
        );
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("yune_web_luna.schema.yaml"), deployed_schema)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("yune_web_luna.schema.yaml"), source_schema)
            .expect("shared schema config should be written");
    }

    fn write_mobile_schema_with_dictionary(&self, dictionary: &str) {
        let default_config = "config_version: yune-web\nschema_list:\n  - schema: yune_web_luna\n";
        let schema_config = format!(
            "\
schema:\n  schema_id: yune_web_luna\n  name: Yune Web Luna\nmenu:\n  page_size: 50\n  alternative_select_keys: \"\\x00\"\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  processors:\n    - speller\n    - express_editor\n  translators:\n    - script_translator\nspeller:\n  alphabet: zyxwvutsrqponmlkjihgfedcba\n  delimiter: \" '\"\n  algebra:\n    - \"derive/\\\\d//\"\ntranslator:\n  dictionary: {dictionary}\n  enable_completion: true\n  enable_sentence: false\n"
        );
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("yune_web_luna.schema.yaml"), &schema_config)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("yune_web_luna.schema.yaml"), schema_config)
            .expect("shared schema config should be written");
    }

    fn write_mobile_schema_with_reverse_dictionary(&self) {
        let default_config = "config_version: yune-web\nschema_list:\n  - schema: yune_web_luna\n";
        let schema_config = "\
schema:\n  schema_id: yune_web_luna\n  name: Yune Web Luna\nmenu:\n  page_size: 50\n  alternative_select_keys: \"\\x00\"\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  processors:\n    - speller\n    - express_editor\n  translators:\n    - script_translator\n    - table_translator@cangjie\nspeller:\n  alphabet: zyxwvutsrqponmlkjihgfedcba\n  delimiter: \" '\"\n  algebra:\n    - \"derive/\\\\d//\"\ntranslator:\n  dictionary: jyut6ping3\n  enable_completion: true\n  enable_sentence: false\ncangjie:\n  dictionary: cangjie5\n  prefix: \"`vc\"\n  suffix: \";\"\n";
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("yune_web_luna.schema.yaml"), schema_config)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("yune_web_luna.schema.yaml"), schema_config)
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
            "jyut6ping3_scolar.schema.yaml",
            "jyut6ping3_scolar.dict.yaml",
            "luna_pinyin.schema.yaml",
            "luna_pinyin.dict.yaml",
            "opencc/hk2s.json",
            "opencc/HKVariantsRev.ocd2",
            "opencc/HKVariantsRevPhrases.ocd2",
            "opencc/TSCharacters.ocd2",
            "opencc/TSPhrases.ocd2",
        ] {
            copy_asset(&schema_root, &self.shared, file_name);
        }
        for file_name in [
            "jyut6ping3_mobile_longpress.schema.yaml",
            "jyut6ping3_mobile_10keys.schema.yaml",
            "loengfan.schema.yaml",
            "loengfan.dict.yaml",
            "loengfan_longpress.schema.yaml",
            "cangjie3.schema.yaml",
            "cangjie3.dict.yaml",
            "cangjie5.schema.yaml",
            "cangjie5.dict.yaml",
        ] {
            copy_asset_if_exists(&schema_root, &self.shared, file_name);
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
            "jyut6ping3_scolar.schema.yaml",
            "jyut6ping3_scolar.dict.yaml",
            "luna_pinyin.schema.yaml",
            "luna_pinyin.dict.yaml",
            "luna_pinyin_yune_reverse.dict.yaml",
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
            "luna_pinyin_yune_reverse.table.bin",
            "luna_pinyin_yune_reverse.reverse.bin",
            "luna_pinyin_yune_reverse.prism.bin",
            "cangjie5.table.bin",
            "cangjie5.reverse.bin",
            "cangjie5.prism.bin",
            "luna_pinyin.table.bin",
            "luna_pinyin.reverse.bin",
            "luna_pinyin.prism.bin",
        ] {
            copy_asset(&schema_root, &self.shared, file_name);
        }
    }

    fn write_public_demo_assets(&self) {
        let schema_root = public_demo_schema_root();
        copy_dir_contents(&schema_root, &self.shared);
        let staging = self.user.join("build");
        for file_name in ["default.yaml", "jyut6ping3_mobile.schema.yaml"] {
            fs::copy(
                schema_root.join("build").join(file_name),
                staging.join(file_name),
            )
            .expect("public-demo preloaded build asset should be copied");
        }
    }

    fn remove(self) {
        reset_rime();
        fs::remove_dir_all(self.root).expect("temp dir should be removed");
    }
}

#[test]
fn yune_web_adapter_deploys_browser_real_assets_after_init() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create_with_schema("browser-real-deploy", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_storage_diagnostics_reports_live_jyutping_storage() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-app-storage-diagnostics", "jyut6ping3_mobile");
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let inspector = CString::new("yune_inspector").expect("option should be valid");
    assert_eq!(
        unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
        TRUE
    );
    let composing = process_input(state, "nei");
    let storage = &composing["context"]["debug"]["storage"];
    assert!(storage["source_fallback"].is_boolean());
    let selected = storage["selected"]
        .as_array()
        .expect("selected storage rows should be an array");
    assert!(
        selected
            .iter()
            .any(|row| row["owner"] == "compact_table.storage"
                || row["owner"] == "translator.entries_by_code"),
        "Jyutping should report a live translator storage row: {selected:?}"
    );
    assert!(storage["memory_owner_rows"]
        .as_array()
        .expect("memory owner rows should be an array")
        .iter()
        .any(|row| row["owner"] == "compact_table.storage"
            || row["owner"] == "translator.entries_by_code"));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
#[ignore = "WEB-02 evidence-only: writes public-demo storage diagnostics to YUNE_WEB02_EVIDENCE_DIR"]
fn web02_public_demo_storage_diagnostics_exports_owner_rows() {
    let _guard = test_guard();
    let evidence_dir = PathBuf::from(
        std::env::var_os("YUNE_WEB02_EVIDENCE_DIR")
            .expect("YUNE_WEB02_EVIDENCE_DIR must point to the WEB-02 evidence directory"),
    );
    fs::create_dir_all(&evidence_dir).expect("evidence directory should be created");

    let runtime =
        YuneWebRuntime::create_with_schema("web02-public-demo-diagnostics", "jyut6ping3_mobile");
    runtime.write_public_demo_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let inspector = CString::new("yune_inspector").expect("option should be valid");
    assert_eq!(
        unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
        TRUE
    );
    let composing = process_input(state, "nei");
    let storage = &composing["context"]["debug"]["storage"];
    let selected = storage["selected"]
        .as_array()
        .expect("selected storage rows should be an array");
    assert!(
        !selected.is_empty(),
        "public-demo Jyutping should report at least one live storage row"
    );

    let total_byte_source_len = selected
        .iter()
        .filter_map(|row| row["byte_source_len"].as_u64())
        .sum::<u64>();
    let memory_rows = storage["memory_owner_rows"]
        .as_array()
        .expect("memory owner rows should be an array");
    let reported_memory_owner_bytes = memory_rows
        .iter()
        .filter_map(|row| row["estimated_bytes"].as_u64())
        .sum::<u64>();
    let evidence = json!({
        "schema_id": "jyut6ping3_mobile",
        "input": "nei",
        "asset_root": public_demo_schema_root().display().to_string(),
        "source_fallback": storage["source_fallback"],
        "storage": storage,
        "summary": {
            "selected_storage_values": selected.iter()
                .filter_map(|row| row["selected_storage"].as_str())
                .collect::<Vec<_>>(),
            "total_byte_source_len": total_byte_source_len,
            "reported_memory_owner_bytes": reported_memory_owner_bytes,
        },
        "candidate_head": composing["context"]["candidates"]
            .as_array()
            .expect("candidate rows should be an array")
            .iter()
            .take(5)
            .cloned()
            .collect::<Vec<_>>(),
    });
    fs::write(
        evidence_dir.join("storage-diagnostics.json"),
        serde_json::to_string_pretty(&evidence).expect("evidence should serialize"),
    )
    .expect("storage diagnostics evidence should be written");
    fs::write(
        evidence_dir.join("storage-selected.csv"),
        storage_selected_csv(selected),
    )
    .expect("storage selected CSV should be written");
    fs::write(
        evidence_dir.join("memory-owner-rows.csv"),
        memory_owner_rows_csv(memory_rows),
    )
    .expect("memory owner rows CSV should be written");
    fs::write(
        evidence_dir.join("compiled-asset-inventory.csv"),
        compiled_asset_inventory_csv(&runtime),
    )
    .expect("compiled asset inventory CSV should be written");

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
#[ignore = "WEB-03 evidence-only: set YUNE_WEB03_EVIDENCE_DIR and optionally YUNE_WEB03_APPLY_ASSETS=1"]
fn web03_regenerates_public_schema_compiled_assets_from_clean_rebuild() {
    let _guard = test_guard();
    let evidence_dir = web03_evidence_root_from_env()
        .expect("YUNE_WEB03_EVIDENCE_DIR must point to the WEB-03 evidence directory")
        .join("task2-native-regeneration");
    fs::create_dir_all(&evidence_dir).expect("evidence directory should be created");

    let root = unique_temp_dir("web03-clean-schema-rebuild");
    let shared = root.join("shared");
    let user = root.join("user");
    copy_clean_schema_sources(&browser_app_schema_root(), &shared);
    fs::create_dir_all(user.join("build")).expect("user build dir should be created");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_rime_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { RimeDeployerInitialize(&traits) };

    for task in [
        "workspace_update:jyut6ping3_mobile",
        "workspace_update:cangjie5",
        "workspace_update:luna_pinyin",
    ] {
        let task = CString::new(task).expect("task name should be valid");
        assert_eq!(RimeRunTask(task.as_ptr()), TRUE);
    }

    let reports = workspace_dictionary_rebuild_reports();
    assert!(
        !reports.is_empty(),
        "clean public-schema rebuild should emit dictionary rebuild reports"
    );
    for report in &reports {
        assert_ne!(
            report.report.table,
            RimeDictArtifactStatus::ReusedPrebuilt,
            "{report:?}"
        );
        assert_ne!(
            report.report.prism,
            RimeDictArtifactStatus::ReusedPrebuilt,
            "{report:?}"
        );
        assert_ne!(
            report.report.reverse,
            RimeDictArtifactStatus::ReusedPrebuilt,
            "{report:?}"
        );
    }
    for dictionary_id in [
        "jyut6ping3",
        "jyut6ping3_scolar",
        "luna_pinyin_yune_reverse",
        "cangjie5",
        "luna_pinyin",
    ] {
        assert_dictionary_rebuilt_from_source(&reports, dictionary_id);
    }

    let build_dir = user.join("build");
    for file_name in WEB03_COMPILED_SCHEMA_ASSETS {
        let path = build_dir.join(file_name);
        assert!(path.is_file(), "missing regenerated asset {file_name}");
        if file_name.ends_with(".prism.bin") {
            yune_core::parse_rime_prism_bin_payload(
                fs::read(&path).expect("prism should be readable"),
            )
            .unwrap_or_else(|error| panic!("{file_name} should be Rime::Prism/4.0: {error:?}"));
        }
    }

    fs::write(
        evidence_dir.join("workspace-rebuild-reports.json"),
        serde_json::to_string_pretty(&workspace_rebuild_reports_json(&reports))
            .expect("rebuild report evidence should serialize"),
    )
    .expect("rebuild report evidence should be written");
    fs::write(
        evidence_dir.join("workspace-rebuild-reports.csv"),
        workspace_rebuild_reports_csv(&reports),
    )
    .expect("rebuild report CSV should be written");
    fs::write(
        evidence_dir.join("compiled-asset-inventory.csv"),
        compiled_asset_inventory_for_root_csv(&build_dir, WEB03_COMPILED_SCHEMA_ASSETS),
    )
    .expect("compiled asset inventory should be written");

    if std::env::var_os("YUNE_WEB03_APPLY_ASSETS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        for file_name in WEB03_COMPILED_SCHEMA_ASSETS {
            fs::copy(
                build_dir.join(file_name),
                browser_app_schema_root().join(file_name),
            )
            .expect("regenerated asset should be copied into public/schema");
        }
    }

    reset_rime();
    fs::remove_dir_all(root).expect("temp dir should be removed");
}

#[test]
fn web03_public_demo_launch_schemas_byte_back_compiled_assets() {
    let _guard = test_guard();
    let mut evidence = Vec::new();
    let mut selected_csv_rows = Vec::new();
    let mut memory_csv_rows = Vec::new();

    for (schema_id, input, expected_top) in [
        ("jyut6ping3_mobile", "nei", Some("\u{4f60}")),
        ("cangjie5", "a", Some("\u{65e5}")),
        ("luna_pinyin", "ni", Some("\u{4f60}")),
    ] {
        let runtime = YuneWebRuntime::create_with_schema(
            &format!("web03-byte-backed-{schema_id}"),
            schema_id,
        );
        runtime.write_public_demo_assets();
        deploy_public_demo_schema(&runtime, schema_id);

        let state = unsafe {
            yune_web_init(
                runtime.shared_c.as_ptr(),
                runtime.user_c.as_ptr(),
                runtime.schema_id_c.as_ptr(),
            )
        };
        assert!(
            !state.is_null(),
            "{schema_id} should initialize from public-demo assets"
        );
        assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

        let inspector = CString::new("yune_inspector").expect("option should be valid");
        assert_eq!(
            unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
            TRUE
        );
        let composing = process_input(state, input);
        let candidates = composing["context"]["candidates"]
            .as_array()
            .expect("candidate rows should be an array");
        assert!(
            !candidates.is_empty(),
            "{schema_id} should produce candidates for {input:?}: {composing:?}"
        );
        if let Some(expected_top) = expected_top {
            assert_eq!(
                candidates[0]["text"],
                Value::String(expected_top.to_owned()),
                "{schema_id} should preserve deterministic smoke output for {input:?}"
            );
        }

        if schema_id == "jyut6ping3_mobile" {
            drop(response_json(unsafe {
                yune_web_process_key(state, 0xff1b, 0)
            }));
            let phrase = process_input(state, "ngogokdak");
            let phrase_candidates = phrase["context"]["candidates"]
                .as_array()
                .expect("phrase candidate rows should be an array");
            assert_eq!(
                phrase_candidates.first().map(|candidate| &candidate["text"]),
                Some(&Value::String("\u{6211}\u{89ba}\u{5f97}".to_owned())),
                "{schema_id} should compose multi-syllable phrase ngogokdak byte-backed: {phrase:?}"
            );
        }

        let storage = &composing["context"]["debug"]["storage"];
        assert_schema_storage_byte_backed(schema_id, storage);
        let selected = storage["selected"]
            .as_array()
            .expect("selected storage rows should be an array");
        let memory_rows = storage["memory_owner_rows"]
            .as_array()
            .expect("memory owner rows should be an array");
        evidence.push(json!({
            "schema_id": schema_id,
            "input": input,
            "source_fallback": storage["source_fallback"],
            "storage": storage,
            "candidate_head": candidates.iter().take(5).cloned().collect::<Vec<_>>(),
        }));
        selected_csv_rows.extend(
            selected
                .iter()
                .cloned()
                .map(|mut row| {
                    row["schema_id"] = Value::String(schema_id.to_owned());
                    row
                })
                .collect::<Vec<_>>(),
        );
        memory_csv_rows.extend(
            memory_rows
                .iter()
                .cloned()
                .map(|mut row| {
                    row["schema_id"] = Value::String(schema_id.to_owned());
                    row
                })
                .collect::<Vec<_>>(),
        );

        unsafe { yune_web_cleanup(state) };
        runtime.remove();
    }

    if let Some(evidence_dir) = web03_evidence_root_from_env() {
        let evidence_dir = evidence_dir.join("task3-native-byte-backed");
        fs::create_dir_all(&evidence_dir).expect("evidence directory should be created");
        fs::write(
            evidence_dir.join("storage-diagnostics-all-schemas.json"),
            serde_json::to_string_pretty(&evidence).expect("evidence should serialize"),
        )
        .expect("storage diagnostics evidence should be written");
        fs::write(
            evidence_dir.join("storage-selected-all-schemas.csv"),
            schema_storage_selected_csv(&selected_csv_rows),
        )
        .expect("storage selected CSV should be written");
        fs::write(
            evidence_dir.join("memory-owner-rows-all-schemas.csv"),
            schema_memory_owner_rows_csv(&memory_csv_rows),
        )
        .expect("memory owner rows CSV should be written");
        fs::write(
            evidence_dir.join("compiled-asset-inventory.csv"),
            compiled_asset_inventory_for_root_csv(
                &public_demo_schema_root(),
                WEB03_COMPILED_SCHEMA_ASSETS,
            ),
        )
        .expect("compiled asset inventory should be written");
    }
}

#[test]
fn web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("web03-jyutping-long-input-perf", "jyut6ping3_mobile");
    runtime.write_public_demo_assets();
    deploy_public_demo_schema(&runtime, "jyut6ping3_mobile");

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        !state.is_null(),
        "jyut6ping3_mobile should initialize from public-demo assets"
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let inspector = CString::new("yune_inspector").expect("option should be valid");
    assert_eq!(
        unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
        TRUE
    );
    let composing = process_input(state, "nei");
    assert_schema_storage_byte_backed(
        "jyut6ping3_mobile",
        &composing["context"]["debug"]["storage"],
    );

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));

    let long_inputs = [
        (
            "sihaacoenggeoisyujapgecukdou",
            "\u{6642}\u{4e0b}\u{5834}\u{64da}\u{8f38}\u{5165}\u{5605}\u{901f}\u{5ea6}",
            5_000,
        ),
        (
            "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng",
            "\u{7747}\u{4e0b}\u{5982}\u{679c}\u{6253}\u{597d}\u{5834}\u{5605}\u{53e5}\u{5b50}\u{500b}\u{6027}\u{80fd}\u{6703}\u{9ede}\u{6a23}",
            6_000,
        ),
    ];

    for (long_input, expected_first_candidate, prefix_fallback_visit_limit) in long_inputs {
        drop(response_json(unsafe {
            yune_web_process_key(state, 0xff1b, 0)
        }));

        m37_metrics_enable(true);
        m37_metrics_reset();
        let response = process_input(state, long_input);
        let metrics = m37_metrics_snapshot();
        m37_metrics_enable(false);

        let candidates = response["context"]["candidates"]
            .as_array()
            .expect("candidate rows should be an array");
        let first_candidate = candidates
            .first()
            .and_then(|candidate| candidate["text"].as_str());
        assert_eq!(
            first_candidate,
            Some(expected_first_candidate),
            "long byte-backed Jyutping input should preserve first-candidate quality for {long_input}: {response:?}"
        );
        assert!(
            metrics.prefix_fallback_views_visited <= prefix_fallback_visit_limit,
            "long byte-backed Jyutping input should not materialize every prefix fallback row for {long_input}: {metrics:?}"
        );
        assert!(
            metrics.sentence_entry_matches_collected <= 5_000,
            "long byte-backed Jyutping input should not materialize every sentence substring row for {long_input}: {metrics:?}"
        );
    }

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn m31_yune_web_hk2s_option_changes_real_asset_candidates() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create_with_schema("m31-browser-real-hk2s", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let traditional_state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!traditional_state.is_null());
    let traditional = process_input(traditional_state, "ngohaigo");
    assert_eq!(
        traditional["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())
    );
    unsafe { yune_web_cleanup(traditional_state) };

    let simplified_state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!simplified_state.is_null());
    let simplification =
        CString::new("simplification").expect("option name should be a valid C string");
    assert_eq!(
        unsafe { yune_web_set_option(simplified_state, simplification.as_ptr(), TRUE) },
        TRUE
    );
    let simplified = process_input(simplified_state, "ngohaigo");
    assert_eq!(
        simplified["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{7cfb}\u{4e2a}".to_owned())
    );
    assert_ne!(
        simplified["context"]["candidates"][0]["text"],
        traditional["context"]["candidates"][0]["text"]
    );

    unsafe { yune_web_cleanup(simplified_state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_real_assets_prefix_fallback_commits_consumed_span() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-real-prefix-fallback", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
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
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

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

    let committed = response_json(unsafe { yune_web_select_candidate(state, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}".to_owned())])
    );
    assert_eq!(
        committed["context"]["input"],
        Value::String("ri".to_owned())
    );
    assert_eq!(committed["status"]["is_composing"], Value::Bool(true));
    assert_ne!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}ri".to_owned())])
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn m28_partial_selection_real_assets_commits_only_consumed_span_and_recomposes() {
    let _guard = test_guard();
    let fixture: Value = serde_json::from_str(TYPEDUCK_V112_M28_PARTIAL_SELECTION)
        .expect("TypeDuck v1.1.2 M28 fixture should parse");
    let input = fixture["input"]
        .as_str()
        .expect("M28 fixture should capture input");
    let selected_text = fixture["selection_request"]["requested_candidate_text"]
        .as_str()
        .expect("M28 fixture should capture selected text");
    let selection_index = fixture["selection_request"]["actual_candidate_index"]
        .as_u64()
        .expect("M28 fixture should capture selected index") as usize;
    let remaining_input = fixture["captured_active_remaining_input_by_consumed_span"]
        .as_str()
        .expect("M28 fixture should capture active remaining input");
    let final_commit = fixture["captured_final_flow"]["final_commit_text"]
        .as_str()
        .expect("M28 fixture should capture final commit");
    let continuation_components = m28_oracle_continuation_components(&fixture);

    let runtime =
        YuneWebRuntime::create_with_schema("browser-real-m28-partial", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let composing = process_input(state, input);
    assert_eq!(
        composing["context"]["candidates"][selection_index]["text"],
        Value::String(selected_text.to_owned())
    );

    let selected = response_json(unsafe { yune_web_select_candidate(state, selection_index) });
    assert_eq!(
        selected["commits"],
        Value::Array(vec![Value::String(selected_text.to_owned())])
    );
    assert_ne!(
        selected["commits"],
        Value::Array(vec![Value::String(format!(
            "{selected_text}{remaining_input}"
        ))])
    );
    assert_eq!(
        selected["context"]["input"],
        Value::String(remaining_input.to_owned())
    );
    assert_eq!(selected["status"]["is_composing"], Value::Bool(true));

    let mut current = selected;
    let mut combined = current["commits"][0]
        .as_str()
        .expect("first commit should be text")
        .to_owned();
    for component in continuation_components {
        let component_index = current["context"]["candidates"]
            .as_array()
            .expect("remaining candidates should be an array")
            .iter()
            .position(|candidate| candidate["text"] == Value::String(component.clone()))
            .unwrap_or_else(|| {
                panic!("oracle continuation component {component} should remain selectable")
            });
        current = response_json(unsafe { yune_web_select_candidate(state, component_index) });
        assert_eq!(
            current["commits"],
            Value::Array(vec![Value::String(component.clone())])
        );
        combined.push_str(&component);
    }
    assert_eq!(combined, final_commit);
    assert_eq!(current["context"]["input"], Value::String(String::new()));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_m28_followup_space_partial_candidate_recomposes() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("m28-followup-space-partial", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    let key = CString::new("translator/enable_sentence").expect("custom key should be valid");
    let value = CString::new("false").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let composing = process_input(state, "caksijathaacoenggeoizi");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("測".to_owned())
    );

    let space = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        space["commits"],
        Value::Array(vec![Value::String("測".to_owned())])
    );
    assert_eq!(
        space["context"]["input"],
        Value::String("sijathaacoenggeoizi".to_owned())
    );
    assert_eq!(space["status"]["is_composing"], Value::Bool(true));
    assert_ne!(
        space["commits"],
        Value::Array(vec![Value::String("測sijathaacoenggeoizi".to_owned())])
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_m28_followup_upstream_style_phrase_prefix_ranking() {
    let _guard = test_guard();
    let fixture: Value = serde_json::from_str(M28_UPSTREAM_JYUTPING_COMPOSITION)
        .expect("M28 follow-up upstream Jyutping fixture should parse");
    let input = fixture["capture"]["target_input"]
        .as_str()
        .expect("fixture should capture target input");
    let expected_rows = fixture["auto_composition_on"]["candidate_rows"]
        .as_array()
        .expect("fixture should capture candidate rows");
    let expected_texts = expected_rows
        .iter()
        .map(|row| {
            row["text"]
                .as_str()
                .expect("candidate text should be present")
        })
        .collect::<Vec<_>>();

    let runtime =
        YuneWebRuntime::create_with_schema("m28-followup-upstream-ranking", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let response = process_input(state, input);
    let candidates = response["context"]["candidates"]
        .as_array()
        .expect("candidates should be present");
    let actual_texts = candidates
        .iter()
        .take(expected_texts.len())
        .map(|row| {
            row["text"]
                .as_str()
                .expect("candidate text should be present")
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual_texts, expected_texts,
        "Yune web adapter ranking should follow the accepted upstream Jyutping fixture"
    );

    let space = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        space["commits"],
        Value::Array(vec![Value::String(
            fixture["auto_composition_on"]["space_commit"]
                .as_str()
                .expect("fixture should capture Space commit")
                .to_owned()
        )])
    );
    assert_eq!(space["context"]["input"], Value::String(String::new()));

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_real_assets_correction_enabled_reorders_nri() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-real-correction-enabled", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
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
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let composing = process_input(state, "nri");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    let committed = response_json(unsafe { yune_web_select_candidate(state, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{4f60}".to_owned())])
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_real_assets_browser_defaults_keep_correction_nri_first() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create_with_schema(
        "browser-real-correction-browser-defaults",
        "jyut6ping3_mobile",
    );
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    for (key, value) in [
        ("menu/page_size", "6"),
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
            unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
            TRUE
        );
    }
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let composing = process_input(state, "nri");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_browser_app_assets_load_public_mobile_schema() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-app-public-mobile", "jyut6ping3_mobile");
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        !state.is_null(),
        "jyut6ping3_mobile should initialize from browser app assets"
    );

    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);
    let composing = process_input(state, "cak");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6e2c}".to_owned()),
        "public mobile schema should compose shipped Jyutping candidates"
    );

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));
    let sentence = process_input(state, "ngogokdak");
    assert_eq!(
        sentence["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{89ba}\u{5f97}".to_owned()),
        "clean browser app assets should compose multi-syllable Jyutping phrases"
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_browser_app_assets_compose_reported_multi_syllable_words() {
    // Locks down the user-reported live-demo regressions against the committed
    // public/schema byte-backed assets. A double-array trie collision had dropped
    // the toneless form of multi-syllable codes from the prism, so these returned
    // unrelated sentence-path candidates (e.g. litbiu -> 小斑啄木鳥). The expected
    // first candidates are TypeDuck's, matching the source-fallback/oracle path.
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-app-multi-syllable-words", "jyut6ping3_mobile");
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        !state.is_null(),
        "jyut6ping3_mobile should initialize from browser app assets"
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    for (input, expected) in [
        ("litbiu", "\u{5217}\u{8868}"),            // 列表
        ("ngojiu", "\u{6211}\u{8981}"),            // 我要
        ("ngaamngaam", "\u{5571}\u{5571}"),        // 啱啱
        ("caamhaau", "\u{53c3}\u{8003}"),          // 參考
        ("waakze", "\u{6216}\u{8005}"),            // 或者
        ("honangwui", "\u{53ef}\u{80fd}\u{6703}"), // 可能會
    ] {
        drop(response_json(unsafe {
            yune_web_process_key(state, 0xff1b, 0)
        }));
        let response = process_input(state, input);
        assert_eq!(
            response["context"]["candidates"][0]["text"],
            Value::String(expected.to_owned()),
            "committed byte-backed Jyutping assets should map {input} to its canonical word: {response:?}"
        );
    }

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_keyboard_profile_optouts_keep_jyutping_core_behavior() {
    let _guard = test_guard();
    let fixture: Value = serde_json::from_str(M28_UPSTREAM_JYUTPING_COMPOSITION)
        .expect("M28 follow-up upstream Jyutping fixture should parse");
    let m28_input = fixture["capture"]["target_input"]
        .as_str()
        .expect("fixture should capture target input");
    let m28_expected = fixture["auto_composition_on"]["candidate_rows"][0]["text"]
        .as_str()
        .expect("fixture should capture top M28 candidate");

    let runtime = YuneWebRuntime::create_with_schema(
        "browser-app-keyboard-profile-optouts",
        "jyut6ping3_mobile",
    );
    runtime.write_browser_app_assets();
    patch_browser_app_keyboard_profile(&runtime);

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        !state.is_null(),
        "jyut6ping3_mobile should initialize from keyboard-profile assets"
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let composing = process_input(state, "cak");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6e2c}".to_owned()),
        "keyboard profile should keep normal Jyutping candidate output"
    );

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));
    let phrase = process_input(state, "ngogokdak");
    assert_eq!(
        phrase["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{89ba}\u{5f97}".to_owned()),
        "keyboard profile should keep multi-syllable sentence composition"
    );

    for (input, expected) in [
        ("litbiu", "\u{5217}\u{8868}"),
        ("ngojiu", "\u{6211}\u{8981}"),
        ("honangwui", "\u{53ef}\u{80fd}\u{6703}"),
    ] {
        drop(response_json(unsafe {
            yune_web_process_key(state, 0xff1b, 0)
        }));
        let response = process_input(state, input);
        assert_eq!(
            response["context"]["candidates"][0]["text"],
            Value::String(expected.to_owned()),
            "keyboard profile should preserve reported matching regression guard for {input}: {response:?}"
        );
    }

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));
    let m28_response = process_input(state, m28_input);
    assert_eq!(
        m28_response["context"]["candidates"][0]["text"],
        Value::String(m28_expected.to_owned()),
        "keyboard profile should preserve M28 follow-up phrase/ranking guard"
    );

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));
    let lookup = process_input(state, "zouhapci");
    let candidates = lookup["context"]["candidates"]
        .as_array()
        .expect("candidate page should be an array");
    let rich_lookup_marker = "\u{000c}\r1,";
    assert!(
        candidates.iter().all(|candidate| candidate["comment"]
            .as_str()
            .is_none_or(|comment| !comment.contains(rich_lookup_marker))),
        "keyboard profile intentionally drops TypeDuck dictionary-panel rich comments: {lookup:?}"
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_browser_app_assets_load_jyutping_mandarin_pinyin_reverse_lookup() {
    let _guard = test_guard();
    let desktop_schema =
        fs::read_to_string(browser_app_schema_root().join("jyut6ping3.schema.yaml"))
            .expect("desktop Jyutping browser schema should be readable");
    assert!(
        !desktop_schema.contains("affix_segmentor@reverse_lookup"),
        "desktop Jyutping schema should not keep the vestigial bare-grave reverse_lookup segmentor"
    );
    assert!(
        !desktop_schema.contains("\nreverse_lookup:"),
        "desktop Jyutping schema should remove the vestigial bare-grave reverse_lookup block"
    );
    assert!(desktop_schema.contains("prefix: \"`vl\""));
    assert!(desktop_schema.contains("prefix: \"`vc\""));
    let loengfan_segmentor = desktop_schema
        .find("affix_segmentor@loengfan")
        .expect("desktop schema should keep Loengfan lookup");
    let cangjie_segmentor = desktop_schema
        .find("affix_segmentor@cangjie")
        .expect("desktop schema should keep Cangjie lookup");
    let luna_segmentor = desktop_schema
        .find("affix_segmentor@luna_pinyin")
        .expect("desktop schema should keep Luna lookup");
    assert!(
        loengfan_segmentor < luna_segmentor && cangjie_segmentor < luna_segmentor,
        "specific `vl/`vc segmentors should run before the bare-grave Luna catch-all"
    );

    let runtime = YuneWebRuntime::create_with_schema(
        "browser-app-jyutping-pinyin-reverse",
        "jyut6ping3_mobile",
    );
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        !state.is_null(),
        "jyut6ping3_mobile should initialize from browser app assets"
    );

    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let reverse = process_input(state, "`zhe");
    let reverse_texts = reverse["context"]["candidates"]
        .as_array()
        .expect("reverse lookup candidates should be an array")
        .iter()
        .map(|candidate| candidate["text"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();
    assert!(
        reverse_texts.contains(&"\u{9019}"),
        "jyut6ping3_mobile reverse lookup should expose \u{9019} for bare `zhe, got {reverse_texts:?}"
    );

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));
    for input in ["`lai", "`ci", "`xi", "`re"] {
        let reverse = process_input(state, input);
        let candidates = reverse["context"]["candidates"]
            .as_array()
            .expect("reverse lookup overlap candidates should be an array");
        assert!(
            !candidates.is_empty(),
            "{input} should route to bare-grave luna_pinyin instead of being stolen by secondary lookup prefixes"
        );
        drop(response_json(unsafe {
            yune_web_process_key(state, 0xff1b, 0)
        }));
    }

    let normal = process_input(state, "nei");
    assert_eq!(
        normal["context"]["candidates"][0]["text"],
        Value::String("\u{4f60}".to_owned())
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_real_assets_preserve_profile_full_shape_state_labels() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-real-state-labels", "jyut6ping3_mobile");
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
fn yune_web_adapter_deploys_browser_real_assets_after_customize() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-real-customize-deploy", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let config_id = CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
    let key = CString::new("menu/page_size").expect("custom key should be valid");
    let value = CString::new("6").expect("custom value should be valid");
    assert_eq!(
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_real_assets_page_size_customize_limits_context_page() {
    let _guard = test_guard();
    for page_size in [3, 9] {
        let runtime = YuneWebRuntime::create_with_schema(
            &format!("browser-real-page-size-{page_size}"),
            "jyut6ping3_mobile",
        );
        runtime.write_browser_real_assets();

        let state = unsafe {
            yune_web_init(
                runtime.shared_c.as_ptr(),
                runtime.user_c.as_ptr(),
                runtime.schema_id_c.as_ptr(),
            )
        };
        assert!(!state.is_null());

        let config_id =
            CString::new("jyut6ping3_mobile.schema").expect("config id should be valid");
        let key = CString::new("menu/page_size").expect("custom key should be valid");
        let value = CString::new(page_size.to_string()).expect("custom value should be valid");
        assert_eq!(
            unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
            TRUE
        );
        assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

        let composing = process_input(state, "hai");
        let candidates = composing["context"]["candidates"]
            .as_array()
            .expect("candidate page should be an array");
        assert_eq!(composing["context"]["page_size"], Value::from(page_size));
        assert_eq!(
            candidates.len(),
            page_size,
            "browser adapter should expose exactly menu/page_size rows for high-candidate input"
        );

        let next_page = response_json(unsafe { yune_web_flip_page(state, FALSE) });
        let next_candidates = next_page["context"]["candidates"]
            .as_array()
            .expect("next candidate page should be an array");
        assert_eq!(next_page["context"]["page_size"], Value::from(page_size));
        assert!(
            next_candidates.len() <= page_size,
            "next page must not exceed menu/page_size"
        );

        unsafe { yune_web_cleanup(state) };
        runtime.remove();
    }
}

#[test]
fn yune_web_adapter_real_assets_sentence_mode_commits_multisyllable_phrase() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-real-sentence-mode", "jyut6ping3_mobile");
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
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
        unsafe { yune_web_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
        TRUE
    );
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);
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
        composing = response_json(unsafe { yune_web_process_key(state, key as i32, 0) });
    }
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())
    );

    let committed = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    let mut composing = Value::Null;
    for key in "ngohaig".chars() {
        composing = response_json(unsafe { yune_web_process_key(state, key as i32, 0) });
    }
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())
    );

    let committed = response_json(unsafe { yune_web_process_key(state, ' ' as i32, 0) });
    assert_eq!(
        committed["commits"],
        Value::Array(vec![Value::String("\u{6211}\u{4fc2}\u{500b}".to_owned())])
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_real_assets_emit_oracle_dictionary_panel_comments() {
    let _guard = test_guard();
    let runtime =
        YuneWebRuntime::create_with_schema("browser-real-dictionary-comments", "jyut6ping3_mobile");
    if let Err(reason) = rich_dictionary_comment_oracle_build_status() {
        eprintln!(
            "SKIP yune_web_adapter_real_assets_emit_oracle_dictionary_panel_comments: {reason}. \
             This integration test does not pass against the degraded three-column fallback. \
             Clean-checkout byte parity is covered by `cargo test -p yune-core --test cantonese_parity`."
        );
        runtime.remove();
        return;
    }
    runtime.write_browser_real_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let mut composing = Value::Null;
    for key in "nei".chars() {
        composing = response_json(unsafe { yune_web_process_key(state, key as i32, 0) });
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

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_browser_app_assets_enrich_visible_lookup_candidates() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create_with_schema(
        "browser-app-visible-lookup-candidates",
        "jyut6ping3_mobile",
    );
    runtime.write_browser_app_assets();

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());

    let composing = process_input(state, "zouhapci");
    let candidates = composing["context"]["candidates"]
        .as_array()
        .expect("candidate page should be an array");

    for (text, code) in [
        ("\u{7d44}\u{5408}", "zou2hap6"),
        ("\u{505a}", "zou6"),
        ("\u{65e9}", "zou2"),
        ("\u{7d44}", "zou2"),
        ("\u{79df}", "zou1"),
    ] {
        let candidate = candidates
            .iter()
            .find(|candidate| candidate["text"] == Value::String(text.to_owned()))
            .unwrap_or_else(|| panic!("{text} should be visible for zouhapci: {candidates:?}"));
        let comment = candidate["comment"]
            .as_str()
            .unwrap_or_else(|| panic!("{text} should have a string comment: {candidate:?}"));
        assert!(
            comment.contains(&format!("\u{000c}\r1,{text},{code},")),
            "{text} should carry rich dictionary lookup bytes; got {comment:?}"
        );
    }

    write_red07_comment_evidence_if_requested(candidates);

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

#[test]
fn yune_web_adapter_comments_intact_keyboard_profile_skips_translator_lookup_payload() {
    let _guard = test_guard();
    let runtime = YuneWebRuntime::create_with_schema(
        "browser-app-comments-intact-keyboard-red08",
        "jyut6ping3_mobile",
    );
    runtime.write_browser_app_assets();
    patch_browser_app_comments_intact_keyboard_profile(&runtime);

    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
    assert_eq!(unsafe { yune_web_deploy(state) }, TRUE);

    let inspector = CString::new("yune_inspector").expect("option should be valid");
    assert_eq!(
        unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
        TRUE
    );

    let composing = process_input(state, "cak");
    assert_eq!(
        composing["context"]["candidates"][0]["text"],
        Value::String("\u{6e2c}".to_owned()),
        "comments-intact keyboard profile should keep normal Jyutping candidate output"
    );

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));
    let phrase = process_input(state, "ngogokdak");
    assert_eq!(
        phrase["context"]["candidates"][0]["text"],
        Value::String("\u{6211}\u{89ba}\u{5f97}".to_owned()),
        "comments-intact keyboard profile should keep multi-syllable sentence composition"
    );

    for (input, expected) in [
        ("litbiu", "\u{5217}\u{8868}"),
        ("ngojiu", "\u{6211}\u{8981}"),
        ("honangwui", "\u{53ef}\u{80fd}\u{6703}"),
    ] {
        drop(response_json(unsafe {
            yune_web_process_key(state, 0xff1b, 0)
        }));
        let response = process_input(state, input);
        assert_eq!(
            response["context"]["candidates"][0]["text"],
            Value::String(expected.to_owned()),
            "comments-intact keyboard profile should preserve matching regression guard for {input}: {response:?}"
        );
    }

    drop(response_json(unsafe {
        yune_web_process_key(state, 0xff1b, 0)
    }));
    let composing = process_input(state, "zouhapci");
    let candidates = composing["context"]["candidates"]
        .as_array()
        .expect("candidate page should be an array");
    let candidate = candidates
        .iter()
        .find(|candidate| candidate["text"] == Value::String("\u{7d44}\u{5408}".to_owned()))
        .unwrap_or_else(|| panic!("組合 should be visible for zouhapci: {candidates:?}"));
    let comment = candidate["comment"]
        .as_str()
        .unwrap_or_else(|| panic!("組合 should have a string comment: {candidate:?}"));
    assert!(
        comment.contains("\u{000c}\r1,\u{7d44}\u{5408},zou2hap6,"),
        "comments-intact keyboard profile should keep rich lookup comment bytes: {comment:?}"
    );

    let owner_rows = composing["context"]["debug"]["storage"]["memory_owner_rows"]
        .as_array()
        .expect("inspector should expose memory owner rows");
    assert!(
        owner_rows
            .iter()
            .any(|row| row["owner"] == "dictionary_lookup_filter.lookup_records"),
        "comments-intact profile should keep dictionary lookup records for rich comments: {owner_rows:?}"
    );
    let translator_lookup_rows = owner_rows
        .iter()
        .filter(|row| row["owner"] == "compact_table.lookup_records")
        .collect::<Vec<_>>();
    assert!(
        !translator_lookup_rows.is_empty(),
        "comments-intact profile should report translator lookup payload rows: {owner_rows:?}"
    );
    assert!(
        translator_lookup_rows.iter().all(|row| row["class"] == "shared"
            && row["estimated_bytes"].as_u64() == Some(0)),
        "comments-intact profile should report no retained translator lookup payload: {owner_rows:?}"
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

fn write_red07_comment_evidence_if_requested(candidates: &[Value]) {
    let Some(evidence_root) = red07_evidence_root_from_env() else {
        return;
    };
    fs::create_dir_all(&evidence_root).expect("RED-07 evidence directory should be writable");
    let sample_texts = [
        "\u{7d44}\u{5408}",
        "\u{505a}",
        "\u{65e9}",
        "\u{7d44}",
        "\u{79df}",
    ];
    let samples = sample_texts
        .iter()
        .filter_map(|text| {
            let candidate = candidates
                .iter()
                .find(|candidate| candidate["text"] == Value::String((*text).to_owned()))?;
            Some(json!({
                "text": text,
                "comment": candidate["comment"],
                "has_dictionary_payload": candidate["comment"]
                    .as_str()
                    .is_some_and(|comment| comment.contains('\u{000c}') && comment.contains("\r1,")),
            }))
        })
        .collect::<Vec<_>>();
    let payload = json!({
        "schema": "jyut6ping3_mobile",
        "input": "zouhapci",
        "profile": "comments-intact keyboard profile",
        "evidence": "visible candidates retain TypeDuck dictionary-panel rich comment bytes after RED-07 byte-backed lookup storage",
        "samples": samples,
    });
    fs::write(
        evidence_root.join("rich-comment-zouhapci.json"),
        serde_json::to_vec_pretty(&payload).expect("RED-07 evidence JSON should serialize"),
    )
    .expect("RED-07 evidence JSON should be writable");
}

fn typeduck_oracle_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/typeduck-oracle/v1.1.2")
}

fn browser_app_schema_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/yune-web/public/schema")
}

fn public_demo_schema_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/yune-web/public-demo/dist/schema")
}

fn web03_evidence_root_from_env() -> Option<PathBuf> {
    std::env::var_os("YUNE_WEB03_EVIDENCE_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(path)
            }
        })
}

fn red07_evidence_root_from_env() -> Option<PathBuf> {
    std::env::var_os("YUNE_M47_RED07_EVIDENCE_DIR")
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../..")
                    .join(path)
            }
        })
}

fn deploy_public_demo_schema(runtime: &YuneWebRuntime, schema_id: &str) {
    let mut traits = empty_rime_traits();
    traits.shared_data_dir = runtime.shared_c.as_ptr();
    traits.user_data_dir = runtime.user_c.as_ptr();
    unsafe { RimeDeployerInitialize(&traits) };
    let schema_file =
        CString::new(format!("{schema_id}.schema.yaml")).expect("schema file name should be valid");
    assert_eq!(
        RimeDeploySchema(schema_file.as_ptr()),
        TRUE,
        "{schema_id} should deploy from public-demo assets"
    );
}

fn copy_clean_schema_sources(source_root: &Path, destination_root: &Path) {
    for entry in fs::read_dir(source_root).expect("source schema dir should be readable") {
        let entry = entry.expect("source schema entry should be readable");
        let source = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if source.is_dir() {
            if file_name == "build" {
                continue;
            }
            copy_clean_schema_sources(&source, &destination_root.join(file_name.as_ref()));
            continue;
        }
        if source.extension().and_then(|extension| extension.to_str()) == Some("bin") {
            continue;
        }
        let destination = destination_root.join(file_name.as_ref());
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("destination parent should be created");
        }
        fs::copy(&source, destination).expect("schema source file should be copied");
    }
}

fn assert_dictionary_rebuilt_from_source(
    reports: &[yune_rime_api::WorkspaceDictionaryRebuildReport],
    dictionary_id: &str,
) {
    assert!(
        reports
            .iter()
            .any(|report| report.dictionary_id == dictionary_id
                && report.report.table == RimeDictArtifactStatus::Rebuilt
                && report.report.prism == RimeDictArtifactStatus::Rebuilt
                && report.report.reverse == RimeDictArtifactStatus::Rebuilt),
        "{dictionary_id} should have a full Rebuilt report: {reports:?}"
    );
}

fn workspace_rebuild_reports_json(
    reports: &[yune_rime_api::WorkspaceDictionaryRebuildReport],
) -> Value {
    Value::Array(
        reports
            .iter()
            .map(|report| {
                json!({
                    "schema_id": report.schema_id,
                    "dictionary_id": report.dictionary_id,
                    "table": format!("{:?}", report.report.table),
                    "prism": format!("{:?}", report.report.prism),
                    "reverse": format!("{:?}", report.report.reverse),
                })
            })
            .collect(),
    )
}

fn workspace_rebuild_reports_csv(
    reports: &[yune_rime_api::WorkspaceDictionaryRebuildReport],
) -> String {
    let mut output = String::from("schema_id,dictionary_id,table,prism,reverse\n");
    for report in reports {
        output.push_str(&format!(
            "{},{},{:?},{:?},{:?}\n",
            csv_field(&report.schema_id),
            csv_field(&report.dictionary_id),
            report.report.table,
            report.report.prism,
            report.report.reverse,
        ));
    }
    output
}

fn assert_schema_storage_byte_backed(schema_id: &str, storage: &Value) {
    assert_eq!(
        storage["source_fallback"].as_bool(),
        Some(false),
        "{schema_id} should not report dictionary source fallback: {storage:?}"
    );
    assert!(
        storage["source_fallbacks"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "{schema_id} source fallback rows should be empty: {storage:?}"
    );
    let selected = storage["selected"]
        .as_array()
        .expect("selected storage rows should be an array");
    assert!(
        !selected.is_empty(),
        "{schema_id} should expose at least one selected storage row"
    );
    for row in selected {
        assert_eq!(
            row["selected_storage"].as_str(),
            Some("byte_backed"),
            "{schema_id} should select byte-backed storage: {row:?}"
        );
        assert!(
            row["byte_source_len"].as_u64().unwrap_or_default() > 0,
            "{schema_id} byte-backed row should carry source bytes: {row:?}"
        );
    }
}

fn storage_selected_csv(rows: &[Value]) -> String {
    let mut output = String::from(
        "translator_index,translator,owner,selected_storage,mapping_mode,is_marisa_backed,byte_source_len,stored_entry_count\n",
    );
    for row in rows {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            row["translator_index"].as_u64().unwrap_or_default(),
            csv_field(row["translator"].as_str().unwrap_or_default()),
            csv_field(row["owner"].as_str().unwrap_or_default()),
            csv_field(row["selected_storage"].as_str().unwrap_or_default()),
            csv_field(row["mapping_mode"].as_str().unwrap_or_default()),
            row["is_marisa_backed"].as_bool().unwrap_or_default(),
            row["byte_source_len"].as_u64().unwrap_or_default(),
            row["stored_entry_count"].as_u64().unwrap_or_default(),
        ));
    }
    output
}

fn schema_storage_selected_csv(rows: &[Value]) -> String {
    let mut output = String::from(
        "schema_id,translator_index,translator,owner,selected_storage,mapping_mode,is_marisa_backed,byte_source_len,stored_entry_count\n",
    );
    for row in rows {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}\n",
            csv_field(row["schema_id"].as_str().unwrap_or_default()),
            row["translator_index"].as_u64().unwrap_or_default(),
            csv_field(row["translator"].as_str().unwrap_or_default()),
            csv_field(row["owner"].as_str().unwrap_or_default()),
            csv_field(row["selected_storage"].as_str().unwrap_or_default()),
            csv_field(row["mapping_mode"].as_str().unwrap_or_default()),
            row["is_marisa_backed"].as_bool().unwrap_or_default(),
            row["byte_source_len"].as_u64().unwrap_or_default(),
            row["stored_entry_count"].as_u64().unwrap_or_default(),
        ));
    }
    output
}

fn memory_owner_rows_csv(rows: &[Value]) -> String {
    let mut output = String::from("owner,class,estimated_bytes,item_count,storage,notes\n");
    for row in rows {
        output.push_str(&format!(
            "{},{},{},{},{},{}\n",
            csv_field(row["owner"].as_str().unwrap_or_default()),
            csv_field(row["class"].as_str().unwrap_or_default()),
            row["estimated_bytes"].as_u64().unwrap_or_default(),
            row["item_count"].as_u64().unwrap_or_default(),
            csv_field(row["storage"].as_str().unwrap_or_default()),
            csv_field(row["notes"].as_str().unwrap_or_default()),
        ));
    }
    output
}

fn schema_memory_owner_rows_csv(rows: &[Value]) -> String {
    let mut output =
        String::from("schema_id,owner,class,estimated_bytes,item_count,storage,notes\n");
    for row in rows {
        output.push_str(&format!(
            "{},{},{},{},{},{},{}\n",
            csv_field(row["schema_id"].as_str().unwrap_or_default()),
            csv_field(row["owner"].as_str().unwrap_or_default()),
            csv_field(row["class"].as_str().unwrap_or_default()),
            row["estimated_bytes"].as_u64().unwrap_or_default(),
            row["item_count"].as_u64().unwrap_or_default(),
            csv_field(row["storage"].as_str().unwrap_or_default()),
            csv_field(row["notes"].as_str().unwrap_or_default()),
        ));
    }
    output
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn compiled_asset_inventory_csv(runtime: &YuneWebRuntime) -> String {
    let mut output = String::from("scope,path,exists,bytes,header\n");
    let roots = [
        ("shared", runtime.shared.clone()),
        ("user_build", runtime.user.join("build")),
    ];
    for (scope, root) in roots {
        for file_name in WEB03_COMPILED_SCHEMA_ASSETS {
            let path = root.join(file_name);
            let (exists, bytes, header) = compiled_asset_inventory_row(&path);
            output.push_str(&format!(
                "{},{},{},{},{}\n",
                scope,
                csv_field(file_name),
                exists,
                bytes,
                csv_field(&header),
            ));
        }
    }
    output
}

fn compiled_asset_inventory_for_root_csv(root: &Path, file_names: &[&str]) -> String {
    let mut output = String::from("path,exists,bytes,header\n");
    for file_name in file_names {
        let (exists, bytes, header) = compiled_asset_inventory_row(&root.join(file_name));
        output.push_str(&format!(
            "{},{},{},{}\n",
            csv_field(file_name),
            exists,
            bytes,
            csv_field(&header),
        ));
    }
    output
}

fn compiled_asset_inventory_row(path: &Path) -> (bool, u64, String) {
    if !path.is_file() {
        return (false, 0, String::new());
    }
    let bytes = fs::read(path).expect("compiled asset should be readable");
    let header_len = bytes.len().min(32);
    let header = String::from_utf8_lossy(&bytes[..header_len]).replace('\0', "\\0");
    (true, bytes.len() as u64, header)
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
        "yune-rime-api-yune-web-{label}-{}-{nanos}",
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

fn copy_asset_if_exists(schema_root: &Path, destination_root: &Path, relative_path: &str) {
    let source = schema_root.join(relative_path);
    if source.is_file() {
        let destination = destination_root.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("asset parent directory should be created");
        }
        fs::copy(source, destination).expect("browser optional asset should be copied");
    }
}

fn copy_dir_contents(source_root: &Path, destination_root: &Path) {
    for entry in fs::read_dir(source_root).expect("source asset directory should be readable") {
        let entry = entry.expect("source asset entry should be readable");
        let source = entry.path();
        let destination = destination_root.join(entry.file_name());
        if source.is_dir() {
            fs::create_dir_all(&destination)
                .expect("destination asset directory should be created");
            copy_dir_contents(&source, &destination);
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent).expect("asset parent directory should be created");
            }
            fs::copy(&source, &destination).expect("public-demo asset should be copied");
        }
    }
}
