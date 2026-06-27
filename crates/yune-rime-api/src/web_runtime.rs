use std::{
    ffi::{CStr, CString},
    mem,
    os::raw::{c_char, c_int},
    ptr,
    time::Duration,
};

use serde_json::{json, Value as JsonValue};
use serde_yaml::Value;
use yune_core::{
    AiCandidateProvider, AiConfidence, AiOffReason, AiPrivacyPolicy, AiResult, CandidateSource,
    LocalModelProvider, LocalModelRule, LOCAL_MODEL_PROVIDER_NAME,
};

use crate::{
    install_config_root, rime_get_api, rime_levers_get_api, Bool, RimeCandidate, RimeCommit,
    RimeComposition, RimeConfig, RimeContext, RimeLeversApi, RimeMenu, RimeSessionId, RimeStatus,
    RimeTraits, FALSE, TRUE,
};

use crate::session::{
    session_candidates_snapshot, session_inspector_snapshot, session_web_diagnostics_snapshot,
    with_session,
};

const AI_BUDGET: Duration = Duration::from_millis(25);
const LOCAL_AI_SOURCE_LABEL: &str = "ai:local";
const INSPECTOR_OPTION: &str = "yune_inspector";

#[repr(C)]
pub struct YuneWebState {
    session_id: RimeSessionId,
    shared_data_dir: CString,
    user_data_dir: CString,
    schema_id: CString,
    initialized: bool,
    ai_enabled: bool,
    ai_provider: Option<LocalModelProvider>,
    inspector_enabled: bool,
}

#[repr(C)]
pub struct YuneWebResponse {
    handled: Bool,
    json: CString,
}

/// # Safety
/// Pointers must be valid NUL-terminated strings for the duration of the call.
#[no_mangle]
pub unsafe extern "C" fn yune_web_init(
    shared_data_dir: *const c_char,
    user_data_dir: *const c_char,
    schema_id: *const c_char,
) -> *mut YuneWebState {
    let Some(shared_data_dir) = cstring_from_ptr(shared_data_dir) else {
        return ptr::null_mut();
    };
    let Some(user_data_dir) = cstring_from_ptr(user_data_dir) else {
        return ptr::null_mut();
    };
    let Some(schema_id) = cstring_from_ptr(schema_id) else {
        return ptr::null_mut();
    };
    let Some(api) = api_table() else {
        return ptr::null_mut();
    };
    let Some(setup) = api.setup else {
        return ptr::null_mut();
    };
    let Some(initialize) = api.initialize else {
        return ptr::null_mut();
    };
    let Some(create_session) = api.create_session else {
        return ptr::null_mut();
    };
    let Some(select_schema) = api.select_schema else {
        return ptr::null_mut();
    };
    if !has_preloaded_runtime_assets(&shared_data_dir, &user_data_dir, &schema_id) {
        return ptr::null_mut();
    }

    let mut traits = empty_traits();
    traits.shared_data_dir = shared_data_dir.as_ptr();
    traits.user_data_dir = user_data_dir.as_ptr();

    unsafe { setup(&traits) };
    unsafe { initialize(&traits) };

    let session_id = create_session();
    if session_id == 0 {
        finalize_api(api);
        return ptr::null_mut();
    }

    if unsafe { select_schema(session_id, schema_id.as_ptr()) } != TRUE {
        destroy_session(api, session_id);
        finalize_api(api);
        return ptr::null_mut();
    }

    Box::into_raw(Box::new(YuneWebState {
        session_id,
        shared_data_dir,
        user_data_dir,
        schema_id,
        initialized: true,
        ai_enabled: false,
        ai_provider: None,
        inspector_enabled: false,
    }))
}

/// # Safety
/// `state` must be a live pointer returned by `yune_web_init`.
#[no_mangle]
pub unsafe extern "C" fn yune_web_process_key(
    state: *mut YuneWebState,
    keycode: c_int,
    mask: c_int,
) -> *mut YuneWebResponse {
    operate(state, |api, state| {
        let Some(process_key) = api.process_key else {
            return response(
                FALSE,
                vec![],
                None,
                None,
                Some("process_key API unavailable"),
            );
        };
        let session_id = state.session_id;
        let handled = process_key(session_id, keycode, mask);
        response_from_session(api, session_id, handled, None, state.inspector_enabled)
    })
}

