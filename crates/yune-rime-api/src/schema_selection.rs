use std::{ffi::CStr, fs, os::raw::c_char, time::UNIX_EPOCH};

use serde_yaml::Value;

use crate::{
    apply_schema_switch_resets_from_config, config_scalar_bool, context_menu_settings_from_config,
    copy_c_string_with_strncpy_semantics, find_config_value,
    install_schema_ascii_composer_processor, install_schema_chord_composer_processor,
    install_schema_editor_processor, install_schema_filter_chain,
    install_schema_key_binder_processor, install_schema_navigator_bindings,
    install_schema_punctuation_processor, install_schema_recognizer_processor,
    install_schema_segment_tags, install_schema_selector_bindings,
    install_schema_speller_processor, install_schema_translator_chain, load_runtime_config_root,
    notify, schema_reload_signature, schema_string_list, selected_runtime_config_path,
    selected_runtime_data_path, sessions, startup_trace, with_session, Bool, ConfigOpenKind,
    NavigatorBindings, NavigatorSyllableJumpPosition, RimeSessionId, SelectorBindings,
    SessionState, FALSE, TRUE,
};

/// Copies the current session schema id into caller-provided storage.
///
/// # Safety
///
/// `schema_id` must point to writable storage of `buffer_size` bytes. Null
/// buffers are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCurrentSchema(
    session_id: RimeSessionId,
    schema_id: *mut c_char,
    buffer_size: usize,
) -> Bool {
    if schema_id.is_null() {
        return FALSE;
    }

    with_session(session_id, |session| {
        let current_schema = session.engine.status().schema_id;
        copy_c_string_with_strncpy_semantics(&current_schema, schema_id, buffer_size);
        true
    })
}

/// Selects the active schema id for a session.
///
/// # Safety
///
/// `schema_id` must be either null or point to a valid nul-terminated C string.
/// Null schema ids are rejected.
#[no_mangle]
pub unsafe extern "C" fn RimeSelectSchema(
    session_id: RimeSessionId,
    schema_id: *const c_char,
) -> Bool {
    let _trace = startup_trace::span("schema_select");
    if schema_id.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `schema_id` is a valid nul-terminated
    // string.
    let schema_id = unsafe { CStr::from_ptr(schema_id) }
        .to_string_lossy()
        .into_owned();

    let selected = with_session(session_id, |session| {
        if let Some(signature) = same_schema_idle_reload_signature(session, &schema_id) {
            if session.schema_reload_signature.as_deref() != Some(signature.as_str()) {
                apply_schema_to_session(session, &schema_id);
            }
        } else {
            apply_schema_to_session(session, &schema_id);
        }
        true
    });
    if selected == TRUE {
        let status = sessions()
            .lock()
            .expect("session registry should not be poisoned")
            .sessions
            .get(&session_id)
            .map(|session| session.engine.status());
        if let Some(status) = status {
            notify(
                session_id,
                "schema",
                &format!("{}/{}", status.schema_id, status.schema_name),
            );
        }
    }
    selected
}

fn same_schema_idle_reload_signature(session: &SessionState, schema_id: &str) -> Option<String> {
    let context = session.engine.context();
    if session.engine.status().schema_id != schema_id
        || !context.composition.input.is_empty()
        || !context.candidates.is_empty()
        || session.input_buffer.is_some()
        || session.unread_commit.is_some()
    {
        return None;
    }
    if schema_id == "luna_pinyin" {
        if let (Some(reload_signature), Some(stored_watch), Some(current_watch)) = (
            session.schema_reload_signature.as_ref(),
            session.schema_reload_watch_signature.as_ref(),
            schema_reload_watch_signature(schema_id, session.user_dict_name.as_deref()),
        ) {
            if stored_watch == &current_watch {
                return Some(reload_signature.clone());
            }
        }
    }
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    Some(schema_reload_signature(&schema_config))
}

