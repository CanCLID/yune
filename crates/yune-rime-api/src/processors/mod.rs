mod ascii_composer;
mod editor;
mod key_binder;
mod recognizer;
mod speller;

pub(crate) use ascii_composer::{
    install_schema_ascii_composer_processor, is_ascii_composer_modifier_key,
    process_ascii_composer_caps_lock_switch_key, process_ascii_composer_modifier_switch_key,
    process_ascii_composer_processor, process_ascii_composer_switch_key,
};
pub(crate) use editor::{install_schema_editor_processor, process_editor_processor};
pub(crate) use key_binder::{
    install_schema_key_binder_processor, process_key_binder_processor,
    select_key_binding_radio_option, update_key_binding_paging_state, KeyBinderProcessor,
};
pub(crate) use recognizer::{install_schema_recognizer_processor, process_recognizer_processor};
pub(crate) use speller::{install_schema_speller_processor, process_speller_processor};