/// # Safety
/// `state` must be a live pointer returned by `yune_web_init`.
#[no_mangle]
pub unsafe extern "C" fn yune_web_select_candidate(
    state: *mut YuneWebState,
    index: usize,
) -> *mut YuneWebResponse {
    operate(state, |api, state| {
        let Some(select_candidate) = api.select_candidate_on_current_page else {
            return response(
                FALSE,
                vec![],
                None,
                None,
                Some("select_candidate_on_current_page API unavailable"),
            );
        };
        let session_id = state.session_id;
        let handled = select_candidate(session_id, index);
        response_from_session(api, session_id, handled, None, state.inspector_enabled)
    })
}

/// # Safety
/// `state` must be a live pointer returned by `yune_web_init`.
#[no_mangle]
pub unsafe extern "C" fn yune_web_delete_candidate(
    state: *mut YuneWebState,
    index: usize,
) -> *mut YuneWebResponse {
    operate(state, |api, state| {
        let Some(delete_candidate) = api.delete_candidate_on_current_page else {
            return response(
                FALSE,
                vec![],
                None,
                None,
                Some("delete_candidate_on_current_page API unavailable"),
            );
        };
        let session_id = state.session_id;
        let handled = delete_candidate(session_id, index);
        response_from_session(api, session_id, handled, None, state.inspector_enabled)
    })
}

/// # Safety
/// `state` must be a live pointer returned by `yune_web_init`.
#[no_mangle]
pub unsafe extern "C" fn yune_web_flip_page(
    state: *mut YuneWebState,
    backward: Bool,
) -> *mut YuneWebResponse {
    operate(state, |api, state| {
        let Some(change_page) = api.change_page else {
            return response(
                FALSE,
                vec![],
                None,
                None,
                Some("change_page API unavailable"),
            );
        };
        let session_id = state.session_id;
        let handled = change_page(session_id, backward);
        response_from_session(api, session_id, handled, None, state.inspector_enabled)
    })
}

/// # Safety
/// `state` must be null or a live pointer returned by `yune_web_init`.
#[no_mangle]
pub unsafe extern "C" fn yune_web_deploy(state: *mut YuneWebState) -> Bool {
    if state.is_null() {
        return FALSE;
    }
    let Some(api) = api_table() else {
        return FALSE;
    };
    let Some(deployer_initialize) = api.deployer_initialize else {
        return FALSE;
    };
    let Some(deploy_schema) = api.deploy_schema else {
        return FALSE;
    };
    let state = unsafe { &*state };
    if !state.initialized || state.session_id == 0 {
        return FALSE;
    }
    let mut traits = empty_traits();
    traits.shared_data_dir = state.shared_data_dir.as_ptr();
    traits.user_data_dir = state.user_data_dir.as_ptr();
    unsafe { deployer_initialize(&traits) };
    let Ok(schema_file) =
        CString::new(format!("{}.schema.yaml", state.schema_id.to_string_lossy()))
    else {
        return FALSE;
    };
    if deploy_schema(schema_file.as_ptr()) != TRUE {
        return FALSE;
    }
    let Some(select_schema) = api.select_schema else {
        return FALSE;
    };
    unsafe { select_schema(state.session_id, state.schema_id.as_ptr()) }
}