pub(crate) fn apply_schema_to_session(session: &mut SessionState, schema_id: &str) {
    let schema_config = {
        let _trace = startup_trace::span("schema_config_load");
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed)
    };
    let schema_name = find_config_value(&schema_config, "schema/name")
        .and_then(Value::as_str)
        .unwrap_or(schema_id)
        .to_owned();
    let menu_settings = context_menu_settings_from_config(&schema_config);
    let reload_signature = schema_reload_signature(&schema_config);
    session.engine.set_schema(schema_id.to_owned(), schema_name);
    session.menu_settings = menu_settings;
    session.schema_reload_signature = Some(reload_signature);
    session.clear_user_dict_name();
    session.engine.clear_translators();
    session.engine.set_prediction_never_first(false);
    session.engine.reset_filters();
    session.key_binder = None;
    session.speller = None;
    session.editor_processor = None;
    session.editor_bindings.clear();
    session.editor_char_handler = None;
    session.chord_composer = None;
    session.engine.set_option("_auto_commit", false);
    session.ascii_composer_enabled = false;
    session.ascii_composer_switch_bindings.clear();
    session.ascii_composer_pressed_switch_key = None;
    session.ascii_composer_inline_ascii = false;
    session.ascii_segmentor_enabled = false;
    session.punct_segmentor = None;
    session.fallback_segmentor_enabled = false;
    session.remaining_gear_deferrals.clear();
    session.punctuation_processor = None;
    session.recognizer_processor = None;
    session.selector_bindings = SelectorBindings::default();
    session.navigator_bindings = NavigatorBindings::default();
    session.navigator_delimiters = " ".to_owned();
    session.navigator_syllable_jump_position = NavigatorSyllableJumpPosition::AfterDelimiter;
    session.paging = false;
    {
        let _trace = startup_trace::span("schema_switch_options");
        restore_switcher_saved_options(session, &schema_config);
        apply_schema_switch_resets_from_config(session, &schema_config);
    }
    {
        let _trace = startup_trace::span("processor_install");
        install_schema_segment_tags(session, schema_id);
        install_schema_editor_processor(session, schema_id);
        install_schema_chord_composer_processor(session, schema_id);
        install_schema_ascii_composer_processor(session, schema_id);
        install_schema_speller_processor(session, schema_id);
        install_schema_recognizer_processor(session, schema_id);
        install_schema_selector_bindings(session, schema_id);
        install_schema_navigator_bindings(session, schema_id);
        install_schema_key_binder_processor(session, schema_id);
        install_schema_punctuation_processor(session, schema_id);
    }
    {
        let _trace = startup_trace::span("translator_install");
        install_schema_translator_chain(session, schema_id);
    }
    {
        let _trace = startup_trace::span("filter_install");
        install_schema_filter_chain(session, schema_id);
    }
    {
        let _trace = startup_trace::span("userdb_open_or_sync");
        session.reload_userdb_from_store();
    }
    session.engine.clear_composition();
    session.input_buffer = None;
    session.unread_commit = None;
    session.schema_reload_watch_signature =
        schema_reload_watch_signature(schema_id, session.user_dict_name.as_deref());
}

fn schema_reload_watch_signature(schema_id: &str, dictionary_id: Option<&str>) -> Option<String> {
    if schema_id != "luna_pinyin" {
        return None;
    }
    let dictionary_id = dictionary_id?;
    let mut parts = Vec::new();
    append_config_watch_signature(&mut parts, &format!("{schema_id}.schema"));
    for resource_id in [
        format!("{dictionary_id}.dict.yaml"),
        format!("{dictionary_id}.table.bin"),
        format!("{dictionary_id}.prism.bin"),
        format!("{dictionary_id}.reverse.bin"),
    ] {
        append_data_watch_signature(&mut parts, &resource_id);
    }
    Some(parts.join("\n"))
}

fn append_config_watch_signature(parts: &mut Vec<String>, config_id: &str) {
    let Some(path) = selected_runtime_config_path(config_id, ConfigOpenKind::Deployed) else {
        parts.push(format!("config:{config_id}:missing"));
        return;
    };
    append_path_watch_signature(parts, "config", &path);
}

fn append_data_watch_signature(parts: &mut Vec<String>, resource_id: &str) {
    let Some(path) = selected_runtime_data_path(resource_id) else {
        parts.push(format!("data:{resource_id}:missing"));
        return;
    };
    append_path_watch_signature(parts, "data", &path);
}

fn append_path_watch_signature(parts: &mut Vec<String>, role: &str, path: &std::path::Path) {
    let Ok(metadata) = fs::metadata(path) else {
        parts.push(format!("{role}:{}:metadata-unavailable", path.display()));
        return;
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_nanos());
    parts.push(format!(
        "{role}:{}:{}:{modified}",
        path.display(),
        metadata.len()
    ));
}

fn restore_switcher_saved_options(session: &mut SessionState, schema_config: &Value) {
    let save_options = schema_string_list(schema_config, "switcher/save_options");
    if save_options.is_empty() {
        return;
    }

    let Some(user_config_path) = selected_runtime_config_path("user", ConfigOpenKind::User) else {
        return;
    };
    let Some(user_config) = fs::read_to_string(user_config_path)
        .ok()
        .and_then(|text| serde_yaml::from_str::<Value>(&text).ok())
    else {
        return;
    };

    for option_name in save_options {
        let Some(value) = find_config_value(&user_config, &format!("var/option/{option_name}"))
            .and_then(config_scalar_bool)
        else {
            continue;
        };
        session.engine.set_option(option_name, value);
    }
}
