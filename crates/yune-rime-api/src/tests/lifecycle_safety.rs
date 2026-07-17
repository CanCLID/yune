use std::{ffi::CString, fs, ptr};

use super::*;

#[test]
fn lifecycle_safety_repeated_setup_initialize_finalize_is_deterministic() {
    let _guard = test_guard();

    for iteration in 0..3 {
        let root = unique_temp_dir(&format!("lifecycle-repeated-{iteration}"));
        let shared = root.join("shared");
        let user = root.join("user");
        fs::create_dir_all(&shared).expect("shared dir should be created");
        fs::create_dir_all(&user).expect("user dir should be created");

        let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
        let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
        let mut traits = empty_traits();
        traits.shared_data_dir = shared_c.as_ptr();
        traits.user_data_dir = user_c.as_ptr();

        unsafe { RimeSetup(&traits) };
        unsafe { RimeInitialize(&traits) };
        let session_id = RimeCreateSession();
        assert_ne!(
            session_id, 0,
            "iteration {iteration} creates a usable session"
        );
        assert_eq!(RimeFindSession(session_id), TRUE);
        assert_eq!(RimeDestroySession(session_id), TRUE);
        assert_eq!(RimeFindSession(session_id), FALSE);

        RimeFinalize();
        assert_eq!(
            RimeCreateSession(),
            0,
            "finalize stops new session creation"
        );
        assert_eq!(RimeFindSession(session_id), FALSE);
        fs::remove_dir_all(root).expect("temp dirs should be removed");
    }

    let reset_traits = empty_traits();
    unsafe { RimeInitialize(&reset_traits) };
}

#[test]
fn lifecycle_safety_repeated_session_destroy_and_cleanup_reject_stale_handles() {
    let _guard = test_guard();

    for iteration in 0..3 {
        RimeCleanupAllSessions();
        let first = RimeCreateSession();
        assert_ne!(first, 0, "iteration {iteration} creates first session");
        assert_eq!(RimeFindSession(first), TRUE);
        assert_eq!(RimeDestroySession(first), TRUE);
        assert_eq!(RimeDestroySession(first), FALSE);
        assert_eq!(RimeFindSession(first), FALSE);

        let second = RimeCreateSession();
        assert_ne!(second, 0, "iteration {iteration} creates second session");
        assert_eq!(RimeFindSession(second), TRUE);
        RimeCleanupAllSessions();
        assert_eq!(RimeFindSession(second), FALSE);

        let after_cleanup = RimeCreateSession();
        assert_ne!(
            after_cleanup, 0,
            "iteration {iteration} can create after cleanup-all"
        );
        assert_ne!(after_cleanup, first, "stale handles are not reused");
        assert_ne!(after_cleanup, second, "cleanup handles are not reused");
        assert_eq!(RimeFindSession(after_cleanup), TRUE);
        assert_eq!(RimeDestroySession(after_cleanup), TRUE);
    }
}

#[test]
fn dictionary_translator_cache_survives_finalize_but_releases_on_cleanup_and_root_change() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    crate::schema_install::clear_dictionary_translator_cache();

    let root = unique_temp_dir("translator-cache-lifetime");
    let other_root = unique_temp_dir("translator-cache-lifetime-other");
    for path in [&root, &other_root] {
        fs::create_dir_all(path.join("shared")).expect("shared dir should be created");
        fs::create_dir_all(path.join("user")).expect("user dir should be created");
    }
    let shared =
        CString::new(root.join("shared").to_string_lossy().as_ref()).expect("path is valid");
    let user = CString::new(root.join("user").to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user.as_ptr();
    unsafe { RimeSetup(&traits) };

    const PROBE: &str = "m59-cache-lifetime-probe";
    crate::schema_install::seed_dictionary_translator_cache_probe(PROBE);
    RimeFinalize();
    assert!(
        crate::schema_install::dictionary_translator_cache_contains_probe(PROBE),
        "D-32 keeps fingerprinted immutable translators warm across finalize"
    );

    unsafe { RimeSetup(&traits) };
    RimeCleanupAllSessions();
    assert!(
        !crate::schema_install::dictionary_translator_cache_contains_probe(PROBE),
        "explicit cleanup must release mapped artifacts for same-path replacement"
    );
    crate::schema_install::seed_dictionary_translator_cache_probe(PROBE);

    let other_shared =
        CString::new(other_root.join("shared").to_string_lossy().as_ref()).expect("path is valid");
    let other_user =
        CString::new(other_root.join("user").to_string_lossy().as_ref()).expect("path is valid");
    let mut other_traits = empty_traits();
    other_traits.shared_data_dir = other_shared.as_ptr();
    other_traits.user_data_dir = other_user.as_ptr();
    unsafe { RimeSetup(&other_traits) };
    assert!(
        !crate::schema_install::dictionary_translator_cache_contains_probe(PROBE),
        "changing dictionary roots must release mmap-backed cache owners"
    );

    crate::schema_install::clear_dictionary_translator_cache();
    fs::remove_dir_all(root).expect("temp root should be removed");
    fs::remove_dir_all(other_root).expect("other temp root should be removed");
}