/// # Safety
/// `state` must be live, and string pointers must be valid NUL-terminated strings.
#[no_mangle]
pub unsafe extern "C" fn yune_web_customize(
    state: *mut YuneWebState,
    config_id: *const c_char,
    key: *const c_char,
    value: *const c_char,
) -> Bool {
    if state.is_null() {
        return FALSE;
    }
    let Some(config_id) = cstring_from_ptr(config_id) else {
        return FALSE;
    };
    let Some(key) = cstring_from_ptr(key) else {
        return FALSE;
    };
    let Some(value) = cstring_from_ptr(value) else {
        return FALSE;
    };
    let levers = rime_levers_get_api().cast::<RimeLeversApi>();
    if levers.is_null() {
        return FALSE;
    }
    let levers = unsafe { &*levers };
    let Some(init) = levers.custom_settings_init else {
        return FALSE;
    };
    let Some(destroy) = levers.custom_settings_destroy else {
        return FALSE;
    };
    let Some(load) = levers.load_settings else {
        return FALSE;
    };
    let Some(save) = levers.save_settings else {
        return FALSE;
    };
    let Some(customize_string) = levers.customize_string else {
        return FALSE;
    };
    let generator = CString::new("yune-web").expect("static generator is valid CString");
    let settings = unsafe { init(config_id.as_ptr(), generator.as_ptr()) };
    if settings.is_null() {
        return FALSE;
    }
    unsafe { load(settings) };
    let key_string = key.as_c_str().to_string_lossy();
    let value_string = value.as_c_str().to_string_lossy();
    let customized = if key_string == "translator/dictionary_exclude" {
        let Some(exclude_list) = dictionary_exclude_config_value(&value_string) else {
            unsafe { destroy(settings) };
            return FALSE;
        };
        let Some(customize_item) = levers.customize_item else {
            unsafe { destroy(settings) };
            return FALSE;
        };
        let mut config = RimeConfig {
            ptr: ptr::null_mut(),
        };
        if unsafe { install_config_root(&mut config, exclude_list) } != TRUE {
            unsafe { destroy(settings) };
            return FALSE;
        }
        let customized = unsafe { customize_item(settings, key.as_ptr(), &mut config) } == TRUE;
        let _ = unsafe { crate::RimeConfigClose(&mut config) };
        customized
    } else {
        (unsafe { customize_string(settings, key.as_ptr(), value.as_ptr()) }) == TRUE
    };
    let saved = customized && unsafe { save(settings) } == TRUE;
    unsafe { destroy(settings) };
    bool_to_rime(saved)
}

/// # Safety
/// `state` must be live, and `option` must be a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn yune_web_set_option(
    state: *mut YuneWebState,
    option: *const c_char,
    value: Bool,
) -> Bool {
    if state.is_null() {
        return FALSE;
    }
    let Some(option) = cstring_from_ptr(option) else {
        return FALSE;
    };
    let state = unsafe { &mut *state };
    if !state.initialized || state.session_id == 0 {
        return FALSE;
    }
    if option.as_c_str().to_str().ok() == Some(INSPECTOR_OPTION) {
        state.inspector_enabled = value != FALSE;
        return TRUE;
    }
    let Some(api) = api_table() else {
        return FALSE;
    };
    let Some(set_option) = api.set_option else {
        return FALSE;
    };
    unsafe {
        set_option(
            state.session_id,
            option.as_ptr(),
            bool_to_rime(value != FALSE),
        )
    };
    TRUE
}

/// # Safety
/// `state` must be live.
#[no_mangle]
pub unsafe extern "C" fn yune_web_set_ai_enabled(state: *mut YuneWebState, enabled: Bool) -> Bool {
    if state.is_null() {
        return FALSE;
    }
    let state = unsafe { &mut *state };
    if !state.initialized || state.session_id == 0 {
        return FALSE;
    }
    state.ai_enabled = enabled != FALSE;
    if state.ai_enabled {
        return TRUE;
    }

    let cleared = with_session(state.session_id, |session| {
        let input = session.engine.context().composition.input.clone();
        session
            .engine
            .stage_ai_result(AiResult::off(input, AiOffReason::Privacy));
        true
    });
    if cleared == TRUE {
        state.ai_provider = None;
    }
    cleared
}

/// # Safety
/// `state` must be a live pointer returned by `yune_web_init`.
#[no_mangle]
pub unsafe extern "C" fn yune_web_stage_ai(state: *mut YuneWebState) -> *mut YuneWebResponse {
    if state.is_null() {
        return ptr::null_mut();
    }
    let Some(api) = api_table() else {
        return response(FALSE, vec![], None, None, Some("RimeApi unavailable"));
    };
    let state = unsafe { &mut *state };
    if !state.initialized || state.session_id == 0 {
        return response(
            FALSE,
            vec![],
            None,
            None,
            Some("YuneWeb state is not initialized"),
        );
    }
    let session_id = state.session_id;
    if !state.ai_enabled {
        return response_from_session(api, session_id, TRUE, None, state.inspector_enabled);
    }

    let provider = state
        .ai_provider
        .get_or_insert_with(browser_local_model_provider)
        .clone();
    let staged = with_session(session_id, |session| {
        let input = session.engine.context().composition.input.clone();
        let result = if AiPrivacyPolicy.allows_provider(session.engine.context(), provider.kind()) {
            provider.provide(session.engine.context(), AI_BUDGET)
        } else {
            AiResult::off(input, AiOffReason::Privacy)
        };
        session.engine.stage_ai_result(result);
        true
    });
    if staged != TRUE {
        return response(FALSE, vec![], None, None, Some("session unavailable"));
    }

    response_from_session(api, session_id, TRUE, None, state.inspector_enabled)
}

