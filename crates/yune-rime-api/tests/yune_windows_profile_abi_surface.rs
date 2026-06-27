use std::{
    ffi::{CStr, CString},
    mem,
    os::raw::c_int,
};

use yune_rime_api::{
    rime_get_api, rime_get_yune_windows_profile_api, RimeApi, RimeConfig,
    RimeYuneWindowsProfileApi, FALSE, TRUE,
};

#[test]
fn default_rime_api_stays_upstream_sized_while_yune_windows_profile_is_larger() {
    let default_api = unsafe { &*rime_get_api() };
    assert_eq!(
        default_api.data_size,
        (mem::size_of::<RimeApi>() - mem::size_of::<c_int>()) as c_int,
        "default rime_get_api must remain the upstream 1.17.0 table"
    );

    let profile_api = unsafe { &*rime_get_yune_windows_profile_api() };
    assert_eq!(
        profile_api.upstream.data_size,
        (mem::size_of::<RimeYuneWindowsProfileApi>() - mem::size_of::<c_int>()) as c_int,
        "Yune Windows accessor advertises the extended profile table"
    );
    assert!(
        profile_api.upstream.data_size > default_api.data_size,
        "Yune Windows profile table must be opt-in and larger than the default upstream table"
    );

    let base = profile_api as *const RimeYuneWindowsProfileApi as usize;
    let append_bool = std::ptr::addr_of!(profile_api.config_list_append_bool) as usize - base;
    let append_int = std::ptr::addr_of!(profile_api.config_list_append_int) as usize - base;
    let append_double = std::ptr::addr_of!(profile_api.config_list_append_double) as usize - base;
    let append_string = std::ptr::addr_of!(profile_api.config_list_append_string) as usize - base;
    let slot_size = mem::size_of::<Option<extern "C" fn()>>();

    assert_eq!(append_bool, mem::size_of::<RimeApi>());
    assert_eq!(append_int, append_bool + slot_size);
    assert_eq!(append_double, append_int + slot_size);
    assert_eq!(append_string, append_double + slot_size);
}

#[test]
fn yune_windows_profile_append_string_slot_creates_and_extends_lists() {
    let profile_api = unsafe { &*rime_get_yune_windows_profile_api() };
    let config_init = profile_api
        .upstream
        .config_init
        .expect("profile table should keep upstream config_init");
    let config_close = profile_api
        .upstream
        .config_close
        .expect("profile table should keep upstream config_close");
    let config_list_size = profile_api
        .upstream
        .config_list_size
        .expect("profile table should keep upstream config_list_size");
    let config_get_string = profile_api
        .upstream
        .config_get_string
        .expect("profile table should keep upstream config_get_string");
    let append_string = profile_api
        .config_list_append_string
        .expect("profile table should expose config_list_append_string");

    let mut config = RimeConfig {
        ptr: std::ptr::null_mut(),
    };
    let languages = CString::new("display_languages").expect("key should be valid");
    let first_language = CString::new("display_languages/@0").expect("key should be valid");
    let second_language = CString::new("display_languages/@1").expect("key should be valid");
    let english = CString::new("en_US").expect("value should be valid");
    let cantonese = CString::new("zh_HK").expect("value should be valid");

    assert_eq!(unsafe { config_init(&mut config) }, TRUE);
    assert_eq!(
        unsafe { append_string(&mut config, languages.as_ptr(), english.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { append_string(&mut config, languages.as_ptr(), cantonese.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { config_list_size(&mut config, languages.as_ptr()) },
        2
    );
    assert_eq!(
        config_string(
            &mut config,
            first_language.as_c_str().to_str().unwrap(),
            config_get_string
        )
        .as_deref(),
        Some("en_US")
    );
    assert_eq!(
        config_string(
            &mut config,
            second_language.as_c_str().to_str().unwrap(),
            config_get_string,
        )
        .as_deref(),
        Some("zh_HK")
    );
    assert_eq!(unsafe { config_close(&mut config) }, TRUE);
}

fn config_string(
    config: &mut RimeConfig,
    key: &str,
    config_get_string: unsafe extern "C" fn(
        *mut RimeConfig,
        *const std::os::raw::c_char,
        *mut std::os::raw::c_char,
        usize,
    ) -> c_int,
) -> Option<String> {
    let key = CString::new(key).expect("key should be valid");
    let mut buffer = vec![0; 128];
    let ok = unsafe { config_get_string(config, key.as_ptr(), buffer.as_mut_ptr(), buffer.len()) };
    if ok == FALSE {
        return None;
    }
    Some(
        unsafe { CStr::from_ptr(buffer.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}
