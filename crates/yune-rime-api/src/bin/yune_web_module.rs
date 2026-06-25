use std::os::raw::{c_char, c_int};

use yune_rime_api::{Bool, YuneWebResponse, YuneWebState};

fn main() {
    keep_yune_web_exports_linked();
}

fn keep_yune_web_exports_linked() {
    let _ = yune_rime_api::yune_web_init
        as unsafe extern "C" fn(*const c_char, *const c_char, *const c_char) -> *mut YuneWebState;
    let _ = yune_rime_api::yune_web_process_key
        as unsafe extern "C" fn(*mut YuneWebState, c_int, c_int) -> *mut YuneWebResponse;
    let _ = yune_rime_api::yune_web_select_candidate
        as unsafe extern "C" fn(*mut YuneWebState, usize) -> *mut YuneWebResponse;
    let _ = yune_rime_api::yune_web_delete_candidate
        as unsafe extern "C" fn(*mut YuneWebState, usize) -> *mut YuneWebResponse;
    let _ = yune_rime_api::yune_web_flip_page
        as unsafe extern "C" fn(*mut YuneWebState, Bool) -> *mut YuneWebResponse;
    let _ = yune_rime_api::yune_web_deploy as unsafe extern "C" fn(*mut YuneWebState) -> Bool;
    let _ = yune_rime_api::yune_web_customize
        as unsafe extern "C" fn(
            *mut YuneWebState,
            *const c_char,
            *const c_char,
            *const c_char,
        ) -> Bool;
    let _ = yune_rime_api::yune_web_set_option
        as unsafe extern "C" fn(*mut YuneWebState, *const c_char, Bool) -> Bool;
    let _ = yune_rime_api::yune_web_set_ai_enabled
        as unsafe extern "C" fn(*mut YuneWebState, Bool) -> Bool;
    let _ = yune_rime_api::yune_web_stage_ai
        as unsafe extern "C" fn(*mut YuneWebState) -> *mut YuneWebResponse;
    let _ = yune_rime_api::yune_web_cleanup as unsafe extern "C" fn(*mut YuneWebState);
    let _ = yune_rime_api::yune_web_response_json
        as unsafe extern "C" fn(*const YuneWebResponse) -> *const c_char;
    let _ = yune_rime_api::yune_web_response_handled
        as unsafe extern "C" fn(*const YuneWebResponse) -> Bool;
    let _ = yune_rime_api::yune_web_free_response as unsafe extern "C" fn(*mut YuneWebResponse);
}