/// # Safety
/// `state` must be null or an unfreed pointer returned by `yune_web_init`.
#[no_mangle]
pub unsafe extern "C" fn yune_web_cleanup(state: *mut YuneWebState) {
    if state.is_null() {
        return;
    }
    let mut state = unsafe { Box::from_raw(state) };
    if let Some(api) = api_table() {
        if state.initialized {
            destroy_session(api, state.session_id);
            finalize_api(api);
            state.initialized = false;
            state.ai_enabled = false;
            state.ai_provider = None;
            state.inspector_enabled = false;
        }
    }
}

/// # Safety
/// `response` must be null or a live pointer returned by a YuneWeb operation.
#[no_mangle]
pub unsafe extern "C" fn yune_web_response_json(response: *const YuneWebResponse) -> *const c_char {
    if response.is_null() {
        return ptr::null();
    }
    unsafe { (*response).json.as_ptr() }
}

/// # Safety
/// `response` must be null or a live pointer returned by a YuneWeb operation.
#[no_mangle]
pub unsafe extern "C" fn yune_web_response_handled(response: *const YuneWebResponse) -> Bool {
    if response.is_null() {
        return FALSE;
    }
    unsafe { (*response).handled }
}

/// # Safety
/// `response` must be null or an unfreed pointer returned by a YuneWeb operation.
#[no_mangle]
pub unsafe extern "C" fn yune_web_free_response(response: *mut YuneWebResponse) {
    if !response.is_null() {
        drop(unsafe { Box::from_raw(response) });
    }
}

fn operate(
    state: *mut YuneWebState,
    operation: impl FnOnce(&crate::RimeApi, &YuneWebState) -> *mut YuneWebResponse,
) -> *mut YuneWebResponse {
    if state.is_null() {
        return ptr::null_mut();
    }
    let Some(api) = api_table() else {
        return response(FALSE, vec![], None, None, Some("RimeApi unavailable"));
    };
    let state = unsafe { &*state };
    if !state.initialized || state.session_id == 0 {
        return response(
            FALSE,
            vec![],
            None,
            None,
            Some("YuneWeb state is not initialized"),
        );
    }
    operation(api, state)
}

fn response_from_session(
    api: &crate::RimeApi,
    session_id: RimeSessionId,
    handled: Bool,
    error: Option<&str>,
    inspector_enabled: bool,
) -> *mut YuneWebResponse {
    let commits = capture_commits(api, session_id).unwrap_or_default();
    let mut context = capture_context(api, session_id).ok();
    if let Some(context) = context.as_mut() {
        attach_candidate_sources(session_id, context, inspector_enabled);
        if inspector_enabled {
            attach_inspector_debug(session_id, context);
        }
    }
    let status = capture_status(api, session_id).ok();
    response(handled, commits, context, status, error)
}

fn response(
    handled: Bool,
    commits: Vec<String>,
    context: Option<JsonValue>,
    status: Option<JsonValue>,
    error: Option<&str>,
) -> *mut YuneWebResponse {
    let mut payload = json!({
        "handled": handled == TRUE,
        "commits": commits,
        "context": context,
        "status": status,
    });
    if let Some(error) = error {
        payload["error"] = json!(error);
    }
    let json = CString::new(payload.to_string()).unwrap_or_else(|_| {
        CString::new("{\"handled\":false,\"commits\":[],\"context\":null,\"status\":null,\"error\":\"response serialization failed\"}")
            .expect("fallback JSON is valid CString")
    });
    Box::into_raw(Box::new(YuneWebResponse { handled, json }))
}

fn capture_commits(api: &crate::RimeApi, session_id: RimeSessionId) -> Result<Vec<String>, ()> {
    let get_commit = api.get_commit.ok_or(())?;
    let free_commit = api.free_commit.ok_or(())?;
    let mut commits = Vec::new();
    loop {
        let mut commit = empty_commit();
        if unsafe { get_commit(session_id, &mut commit) } != TRUE {
            break;
        }
        commits.push(c_string_from_mut(commit.text));
        if unsafe { free_commit(&mut commit) } != TRUE {
            return Err(());
        }
    }
    Ok(commits)
}

