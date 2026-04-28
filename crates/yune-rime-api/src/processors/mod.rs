mod ascii_composer;
mod editor;
mod recognizer;

pub(crate) use ascii_composer::{
    install_schema_ascii_composer_processor, is_ascii_composer_modifier_key,
    process_ascii_composer_caps_lock_switch_key, process_ascii_composer_modifier_switch_key,
    process_ascii_composer_processor, process_ascii_composer_switch_key,
};
pub(crate) use editor::{install_schema_editor_processor, process_editor_processor};
pub(crate) use recognizer::{install_schema_recognizer_processor, process_recognizer_processor};