#[test]
fn lifecycle_safety_schema_switching_and_deployment_emit_ordered_notifications() {
    let _guard = test_guard();
    RimeCleanupAllSessions();

    let root = unique_temp_dir("lifecycle-notification-order");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: sample_schema\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("sample_schema.schema.yaml"),
        "schema:\n  schema_id: sample_schema\n  name: Sample\n  version: test\n",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    unsafe { RimeSetup(&traits) };

    notification_events_lock().clear();
    RimeSetNotificationHandler(Some(record_notification), 0x7b_usize as *mut c_void);

    let session_id = RimeCreateSession();
    let ascii_mode = CString::new("ascii_mode").expect("option name is valid");
    let property = CString::new("client_app").expect("property name is valid");
    let property_value = CString::new("frontend_client").expect("property value is valid");
    let schema_id = CString::new("sample_schema").expect("schema id is valid");
    unsafe {
        RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE);
        RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE);
        RimeSetProperty(session_id, property.as_ptr(), property_value.as_ptr());
        assert_eq!(RimeSelectSchema(session_id, schema_id.as_ptr()), TRUE);
    }
    assert_eq!(RimeStartMaintenance(TRUE), TRUE);
    assert_eq!(RimeDeployWorkspace(), TRUE);
    assert_eq!(
        RimeFindSession(session_id),
        FALSE,
        "workspace artifact mutation closes sessions before releasing shared translator owners"
    );
    assert_eq!(
        RimeDeploySchema(c"sample_schema.schema.yaml".as_ptr()),
        TRUE
    );

    let events = notification_events_lock();
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "option".to_owned(),
                message_value: "ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "option".to_owned(),
                message_value: "!ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "property".to_owned(),
                message_value: "client_app=frontend_client".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id,
                message_type: "schema".to_owned(),
                message_value: "sample_schema/sample_schema".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "start".to_owned(),
            },
            NotificationEvent {
                context_object: 0x7b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "success".to_owned(),
            },
        ]
    );
    drop(events);

    RimeSetNotificationHandler(None, ptr::null_mut());
    assert_eq!(
        RimeDestroySession(session_id),
        FALSE,
        "the deployment boundary already invalidated this stale handle"
    );
    let reset_traits = empty_traits();
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn lifecycle_safety_notification_handler_replacement_and_clearing_are_deterministic() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    notification_events_lock().clear();

    let session_id = RimeCreateSession();
    let ascii_mode = CString::new("ascii_mode").expect("option name is valid");

    RimeSetNotificationHandler(Some(record_notification), 0x11_usize as *mut c_void);
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };
    RimeSetNotificationHandler(Some(record_notification), 0x22_usize as *mut c_void);
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), FALSE) };
    RimeSetNotificationHandler(None, ptr::null_mut());
    unsafe { RimeSetOption(session_id, ascii_mode.as_ptr(), TRUE) };

    let events = notification_events_lock();
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x11,
                session_id,
                message_type: "option".to_owned(),
                message_value: "ascii_mode".to_owned(),
            },
            NotificationEvent {
                context_object: 0x22,
                session_id,
                message_type: "option".to_owned(),
                message_value: "!ascii_mode".to_owned(),
            },
        ]
    );
    drop(events);
    assert_eq!(RimeDestroySession(session_id), TRUE);
}

#[test]
fn lifecycle_safety_records_process_global_service_and_narrow_threading_contract() {
    let _guard = test_guard();
    let conventions = include_str!("../../../../docs/conventions.md");
    let support_contract = include_str!("../../../../docs/contracts/engine-support-contract.md");
    assert!(conventions.contains("Process-global service"));
    assert!(conventions.contains("one active RIME service per WASM instance"));
    assert!(support_contract.contains("cross-thread calls using valid session"));
    assert!(support_contract.contains("does not promise parallel progress"));
}