fn capture_context(api: &crate::RimeApi, session_id: RimeSessionId) -> Result<JsonValue, ()> {
    let get_input = api.get_input.ok_or(())?;
    let get_context = api.get_context.ok_or(())?;
    let free_context = api.free_context.ok_or(())?;
    let input = c_string_from_const(get_input(session_id));
    let mut context = empty_context();
    if unsafe { get_context(session_id, &mut context) } != TRUE {
        return Err(());
    }
    let captured = copy_context(&context, input);
    if unsafe { free_context(&mut context) } != TRUE {
        return Err(());
    }
    Ok(captured)
}

fn copy_context(context: &RimeContext, input: String) -> JsonValue {
    let candidates = copy_candidates(context);
    json!({
        "input": input,
        "preedit": c_string_from_mut(context.composition.preedit),
        "caret": context.composition.cursor_pos,
        "highlighted": context.menu.highlighted_candidate_index,
        "page_size": context.menu.page_size,
        "page_no": context.menu.page_no,
        "is_last_page": context.menu.is_last_page == TRUE,
        "select_keys": if context.menu.select_keys.is_null() {
            serde_json::Value::Null
        } else {
            json!(c_string_from_mut(context.menu.select_keys))
        },
        "select_labels": copy_select_labels(context),
        "candidates": candidates,
    })
}

fn copy_candidates(context: &RimeContext) -> Vec<JsonValue> {
    if context.menu.candidates.is_null() || context.menu.num_candidates <= 0 {
        return Vec::new();
    }
    let candidate_count = usize::try_from(context.menu.num_candidates).unwrap_or(0);
    let candidates =
        unsafe { std::slice::from_raw_parts(context.menu.candidates, candidate_count) };
    candidates.iter().map(copy_candidate).collect()
}

fn copy_candidate(candidate: &RimeCandidate) -> JsonValue {
    json!({
        "text": c_string_from_mut(candidate.text),
        "comment": c_string_from_mut(candidate.comment),
    })
}

fn attach_candidate_sources(
    session_id: RimeSessionId,
    context: &mut JsonValue,
    inspector_enabled: bool,
) {
    let Some(engine_candidates) = session_candidates_snapshot(session_id) else {
        return;
    };
    let page_no = context
        .get("page_no")
        .and_then(serde_json::Value::as_u64)
        .and_then(|page_no| usize::try_from(page_no).ok())
        .unwrap_or(0);
    let page_size = context
        .get("page_size")
        .and_then(serde_json::Value::as_u64)
        .and_then(|page_size| usize::try_from(page_size).ok())
        .unwrap_or(0);
    let Some(page_candidates) = context
        .get_mut("candidates")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let page_start = page_no.saturating_mul(page_size);
    for (page_index, candidate) in page_candidates.iter_mut().enumerate() {
        let full_index = page_start.saturating_add(page_index);
        let Some(engine_candidate) = engine_candidates.get(full_index) else {
            continue;
        };
        if let Some(source) = source_label(&engine_candidate.source, inspector_enabled) {
            candidate["source"] = json!(source);
        }
        if inspector_enabled {
            candidate["quality"] = json!(engine_candidate.quality);
            if let Some(preedit) = &engine_candidate.preedit {
                candidate["preedit"] = json!(preedit);
            }
            if let Some(confidence) = engine_candidate.source.ai_confidence() {
                candidate["ai_confidence"] = json!(confidence.as_score());
            }
        }
    }
}

