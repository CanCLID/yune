use std::{ffi::CStr, fs, os::raw::c_char};

use serde_yaml::Value;

use crate::{
    apply_schema_switch_resets, config_scalar_bool, copy_c_string_with_strncpy_semantics,
    deployed_schema_name, find_config_value, install_schema_ascii_composer_processor,
    install_schema_chord_composer_processor, install_schema_editor_processor,
    install_schema_filter_chain, install_schema_key_binder_processor,
    install_schema_navigator_bindings, install_schema_punctuation_processor,
    install_schema_recognizer_processor, install_schema_segment_tags,
    install_schema_selector_bindings, install_schema_speller_processor,
    install_schema_translator_chain, load_runtime_config_root, notify, schema_string_list,
    selected_runtime_config_path, sessions, with_session, Bool, ConfigOpenKind, NavigatorBindings,
    NavigatorSyllableJumpPosition, RimeSessionId, SelectorBindings, SessionState, FALSE, TRUE,
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
    if schema_id.is_null() {
        return FALSE;
    }
    // SAFETY: callers promise that `schema_id` is a valid nul-terminated
    // string.
    let schema_id = unsafe { CStr::from_ptr(schema_id) }
        .to_string_lossy()
        .into_owned();

    let selected = with_session(session_id, |session| {
        apply_schema_to_session(session, &schema_id);
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

pub(crate) fn apply_schema_to_session(session: &mut SessionState, schema_id: &str) {
    let schema_name = deployed_schema_name(schema_id);
    session.engine.set_schema(schema_id.to_owned(), schema_name);
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
    restore_switcher_saved_options(session, schema_id);
    apply_schema_switch_resets(session, schema_id);
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
    install_schema_translator_chain(session, schema_id);
    install_schema_filter_chain(session, schema_id);
    session.reload_userdb_from_store();
    session.engine.clear_composition();
    session.input_buffer = None;
    session.unread_commit = None;
}

fn restore_switcher_saved_options(session: &mut SessionState, schema_id: &str) {
    let schema_config =
        load_runtime_config_root(&format!("{schema_id}.schema"), ConfigOpenKind::Deployed);
    let save_options = schema_string_list(&schema_config, "switcher/save_options");
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
