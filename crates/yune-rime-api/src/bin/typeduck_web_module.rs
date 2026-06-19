use std::os::raw::{c_char, c_int};

use yune_rime_api::{Bool, YuneTypeDuckResponse, YuneTypeDuckState};

fn main() {
    keep_typeduck_exports_linked();
}

fn keep_typeduck_exports_linked() {
    let _ = yune_rime_api::yune_typeduck_init
        as unsafe extern "C" fn(
            *const c_char,
            *const c_char,
            *const c_char,
        ) -> *mut YuneTypeDuckState;
    let _ = yune_rime_api::yune_typeduck_process_key
        as unsafe extern "C" fn(*mut YuneTypeDuckState, c_int, c_int) -> *mut YuneTypeDuckResponse;
    let _ = yune_rime_api::yune_typeduck_select_candidate
        as unsafe extern "C" fn(*mut YuneTypeDuckState, usize) -> *mut YuneTypeDuckResponse;
    let _ = yune_rime_api::yune_typeduck_delete_candidate
        as unsafe extern "C" fn(*mut YuneTypeDuckState, usize) -> *mut YuneTypeDuckResponse;
    let _ = yune_rime_api::yune_typeduck_flip_page
        as unsafe extern "C" fn(*mut YuneTypeDuckState, Bool) -> *mut YuneTypeDuckResponse;
    let _ =
        yune_rime_api::yune_typeduck_deploy as unsafe extern "C" fn(*mut YuneTypeDuckState) -> Bool;
    let _ = yune_rime_api::yune_typeduck_customize
        as unsafe extern "C" fn(
            *mut YuneTypeDuckState,
            *const c_char,
            *const c_char,
            *const c_char,
        ) -> Bool;
    let _ = yune_rime_api::yune_typeduck_set_option
        as unsafe extern "C" fn(*mut YuneTypeDuckState, *const c_char, Bool) -> Bool;
    let _ = yune_rime_api::yune_typeduck_set_ai_enabled
        as unsafe extern "C" fn(*mut YuneTypeDuckState, Bool) -> Bool;
    let _ = yune_rime_api::yune_typeduck_stage_ai
        as unsafe extern "C" fn(*mut YuneTypeDuckState) -> *mut YuneTypeDuckResponse;
    let _ = yune_rime_api::yune_typeduck_cleanup as unsafe extern "C" fn(*mut YuneTypeDuckState);
    let _ = yune_rime_api::yune_typeduck_response_json
        as unsafe extern "C" fn(*const YuneTypeDuckResponse) -> *const c_char;
    let _ = yune_rime_api::yune_typeduck_response_handled
        as unsafe extern "C" fn(*const YuneTypeDuckResponse) -> Bool;
    let _ = yune_rime_api::yune_typeduck_free_response
        as unsafe extern "C" fn(*mut YuneTypeDuckResponse);
}