fn attach_inspector_debug(session_id: RimeSessionId, context: &mut JsonValue) {
    let Some((snapshot, candidates)) = session_inspector_snapshot(session_id) else {
        return;
    };
    let storage_debug = session_web_diagnostics_snapshot(session_id).map_or_else(
        || json!({}),
        |diagnostics| {
            json!({
                "source_fallback": !diagnostics.source_fallbacks.is_empty(),
                "source_fallbacks": diagnostics.source_fallbacks.into_iter().map(|deferral| json!({
                    "gear": deferral.gear,
                    "observed_librime_role": deferral.observed_librime_role,
                    "current_yune_behavior": deferral.current_yune_behavior,
                    "scope_decision": deferral.scope_decision,
                    "target_phase": deferral.target_phase,
                })).collect::<Vec<_>>(),
                "selected": diagnostics.storage.into_iter().map(|row| json!({
                    "translator_index": row.translator_index,
                    "translator": row.translator,
                    "owner": row.owner,
                    "selected_storage": row.selected_storage,
                    "mapping_mode": row.mapping_mode,
                    "is_marisa_backed": row.is_marisa_backed,
                    "byte_source_len": row.byte_source_len,
                    "stored_entry_count": row.stored_entry_count,
                })).collect::<Vec<_>>(),
                "memory_owner_rows": diagnostics.memory_owner_rows.into_iter().map(|row| json!({
                    "owner": row.owner,
                    "class": row.class.as_str(),
                    "estimated_bytes": row.estimated_bytes,
                    "item_count": row.item_count,
                    "storage": row.storage,
                    "notes": row.notes,
                })).collect::<Vec<_>>(),
            })
        },
    );
    let threshold = snapshot.prediction_weight_threshold;
    let prediction_candidates = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            json!({
                "index": index,
                "text": candidate.text.as_str(),
                "source": inspector_source_label(&candidate.source),
                "quality": candidate.quality,
                "threshold": threshold,
                "above_threshold": threshold.map(|threshold| candidate.quality >= threshold),
            })
        })
        .collect::<Vec<_>>();
    context["debug"] = json!({
        "segment_tags": snapshot.segment_tags,
        "segments": snapshot.segments.into_iter().map(|segment| json!({
            "start": segment.start,
            "end": segment.end,
            "tag": segment.tag,
            "source": segment.source,
        })).collect::<Vec<_>>(),
        "filter_pipeline": snapshot.filter_pipeline,
        "filter_audit": snapshot.filter_audit.into_iter().map(|record| json!({
            "name": record.name,
            "before_count": record.before_count,
            "after_count": record.after_count,
        })).collect::<Vec<_>>(),
        "spelling_algebra": snapshot.spelling_algebra.into_iter().map(|algebra| json!({
            "translator": algebra.translator,
            "input": algebra.input,
            "lookup_code": algebra.lookup_code,
            "formulas": algebra.formulas,
            "expanded_codes": algebra.expanded_codes,
        })).collect::<Vec<_>>(),
        "prediction": {
            "weight_threshold": threshold,
            "candidates": prediction_candidates,
        },
        "storage": storage_debug,
        "ai_staging": {
            "state": snapshot.ai_staging.state,
            "for_input": snapshot.ai_staging.for_input,
        },
    });
}

fn source_label(source: &CandidateSource, inspector_enabled: bool) -> Option<&'static str> {
    match source {
        CandidateSource::Ai { provider, .. } if provider == LOCAL_MODEL_PROVIDER_NAME => {
            Some(LOCAL_AI_SOURCE_LABEL)
        }
        CandidateSource::Ai { .. } => Some("ai"),
        _ if inspector_enabled => Some(source.as_str()),
        _ => None,
    }
}

fn inspector_source_label(source: &CandidateSource) -> &'static str {
    source_label(source, true).unwrap_or_else(|| source.as_str())
}

fn copy_select_labels(context: &RimeContext) -> Vec<String> {
    if context.select_labels.is_null() || context.menu.page_size <= 0 {
        return Vec::new();
    }
    let label_count = usize::try_from(context.menu.page_size).unwrap_or(0);
    let labels = unsafe { std::slice::from_raw_parts(context.select_labels, label_count) };
    labels
        .iter()
        .map(|label| c_string_from_mut(*label))
        .collect()
}

fn capture_status(api: &crate::RimeApi, session_id: RimeSessionId) -> Result<JsonValue, ()> {
    let get_status = api.get_status.ok_or(())?;
    let free_status = api.free_status.ok_or(())?;
    let mut status = empty_status();
    if unsafe { get_status(session_id, &mut status) } != TRUE {
        return Err(());
    }
    let captured = json!({
        "schema_id": c_string_from_mut(status.schema_id),
        "schema_name": c_string_from_mut(status.schema_name),
        "is_disabled": status.is_disabled == TRUE,
        "is_composing": status.is_composing == TRUE,
        "is_ascii_mode": status.is_ascii_mode == TRUE,
        "is_full_shape": status.is_full_shape == TRUE,
        "is_simplified": status.is_simplified == TRUE,
        "is_traditional": status.is_traditional == TRUE,
        "is_ascii_punct": status.is_ascii_punct == TRUE,
    });
    if unsafe { free_status(&mut status) } != TRUE {
        return Err(());
    }
    Ok(captured)
}

fn browser_local_model_provider() -> LocalModelProvider {
    LocalModelProvider::new([LocalModelRule::new(
        "nei",
        "\u{4f60}\u{554a}",
        AiConfidence::from_basis_points(8_300),
    )])
}

fn api_table() -> Option<&'static crate::RimeApi> {
    let api = rime_get_api();
    if api.is_null() {
        return None;
    }
    Some(unsafe { &*api })
}

fn has_preloaded_runtime_assets(
    shared_data_dir: &CString,
    user_data_dir: &CString,
    schema_id: &CString,
) -> bool {
    let Ok(shared_data_dir) = shared_data_dir.to_str() else {
        return false;
    };
    let Ok(user_data_dir) = user_data_dir.to_str() else {
        return false;
    };
    let Ok(schema_id) = schema_id.to_str() else {
        return false;
    };
    if !is_valid_schema_id(schema_id) {
        return false;
    }

    let shared_data_dir = std::path::Path::new(shared_data_dir);
    let user_data_dir = std::path::Path::new(user_data_dir);
    let build_dir = user_data_dir.join("build");
    let schema_file = format!("{schema_id}.schema.yaml");
    let shared_schema = shared_data_dir.join(&schema_file);
    let deployed_schema = build_dir.join(&schema_file);

    shared_data_dir.join("default.yaml").is_file()
        && shared_schema.is_file()
        && build_dir.join("default.yaml").is_file()
        && deployed_schema.is_file()
        && has_preloaded_dictionary(shared_data_dir, &shared_schema, &deployed_schema)
}

fn is_valid_schema_id(schema_id: &str) -> bool {
    !schema_id.is_empty()
        && schema_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn has_preloaded_dictionary(
    shared_data_dir: &std::path::Path,
    schema_file: &std::path::Path,
    deployed_schema_file: &std::path::Path,
) -> bool {
    let Some(dictionary) =
        required_dictionary(schema_file).or_else(|| required_dictionary(deployed_schema_file))
    else {
        return false;
    };
    shared_data_dir
        .join(format!("{dictionary}.dict.yaml"))
        .is_file()
}

fn required_dictionary(schema_file: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(schema_file).ok()?;
    let yaml: Value = serde_yaml::from_str(&text).ok()?;
    let schema = yaml.as_mapping()?;
    let translator = schema
        .get(Value::String("translator".to_owned()))?
        .as_mapping()?;
    translator
        .get(Value::String("dictionary".to_owned()))?
        .as_str()
        .filter(|dictionary| is_valid_schema_id(dictionary))
        .map(str::to_owned)
}

fn destroy_session(api: &crate::RimeApi, session_id: RimeSessionId) {
    if let Some(destroy_session) = api.destroy_session {
        destroy_session(session_id);
    }
}

fn finalize_api(api: &crate::RimeApi) {
    if let Some(cleanup_all_sessions) = api.cleanup_all_sessions {
        cleanup_all_sessions();
    }
    if let Some(finalize) = api.finalize {
        finalize();
    }
}

fn cstring_from_ptr(ptr: *const c_char) -> Option<CString> {
    if ptr.is_null() {
        return None;
    }
    let text = unsafe { CStr::from_ptr(ptr) };
    CString::new(text.to_bytes()).ok()
}

fn c_string_from_const(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn c_string_from_mut(ptr: *mut c_char) -> String {
    c_string_from_const(ptr.cast_const())
}

fn dictionary_exclude_config_value(value: &str) -> Option<Value> {
    let parsed: JsonValue = serde_json::from_str(value).ok()?;
    let items = parsed.as_array()?;
    items
        .iter()
        .map(|item| item.as_str().map(|text| Value::String(text.to_owned())))
        .collect::<Option<Vec<_>>>()
        .map(Value::Sequence)
}

fn bool_to_rime(value: bool) -> Bool {
    if value {
        TRUE
    } else {
        FALSE
    }
}

fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: mem::size_of::<RimeTraits>() as c_int,
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

fn empty_commit() -> RimeCommit {
    RimeCommit {
        data_size: (mem::size_of::<RimeCommit>() - mem::size_of::<c_int>()) as c_int,
        text: ptr::null_mut(),
    }
}

fn empty_context() -> RimeContext {
    RimeContext {
        data_size: (mem::size_of::<RimeContext>() - mem::size_of::<c_int>()) as c_int,
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
        data_size: (mem::size_of::<RimeStatus>() - mem::size_of::<c_int>()) as c_int,
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
