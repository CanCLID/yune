use super::*;

fn platform_path(base: &str, child: &str) -> String {
    std::path::Path::new(base)
        .join(child)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn setup_and_initialize_expose_runtime_metadata_paths() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let shared = CString::new("/tmp/yune-shared").expect("path should be valid");
    let user = CString::new("/tmp/yune-user").expect("path should be valid");
    let staging = CString::new("/tmp/yune-stage").expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user.as_ptr();
    traits.staging_dir = staging.as_ptr();
    let mut buffer = vec![0 as c_char; 64];
    let mut short_buffer = vec![0 as c_char; 10];

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };

    let version = RimeGetVersion();
    assert!(!version.is_null());
    // SAFETY: version is a static NUL-terminated C string.
    let version = unsafe { CStr::from_ptr(version) };
    assert_eq!(version.to_str(), Ok("yune-rime-api 0.1.0"));

    // SAFETY: runtime path getters return stable process-owned C strings.
    let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
    assert_eq!(shared_dir.to_str(), Ok("/tmp/yune-shared"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_dir = unsafe { CStr::from_ptr(RimeGetUserDataDir()) };
    assert_eq!(user_dir.to_str(), Ok("/tmp/yune-user"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
    let expected_prebuilt = platform_path("/tmp/yune-shared", "build");
    assert_eq!(prebuilt_dir.to_str(), Ok(expected_prebuilt.as_str()));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let staging_dir = unsafe { CStr::from_ptr(RimeGetStagingDir()) };
    assert_eq!(staging_dir.to_str(), Ok("/tmp/yune-stage"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let sync_dir = unsafe { CStr::from_ptr(RimeGetSyncDir()) };
    assert_eq!(sync_dir.to_str(), Ok("sync"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_id = unsafe { CStr::from_ptr(RimeGetUserId()) };
    assert_eq!(user_id.to_str(), Ok("unknown"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetPrebuiltDataDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_prebuilt = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_prebuilt.to_str(), Ok(expected_prebuilt.as_str()));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetSharedDataDirSecure(short_buffer.as_mut_ptr(), short_buffer.len()) };
    // SAFETY: the raw byte view is bounded to the caller-owned buffer.
    let truncated_shared = unsafe {
        std::slice::from_raw_parts(short_buffer.as_ptr().cast::<u8>(), short_buffer.len())
    };
    assert_eq!(truncated_shared, b"/tmp/yune-");

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetUserDataDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_user = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_user.to_str(), Ok("/tmp/yune-user"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetStagingDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_staging = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_staging.to_str(), Ok("/tmp/yune-stage"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetSyncDirSecure(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(copied_sync.to_str(), Ok("sync"));

    // SAFETY: buffers point to writable storage.
    unsafe { RimeGetUserDataSyncDir(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let copied_user_sync = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    let expected_user_sync = platform_path("sync", "unknown");
    assert_eq!(copied_user_sync.to_str(), Ok(expected_user_sync.as_str()));

    let prebuilt = CString::new("/tmp/yune-prebuilt").expect("path should be valid");
    traits.prebuilt_data_dir = prebuilt.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeInitialize(&traits) };
    // SAFETY: runtime path getters return stable process-owned C strings.
    let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
    assert_eq!(prebuilt_dir.to_str(), Ok("/tmp/yune-prebuilt"));

    RimeFinalize();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeInitialize(&traits) };
}

#[test]
fn setup_respects_rime_traits_data_size_for_newer_path_fields() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let shared = CString::new("/tmp/yune-shared-old-traits").expect("path should be valid");
    let user = CString::new("/tmp/yune-user-old-traits").expect("path should be valid");
    let prebuilt = CString::new("/tmp/yune-prebuilt-ignored").expect("path should be valid");
    let staging = CString::new("/tmp/yune-staging-ignored").expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user.as_ptr();
    traits.prebuilt_data_dir = prebuilt.as_ptr();
    traits.staging_dir = staging.as_ptr();
    traits.data_size = traits_data_size_before_prebuilt_data_dir();

    // SAFETY: traits points to valid storage and all strings live for the call.
    unsafe { RimeSetup(&traits) };

    // SAFETY: runtime path getters return stable process-owned C strings.
    let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
    assert_eq!(shared_dir.to_str(), Ok("/tmp/yune-shared-old-traits"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_dir = unsafe { CStr::from_ptr(RimeGetUserDataDir()) };
    assert_eq!(user_dir.to_str(), Ok("/tmp/yune-user-old-traits"));
    // SAFETY: `prebuilt_data_dir` was outside `data_size`, so librime derives
    // it from the provided shared data directory.
    let prebuilt_dir = unsafe { CStr::from_ptr(RimeGetPrebuiltDataDir()) };
    let expected_prebuilt = platform_path("/tmp/yune-shared-old-traits", "build");
    assert_eq!(prebuilt_dir.to_str(), Ok(expected_prebuilt.as_str()));
    // SAFETY: `staging_dir` was outside `data_size`, so librime derives it
    // from the provided user data directory.
    let staging_dir = unsafe { CStr::from_ptr(RimeGetStagingDir()) };
    let expected_staging = platform_path("/tmp/yune-user-old-traits", "build");
    assert_eq!(staging_dir.to_str(), Ok(expected_staging.as_str()));

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
}

#[test]
fn setup_reads_existing_installation_metadata() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("installation-metadata");
    let user = root.join("user");
    let sync = root.join("cloud-sync");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        user.join("installation.yaml"),
        format!(
            "\
installation_id: device-123
sync_dir: {}
backup_config_files: false
",
            sync.to_string_lossy()
        ),
    )
    .expect("installation metadata should be written");
    fs::write(user.join("default.yaml"), "config_version: '1.0'\n")
        .expect("user config should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_id = unsafe { CStr::from_ptr(RimeGetUserId()) };
    assert_eq!(user_id.to_str(), Ok("device-123"));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let sync_dir = unsafe { CStr::from_ptr(RimeGetSyncDir()) };
    assert_eq!(sync_dir.to_str(), Ok(sync.to_string_lossy().as_ref()));

    let mut buffer = vec![0 as c_char; 256];
    // SAFETY: buffer points to writable storage.
    unsafe { RimeGetUserDataSyncDir(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let user_sync_dir = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(
        user_sync_dir.to_str(),
        Ok(sync.join("device-123").to_string_lossy().as_ref())
    );

    let backup_config_task =
        CString::new("backup_config_files").expect("task name should be valid");
    assert_eq!(RimeRunTask(backup_config_task.as_ptr()), TRUE);
    assert!(!sync.join("device-123").join("default.yaml").exists());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn installation_update_creates_metadata_and_refreshes_runtime_paths() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("installation-update");
    let user = root.join("user");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let distribution_name = CString::new("Yune Test").expect("distribution name should be valid");
    let distribution_code = CString::new("yune-test").expect("distribution code should be valid");
    let distribution_version =
        CString::new("2026.04").expect("distribution version should be valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    traits.distribution_name = distribution_name.as_ptr();
    traits.distribution_code_name = distribution_code.as_ptr();
    traits.distribution_version = distribution_version.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let task_name = CString::new("installation_update").expect("task name should be valid");
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);

    let metadata = fs::read_to_string(user.join("installation.yaml"))
        .expect("installation metadata should be written");
    let metadata: Value =
        serde_yaml::from_str(&metadata).expect("installation metadata should parse");
    let installation_id = find_config_value(&metadata, "installation_id")
        .and_then(Value::as_str)
        .expect("installation id should be recorded");
    assert!(installation_id.starts_with("yune-"));
    assert_eq!(
        find_config_value(&metadata, "distribution_name").and_then(Value::as_str),
        Some("Yune Test")
    );
    assert_eq!(
        find_config_value(&metadata, "distribution_code_name").and_then(Value::as_str),
        Some("yune-test")
    );
    assert_eq!(
        find_config_value(&metadata, "distribution_version").and_then(Value::as_str),
        Some("2026.04")
    );
    assert!(find_config_value(&metadata, "rime_version").is_some());

    // SAFETY: runtime path getters return stable process-owned C strings.
    let user_id = unsafe { CStr::from_ptr(RimeGetUserId()) };
    assert_eq!(user_id.to_str(), Ok(installation_id));
    // SAFETY: runtime path getters return stable process-owned C strings.
    let sync_dir = unsafe { CStr::from_ptr(RimeGetSyncDir()) };
    assert_eq!(
        sync_dir.to_str(),
        Ok(user.join("sync").to_string_lossy().as_ref())
    );
    let mut buffer = vec![0 as c_char; 256];
    // SAFETY: buffer points to writable storage.
    unsafe { RimeGetUserDataSyncDir(buffer.as_mut_ptr(), buffer.len()) };
    // SAFETY: secure getter wrote a trailing NUL into the buffer.
    let user_sync_dir = unsafe { CStr::from_ptr(buffer.as_ptr()) };
    assert_eq!(
        user_sync_dir.to_str(),
        Ok(user
            .join("sync")
            .join(installation_id)
            .to_string_lossy()
            .as_ref())
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn maintenance_and_deployment_shims_are_deterministic() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deployer-shims");
    let shared_path = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared_path).expect("shared dir should be created");
    fs::write(
        shared_path.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared_path.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n  version: test\n",
    )
    .expect("shared schema should be written");
    let shared =
        CString::new(shared_path.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let schema_file = CString::new("default.schema.yaml").expect("file should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let task_name = CString::new("workspace_update").expect("task should be valid");
    let unknown_task = CString::new("no_such_task").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };
    RimeSetupLogging(task_name.as_ptr());
    assert_eq!(RimeStartMaintenance(TRUE), TRUE);
    assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), FALSE);
    assert!(user.join("build").join("default.yaml").is_file());
    assert!(user.join("build").join("default.schema.yaml").is_file());
    assert_eq!(RimeIsMaintenancing(), FALSE);
    RimeJoinMaintenanceThread();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    // SAFETY: runtime path getters return stable process-owned C strings.
    let shared_dir = unsafe { CStr::from_ptr(RimeGetSharedDataDir()) };
    assert_eq!(
        shared_dir.to_str(),
        Ok(shared_path.to_string_lossy().as_ref())
    );

    assert_eq!(RimePrebuildAllSchemas(), TRUE);
    assert_eq!(RimeDeployWorkspace(), TRUE);
    assert_eq!(RimeDeploySchema(schema_file.as_ptr()), TRUE);
    assert_eq!(RimeDeploySchema(std::ptr::null()), FALSE);
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), std::ptr::null()),
        FALSE
    );
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);
    assert_eq!(RimeRunTask(unknown_task.as_ptr()), FALSE);
    assert_eq!(RimeRunTask(std::ptr::null()), FALSE);

    let session_id = RimeCreateSession();
    assert_eq!(RimeFindSession(session_id), TRUE);
    RimeCleanupStaleSessions();
    assert_eq!(RimeFindSession(session_id), TRUE);
    assert_eq!(RimeSyncUserData(), TRUE);
    assert_eq!(RimeFindSession(session_id), FALSE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_copies_shared_yaml_to_staging() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-file");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(shared.join("build")).expect("prebuilt dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '2.0'\ndeployed_value: from_shared\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("build").join("default.yaml"),
        "config_version: '1.0'\ndeployed_value: from_prebuilt\n",
    )
    .expect("prebuilt config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    assert!(user.join("build").join("default.yaml").is_file());

    let config_id = CString::new("default").expect("config id should be valid");
    let mut config = empty_config();
    // SAFETY: config id and config pointer are valid for the call.
    assert_eq!(
        unsafe { RimeConfigOpen(config_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "deployed_value").as_deref(),
        Some("from_shared")
    );
    assert!(config_string(&mut config, "__build_info/rime_version")
        .as_deref()
        .is_some_and(|version| version.starts_with("yune-rime-api ")));
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    let staged_root: Value = serde_yaml::from_str(
        &fs::read_to_string(user.join("build").join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should be valid YAML");
    assert!(
        find_config_value(&staged_root, "__build_info/timestamps/default")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let missing = CString::new("missing.yaml").expect("file should be valid");
    assert_eq!(
        RimeDeployConfigFile(missing.as_ptr(), version_key.as_ptr()),
        FALSE
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_uses_build_info_freshness() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-freshness");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    let source = shared.join("default.yaml");
    let destination = staging.join("default.yaml");
    fs::write(
        &source,
        "config_version: '2.0'\ndeployed_value: from_shared\n",
    )
    .expect("shared config should be written");

    let mut staged_root: Value = serde_yaml::from_str(
        "config_version: '2.0'\ndeployed_value: already_staged\nlocal_marker: keep\n",
    )
    .expect("staged config should parse");
    crate::set_build_info(
        &mut staged_root,
        "default",
        crate::source_modified_secs(&source).expect("source mtime should be readable"),
    )
    .expect("build info should be stamped");
    fs::write(
        &destination,
        serde_yaml::to_string(&staged_root).expect("staged config should serialize"),
    )
    .expect("staged config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "deployed_value").and_then(Value::as_str),
        Some("already_staged")
    );
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("keep")
    );

    let mut stale_root = unchanged;
    crate::set_build_info(&mut stale_root, "default", 0).expect("build info should be updated");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale_root).expect("stale config should serialize"),
    )
    .expect("stale config should be written");

    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert_eq!(
        find_config_value(&rebuilt, "deployed_value").and_then(Value::as_str),
        Some("from_shared")
    );
    assert!(find_config_value(&rebuilt, "local_marker").is_none());
    assert!(
        find_config_value(&rebuilt, "__build_info/timestamps/default")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_custom_patch_and_tracks_freshness() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-custom-patch");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
schema:
  name: Base
menu:
  page_size: 5
  options:
    - alpha
switches:
  - name: ascii_mode
    reset: false
schema_list: []
translator:
  dictionary: base
  settings:
    existing: yes
",
    )
    .expect("shared config should be written");
    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  schema/name/+: ' Extended'
  menu/page_size: 9
  menu/options/+: [beta, gamma]
  switches/@0/reset: true
  schema_list/@next: {schema: luna_pinyin}
  translator/+:
    enable_user_dict: true
    settings:
      new: yes
  new/value: patched
",
    )
    .expect("custom patch should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(9)
    );
    assert_eq!(
        find_config_value(&staged, "schema/name").and_then(Value::as_str),
        Some("Base Extended")
    );
    assert_eq!(
        find_config_value(&staged, "menu/options/@0").and_then(Value::as_str),
        Some("alpha")
    );
    assert_eq!(
        find_config_value(&staged, "menu/options/@1").and_then(Value::as_str),
        Some("beta")
    );
    assert_eq!(
        find_config_value(&staged, "menu/options/@2").and_then(Value::as_str),
        Some("gamma")
    );
    assert_eq!(
        find_config_value(&staged, "switches/@0/reset").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@0/schema").and_then(Value::as_str),
        Some("luna_pinyin")
    );
    assert_eq!(
        find_config_value(&staged, "new/value").and_then(Value::as_str),
        Some("patched")
    );
    assert_eq!(
        find_config_value(&staged, "translator/dictionary").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/enable_user_dict").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/existing").and_then(Value::as_str),
        Some("yes")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/new").and_then(Value::as_str),
        Some("yes")
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/default.custom")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  menu/page_size: 7
",
    )
    .expect("custom patch should be updated");
    let mut stale = staged;
    crate::set_build_info(&mut stale, "default.custom", 0)
        .expect("custom build info should be marked stale");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale).expect("stale staged config should serialize"),
    )
    .expect("stale staged config should be written");

    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert_eq!(
        find_config_value(&rebuilt, "menu/page_size").and_then(Value::as_i64),
        Some(7)
    );
    assert!(find_config_value(&rebuilt, "new/value").is_none());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_supports_librime_list_position_references() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-list-positions");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
units:
  - marine
  - zealot
__patch:
  - units/@before 0: probe
  - units/@after 1: medic
  - units/@last: carrier
  - units/@after last: arbiter
  - sparse/@3: observer
",
    )
    .expect("shared config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    let units = ["probe", "marine", "medic", "carrier", "arbiter"];
    for (index, unit) in units.iter().enumerate() {
        assert_eq!(
            find_config_value(&staged, &format!("units/@{index}")).and_then(Value::as_str),
            Some(*unit)
        );
    }
    assert!(find_config_value(&staged, "sparse/@0").is_some_and(Value::is_null));
    assert_eq!(
        find_config_value(&staged, "sparse/@3").and_then(Value::as_str),
        Some("observer")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_explicit_root_patch_without_auto_custom_patch() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-explicit-patch");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
__patch:
  - menu/page_size: 8
  - explicit/value: patched
",
    )
    .expect("shared config should be written");
    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  menu/page_size: 9
  custom_only/value: ignored
",
    )
    .expect("custom patch should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let mut staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "explicit/value").and_then(Value::as_str),
        Some("patched")
    );
    assert!(find_config_value(&staged, "custom_only/value").is_none());
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/default.custom").is_none());

    crate::set_config_value(
        &mut staged,
        "local_marker",
        Value::String("fresh".to_owned()),
    );
    fs::write(
        &destination,
        serde_yaml::to_string(&staged).expect("fresh staged config should serialize"),
    )
    .expect("fresh staged config should be written");

    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("unchanged config should be readable"),
    )
    .expect("unchanged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("fresh")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_schema_file_applies_custom_patch_after_explicit_root_patch() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-schema-explicit-patch-custom");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
translator:
  dictionary: luna
  enable_sentence: false
__patch:
  - translator/enable_completion: true
",
    )
    .expect("shared schema should be written");
    fs::write(
        user.join("luna.custom.yaml"),
        "\
patch:
  translator/enable_sentence: true
",
    )
    .expect("schema custom patch should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let schema_file = CString::new("luna.schema.yaml").expect("file should be valid");
    let version_key = CString::new("schema/version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(schema_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("luna.schema.yaml");
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged schema should be readable"),
    )
    .expect("staged schema should parse");
    assert_eq!(
        find_config_value(&staged, "translator/enable_completion").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "translator/enable_sentence").and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/luna.custom")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    fs::write(
        user.join("luna.custom.yaml"),
        "\
patch:
  translator/enable_sentence: false
",
    )
    .expect("schema custom patch should be updated");
    let mut stale = staged;
    crate::set_build_info(&mut stale, "luna.custom", 0)
        .expect("custom build info should be marked stale");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale).expect("stale staged schema should serialize"),
    )
    .expect("stale staged schema should be written");

    assert_eq!(
        RimeDeployConfigFile(schema_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt schema should be readable"),
    )
    .expect("rebuilt schema should parse");
    assert_eq!(
        find_config_value(&rebuilt, "translator/enable_sentence").and_then(Value::as_bool),
        Some(false)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_local_root_patch_reference() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-local-patch-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
schema_list:
  - schema: base
__patch:
  - local/patch
  - :/local/extra_patch
  - local/missing?
local:
  patch:
    menu/page_size: 8
    schema_list/@next: {schema: patched}
  extra_patch:
    local_marker: patched
",
    )
    .expect("shared config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@1/schema").and_then(Value::as_str),
        Some("patched")
    );
    assert_eq!(
        find_config_value(&staged, "local_marker").and_then(Value::as_str),
        Some("patched")
    );
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/default.custom").is_none());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_external_root_patch_reference() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-external-patch-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    let patch_source = shared.join("patches.yaml");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
schema_list:
  - schema: base
__patch:
  - patches.yaml:/preset/patch
  - missing:/patch?
",
    )
    .expect("shared config should be written");
    fs::write(
        &patch_source,
        "\
preset:
  patch:
    menu/page_size: 8
    schema_list/@next: {schema: external}
    external_marker: patched
",
    )
    .expect("patch config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let mut staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@1/schema").and_then(Value::as_str),
        Some("external")
    );
    assert_eq!(
        find_config_value(&staged, "external_marker").and_then(Value::as_str),
        Some("patched")
    );
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(
        find_config_value(&staged, "__build_info/timestamps/patches")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );
    assert_eq!(
        find_config_value(&staged, "__build_info/timestamps/missing").and_then(Value::as_i64),
        Some(0)
    );
    assert!(find_config_value(&staged, "__build_info/timestamps/default.custom").is_none());

    crate::set_config_value(
        &mut staged,
        "local_marker",
        Value::String("fresh".to_owned()),
    );
    fs::write(
        &destination,
        serde_yaml::to_string(&staged).expect("fresh staged config should serialize"),
    )
    .expect("fresh staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("unchanged config should be readable"),
    )
    .expect("unchanged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("fresh")
    );

    let mut stale = unchanged;
    crate::set_build_info(&mut stale, "patches", 0).expect("patch timestamp should be updated");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale).expect("stale staged config should serialize"),
    )
    .expect("stale staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert!(find_config_value(&rebuilt, "local_marker").is_none());
    assert_eq!(
        find_config_value(&rebuilt, "external_marker").and_then(Value::as_str),
        Some("patched")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_external_root_include_reference() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-external-include-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
__include: base.yaml:/
config_version: '2.0'
menu:
  page_size: 8
schema_list/+:
  - schema: override
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("base.yaml"),
        "\
config_version: '1.0'
menu:
  page_size: 5
  alternative_select_keys: ABC
schema_list:
  - schema: base
",
    )
    .expect("included config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let destination = staging.join("default.yaml");
    let mut staged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "config_version").and_then(Value::as_str),
        Some("2.0")
    );
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(8)
    );
    assert_eq!(
        find_config_value(&staged, "menu/alternative_select_keys").and_then(Value::as_str),
        Some("ABC")
    );
    assert_eq!(
        find_config_value(&staged, "schema_list/@1/schema").and_then(Value::as_str),
        Some("override")
    );
    assert!(find_config_value(&staged, "__include").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/base")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));

    crate::set_config_value(
        &mut staged,
        "local_marker",
        Value::String("fresh".to_owned()),
    );
    fs::write(
        &destination,
        serde_yaml::to_string(&staged).expect("fresh staged config should serialize"),
    )
    .expect("fresh staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let unchanged: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("unchanged config should be readable"),
    )
    .expect("unchanged config should parse");
    assert_eq!(
        find_config_value(&unchanged, "local_marker").and_then(Value::as_str),
        Some("fresh")
    );

    let mut stale = unchanged;
    crate::set_build_info(&mut stale, "base", 0).expect("base timestamp should be updated");
    fs::write(
        &destination,
        serde_yaml::to_string(&stale).expect("stale staged config should serialize"),
    )
    .expect("stale staged config should be written");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let rebuilt: Value = serde_yaml::from_str(
        &fs::read_to_string(&destination).expect("rebuilt config should be readable"),
    )
    .expect("rebuilt config should parse");
    assert!(find_config_value(&rebuilt, "local_marker").is_none());
    assert_eq!(
        find_config_value(&rebuilt, "schema_list/@1/schema").and_then(Value::as_str),
        Some("override")
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_nested_external_include_references() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-nested-include-ref");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
translator:
  __include: base.yaml:/translator
  enable_user_dict: true
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("base.yaml"),
        "\
translator:
  dictionary: base
  settings:
    __include: settings.yaml:/settings
    option: base
",
    )
    .expect("base config should be written");
    fs::write(
        shared.join("settings.yaml"),
        "\
settings:
  fuzzy: true
",
    )
    .expect("settings config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "translator/dictionary").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/enable_user_dict").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/option").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/fuzzy").and_then(Value::as_bool),
        Some(true)
    );
    assert!(find_config_value(&staged, "translator/__include").is_none());
    assert!(find_config_value(&staged, "translator/settings/__include").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/base")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));
    assert!(
        find_config_value(&staged, "__build_info/timestamps/settings")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_merges_include_directives_into_list_nodes() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-include-list-merge");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
combined_units:
  __include: units.yaml:/base_units
  __append:
    - medic
    - goliath
all_units:
  __patch:
    - __append:
        - scv
        - marine
    - __append:
        - firebat
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("units.yaml"),
        "\
base_units:
  - scv
  - marine
",
    )
    .expect("included config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "combined_units/@0").and_then(Value::as_str),
        Some("scv")
    );
    assert_eq!(
        find_config_value(&staged, "combined_units/@3").and_then(Value::as_str),
        Some("goliath")
    );
    assert_eq!(
        find_config_value(&staged, "all_units/@0").and_then(Value::as_str),
        Some("scv")
    );
    assert_eq!(
        find_config_value(&staged, "all_units/@2").and_then(Value::as_str),
        Some("firebat")
    );
    assert!(find_config_value(&staged, "__build_info/timestamps/units")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));
    assert_eq!(
        find_config_value(&staged, "combined_units/@1").and_then(Value::as_str),
        Some("marine")
    );
    assert!(find_config_value(&staged, "combined_units/__include").is_none());
    assert!(find_config_value(&staged, "combined_units/__append").is_none());
    assert!(find_config_value(&staged, "all_units/__patch").is_none());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_expands_include_directives_inside_patch_values() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-patch-value-include");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
combined_units:
  - probe
  - zealot
__patch:
  combined_units/+:
    __include: units.yaml:/terran_units
literal_units:
  __patch:
    __append:
      __include: units.yaml:/zerg_units
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("units.yaml"),
        "\
terran_units:
  - scv
  - marine
zerg_units:
  - drone
  - zergling
",
    )
    .expect("included config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "combined_units/@0").and_then(Value::as_str),
        Some("probe")
    );
    assert_eq!(
        find_config_value(&staged, "combined_units/@3").and_then(Value::as_str),
        Some("marine")
    );
    assert_eq!(
        find_config_value(&staged, "literal_units/@0").and_then(Value::as_str),
        Some("drone")
    );
    assert_eq!(
        find_config_value(&staged, "literal_units/@1").and_then(Value::as_str),
        Some("zergling")
    );
    assert!(find_config_value(&staged, "__patch").is_none());
    assert!(find_config_value(&staged, "literal_units/__patch").is_none());
    assert!(find_config_value(&staged, "__build_info/timestamps/units")
        .and_then(Value::as_i64)
        .is_some_and(|timestamp| timestamp > 0));

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_applies_nested_patch_directives() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-nested-patch");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '2.0'
menu:
  page_size: 5
translator:
  dictionary: base
  settings:
    option: base
    __patch:
      - option: patched
      - patches.yaml:/translator_settings_patch
",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("patches.yaml"),
        "\
translator_settings_patch:
  fuzzy: true
",
    )
    .expect("patch config should be written");
    fs::write(
        user.join("default.custom.yaml"),
        "\
patch:
  menu/page_size: 9
",
    )
    .expect("custom patch should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let config_file = CString::new("default.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("default.yaml"))
            .expect("staged config should be readable"),
    )
    .expect("staged config should parse");
    assert_eq!(
        find_config_value(&staged, "translator/dictionary").and_then(Value::as_str),
        Some("base")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/option").and_then(Value::as_str),
        Some("patched")
    );
    assert_eq!(
        find_config_value(&staged, "translator/settings/fuzzy").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        find_config_value(&staged, "menu/page_size").and_then(Value::as_i64),
        Some(9)
    );
    assert!(find_config_value(&staged, "translator/settings/__patch").is_none());
    assert!(
        find_config_value(&staged, "__build_info/timestamps/patches")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/default.custom")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_config_file_trashes_deprecated_user_copy() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-config-trash");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '2.0'\nsource: shared\n",
    )
    .expect("shared config should be written");
    fs::write(
        user.join("default.yaml"),
        "config_version: '1.0'\nsource: deprecated\n",
    )
    .expect("deprecated user config should be written");
    fs::write(
        shared.join("symbols.yaml"),
        "config_version: '2.0.minimal'\nsource: shared\n",
    )
    .expect("minimal shared config should be written");
    fs::write(
        user.join("symbols.yaml"),
        "config_version: '2.0.custom.123'\nsource: customized\n",
    )
    .expect("customized user config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let default_file = CString::new("default.yaml").expect("file should be valid");
    let symbols_file = CString::new("symbols.yaml").expect("file should be valid");
    let version_key = CString::new("config_version").expect("key should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(
        RimeDeployConfigFile(default_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    assert_eq!(
        RimeDeployConfigFile(symbols_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );

    assert!(!user.join("default.yaml").exists());
    let trashed_default = fs::read_to_string(user.join("trash").join("default.yaml"))
        .expect("deprecated user copy should be trashed");
    assert_eq!(
        trashed_default,
        "config_version: '1.0'\nsource: deprecated\n"
    );
    assert!(!user.join("symbols.yaml").exists());
    let trashed_symbols = fs::read_to_string(user.join("trash").join("symbols.yaml"))
        .expect("customized user copy should be trashed");
    assert_eq!(
        trashed_symbols,
        "config_version: '2.0.custom.123'\nsource: customized\n"
    );
    let staged_default = fs::read_to_string(user.join("build").join("default.yaml"))
        .expect("shared config should be staged");
    let staged_default: Value =
        serde_yaml::from_str(&staged_default).expect("staged default should be valid YAML");
    assert_eq!(
        find_config_value(&staged_default, "source").and_then(Value::as_str),
        Some("shared")
    );
    assert!(
        find_config_value(&staged_default, "__build_info/timestamps/default")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_schema_validates_and_copies_shared_schema_to_staging() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("deploy-schema");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(shared.join("build")).expect("prebuilt dir should be created");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
  version: '2.0'
",
    )
    .expect("shared schema should be written");
    fs::write(
        shared.join("build").join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Old Luna
  version: '1.0'
",
    )
    .expect("prebuilt schema should be written");
    fs::write(
        shared.join("invalid.schema.yaml"),
        "schema:\n  name: Missing Id\n",
    )
    .expect("invalid schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let schema_file = CString::new("luna.schema.yaml").expect("schema file should be valid");
    let invalid_schema = CString::new("invalid.schema.yaml").expect("schema file should be valid");
    let missing_schema = CString::new("missing.schema.yaml").expect("schema file should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeDeploySchema(schema_file.as_ptr()), TRUE);
    assert!(user.join("build").join("luna.schema.yaml").is_file());

    let schema_id = CString::new("luna").expect("schema id should be valid");
    let mut config = empty_config();
    // SAFETY: schema id and config pointer are valid for the call.
    assert_eq!(
        unsafe { RimeSchemaOpen(schema_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_eq!(
        config_string(&mut config, "schema/name").as_deref(),
        Some("Luna")
    );
    assert_eq!(
        config_string(&mut config, "schema/version").as_deref(),
        Some("2.0")
    );
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);

    assert_eq!(RimeDeploySchema(invalid_schema.as_ptr()), FALSE);
    assert_eq!(RimeDeploySchema(missing_schema.as_ptr()), FALSE);
    assert_eq!(RimeDeploySchema(std::ptr::null()), FALSE);

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn prebuild_all_schemas_deploys_shared_schema_files() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("prebuild-all-schemas");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(shared.join("build")).expect("prebuilt dir should be created");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
  version: '2.0'
",
    )
    .expect("luna schema should be written");
    fs::write(
        shared.join("terra.schema.yaml"),
        "\
schema:
  schema_id: terra
  name: Terra
  version: '3.0'
",
    )
    .expect("terra schema should be written");
    fs::write(shared.join("notes.yaml"), "schema_id: ignored\n")
        .expect("non-schema yaml should be written");
    fs::write(
        shared.join("build").join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Old Luna
  version: '1.0'
",
    )
    .expect("prebuilt luna schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let task_name = CString::new("prebuild_all_schemas").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimePrebuildAllSchemas(), TRUE);
    assert!(user.join("build").join("luna.schema.yaml").is_file());
    assert!(user.join("build").join("terra.schema.yaml").is_file());
    assert!(!user.join("build").join("notes.yaml").exists());

    let luna_id = CString::new("luna").expect("schema id should be valid");
    let mut luna = empty_config();
    // SAFETY: schema id and config pointer are valid for the call.
    assert_eq!(unsafe { RimeSchemaOpen(luna_id.as_ptr(), &mut luna) }, TRUE);
    assert_eq!(
        config_string(&mut luna, "schema/name").as_deref(),
        Some("Luna")
    );
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut luna) }, TRUE);

    fs::remove_file(user.join("build").join("terra.schema.yaml"))
        .expect("staged terra schema should be removable");
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);
    assert!(user.join("build").join("terra.schema.yaml").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn workspace_update_rebuilds_source_dictionary_artifacts_and_reuses_fresh_outputs() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-dictionary-rebuild");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '1.0'\nschema_list:\n  - schema: luna\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\n  version: '1'\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: luna\n",
    )
    .expect("schema should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---\nname: luna\nversion: '1'\nsort: by_weight\n...\n\n八\tba\t2\n爸\tba\t1\n",
    )
    .expect("dictionary should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    let reports = workspace_dictionary_rebuild_reports();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].dictionary_id, "luna");
    assert_eq!(
        reports[0].report.table,
        yune_core::RimeDictArtifactStatus::Rebuilt
    );
    assert_eq!(
        reports[0].report.prism,
        yune_core::RimeDictArtifactStatus::Rebuilt
    );
    assert_eq!(
        reports[0].report.reverse,
        yune_core::RimeDictArtifactStatus::Rebuilt
    );
    for file_name in ["luna.table.bin", "luna.prism.bin", "luna.reverse.bin"] {
        assert!(user.join("build").join(file_name).is_file());
    }
    assert!(yune_core::parse_rime_table_bin_dictionary(
        fs::read(user.join("build").join("luna.table.bin")).expect("table should be readable")
    )
    .is_ok());
    let prism = yune_core::parse_rime_prism_bin_payload(
        fs::read(user.join("build").join("luna.prism.bin")).expect("prism should be readable"),
    )
    .expect("prism should parse");
    assert!(prism.double_array_size > 0);
    assert!(prism.double_array.is_some());
    assert!(yune_core::parse_rime_reverse_bin_dictionary(
        fs::read(user.join("build").join("luna.reverse.bin")).expect("reverse should be readable")
    )
    .is_ok());

    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    let fresh = workspace_dictionary_rebuild_reports();
    assert_eq!(
        fresh[0].report.table,
        yune_core::RimeDictArtifactStatus::ReusedFresh
    );
    assert_eq!(
        fresh[0].report.prism,
        yune_core::RimeDictArtifactStatus::ReusedFresh
    );
    assert_eq!(
        fresh[0].report.reverse,
        yune_core::RimeDictArtifactStatus::ReusedFresh
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn workspace_update_rebuilds_after_pack_changes_and_honors_force_flags() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-dictionary-pack-force");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '1.0'\nschema_list:\n  - schema: luna\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: luna\n  packs:\n    - luna_pack\n",
    )
    .expect("schema should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---\nname: luna\nversion: '1'\nsort: by_weight\n...\n\n八\tba\t2\n",
    )
    .expect("dictionary should be written");
    fs::write(
        shared.join("luna_pack.dict.yaml"),
        "\
---\nname: luna_pack\nversion: '1'\nsort: by_weight\n...\n\n爸\tba\t1\n",
    )
    .expect("pack should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    let initial_table = yune_core::parse_rime_table_bin_dictionary(
        fs::read(user.join("build").join("luna.table.bin")).expect("table should be readable"),
    )
    .expect("table should parse");
    assert!(initial_table
        .entries()
        .iter()
        .any(|entry| entry.text == "爸"));

    fs::write(
        shared.join("luna_pack.dict.yaml"),
        "\
---\nname: luna_pack\nversion: '2'\nsort: by_weight\n...\n\n爸\tba\t1\n吧\tba\t3\n",
    )
    .expect("pack update should be written");
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    let pack_changed = workspace_dictionary_rebuild_reports();
    assert_eq!(
        pack_changed[0].report.table,
        yune_core::RimeDictArtifactStatus::Rebuilt
    );
    assert_eq!(
        pack_changed[0].report.prism,
        yune_core::RimeDictArtifactStatus::Rebuilt
    );
    assert_eq!(
        pack_changed[0].report.reverse,
        yune_core::RimeDictArtifactStatus::Rebuilt
    );
    let updated_table = yune_core::parse_rime_table_bin_dictionary(
        fs::read(user.join("build").join("luna.table.bin")).expect("table should be readable"),
    )
    .expect("table should parse");
    assert!(updated_table
        .entries()
        .iter()
        .any(|entry| entry.text == "吧"));

    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: luna\n  packs:\n    - luna_pack\n  force_rebuild_prism: true\n",
    )
    .expect("force schema should be written");
    fs::remove_file(user.join("build").join("luna.schema.yaml"))
        .expect("deployed schema should be removed");
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    let forced = workspace_dictionary_rebuild_reports();
    assert_eq!(
        forced[0].report.table,
        yune_core::RimeDictArtifactStatus::ReusedFresh
    );
    assert_eq!(
        forced[0].report.prism,
        yune_core::RimeDictArtifactStatus::Rebuilt
    );
    assert_eq!(
        forced[0].report.reverse,
        yune_core::RimeDictArtifactStatus::ReusedFresh
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn workspace_update_reuses_prebuilt_artifacts_when_source_is_missing() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-dictionary-prebuilt");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(shared.join("build")).expect("prebuilt dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '1.0'\nschema_list:\n  - schema: luna\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:\n  schema_id: luna\n  name: Luna\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: luna\n",
    )
    .expect("schema should be written");
    fs::write(
        shared.join("luna.dict.yaml"),
        "\
---\nname: luna\nversion: '1'\nsort: by_weight\n...\n\n八\tba\t2\n爸\tba\t1\n",
    )
    .expect("dictionary should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    for file_name in ["luna.table.bin", "luna.prism.bin", "luna.reverse.bin"] {
        fs::copy(
            user.join("build").join(file_name),
            shared.join("build").join(file_name),
        )
        .expect("prebuilt artifact should be copied");
        fs::remove_file(user.join("build").join(file_name))
            .expect("staging artifact should be removed");
    }
    fs::remove_file(shared.join("luna.dict.yaml")).expect("source should be removed");

    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    let reports = workspace_dictionary_rebuild_reports();
    assert_eq!(reports.len(), 1);
    assert_eq!(
        reports[0].report.table,
        yune_core::RimeDictArtifactStatus::ReusedPrebuilt
    );
    assert_eq!(
        reports[0].report.prism,
        yune_core::RimeDictArtifactStatus::ReusedPrebuilt
    );
    assert_eq!(
        reports[0].report.reverse,
        yune_core::RimeDictArtifactStatus::ReusedPrebuilt
    );
    for file_name in ["luna.table.bin", "luna.prism.bin", "luna.reverse.bin"] {
        assert!(user.join("build").join(file_name).is_file());
    }

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn workspace_update_fails_for_unsafe_or_missing_dictionary_artifacts() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-dictionary-failures");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: '1.0'\nschema_list:\n  - schema: unsafe\n  - schema: missing\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("unsafe.schema.yaml"),
        "\
schema:\n  schema_id: unsafe\n  name: Unsafe\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: ../secret\n",
    )
    .expect("unsafe schema should be written");
    fs::write(
        shared.join("missing.schema.yaml"),
        "\
schema:\n  schema_id: missing\n  name: Missing\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: missing\n",
    )
    .expect("missing schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), FALSE);
    assert!(workspace_dictionary_rebuild_reports().is_empty());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn workspace_update_deploys_default_schemas_and_dependencies() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-update");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "\
config_version: '1.0'
schema_list:
  - schema: luna
  - schema: terra
    case:
      - disabled_flag
",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
  dependencies:
    - luna_ext
",
    )
    .expect("luna schema should be written");
    fs::write(
        shared.join("luna_ext.schema.yaml"),
        "\
schema:
  schema_id: luna_ext
  name: Luna Extension
  dependencies:
    - luna_ext_extra
",
    )
    .expect("dependency schema should be written");
    fs::write(
        shared.join("luna_ext_extra.schema.yaml"),
        "\
schema:
  schema_id: luna_ext_extra
  name: Luna Extension Extra
",
    )
    .expect("transitive dependency schema should be written");
    fs::write(
        shared.join("terra.schema.yaml"),
        "\
schema:
  schema_id: terra
  name: Terra
",
    )
    .expect("terra schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let user_dict_task = CString::new("user_dict_upgrade").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    assert_eq!(RimeRunTask(user_dict_task.as_ptr()), TRUE);
    let dictionary_reports = workspace_dictionary_rebuild_reports();
    assert!(dictionary_reports.is_empty());
    for file_name in [
        "default.yaml",
        "luna.schema.yaml",
        "luna_ext.schema.yaml",
        "terra.schema.yaml",
    ] {
        assert!(user.join("build").join(file_name).is_file());
    }
    assert!(!user
        .join("build")
        .join("luna_ext_extra.schema.yaml")
        .is_file());

    let luna_ext_id = CString::new("luna_ext").expect("schema id should be valid");
    let mut luna_ext = empty_config();
    // SAFETY: schema id and config pointer are valid for the call.
    assert_eq!(
        unsafe { RimeSchemaOpen(luna_ext_id.as_ptr(), &mut luna_ext) },
        TRUE
    );
    assert_eq!(
        config_string(&mut luna_ext, "schema/name").as_deref(),
        Some("Luna Extension")
    );
    // SAFETY: config was opened by the config API.
    assert_eq!(unsafe { RimeConfigClose(&mut luna_ext) }, TRUE);

    let user_yaml = fs::read_to_string(user.join("user.yaml"))
        .expect("workspace update should write user config");
    let user_config: Value =
        serde_yaml::from_str(&user_yaml).expect("user config should be valid yaml");
    assert!(
        find_config_value(&user_config, "var/last_build_time")
            .and_then(Value::as_i64)
            .unwrap_or_default()
            > 0
    );

    fs::write(user.join("stale.bin"), "stale").expect("trash fixture should be written");
    assert_eq!(RimeDeployWorkspace(), TRUE);
    assert!(user.join("trash").join("stale.bin").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[cfg(unix)]
#[test]
fn workspace_update_removes_legacy_shared_data_symlinks() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-symlinks");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("default schema should be written");
    fs::write(shared.join("legacy.table.bin"), "prebuilt")
        .expect("shared prebuilt fixture should be written");
    fs::write(user.join("local.table.bin"), "local").expect("local fixture should be written");
    std::os::unix::fs::symlink(
        shared.join("legacy.table.bin"),
        user.join("legacy.table.bin"),
    )
    .expect("shared symlink should be created");
    std::os::unix::fs::symlink(
        user.join("local.table.bin"),
        user.join("local-link.table.bin"),
    )
    .expect("local symlink should be created");
    std::os::unix::fs::symlink(
        shared.join("missing.table.bin"),
        user.join("missing.table.bin"),
    )
    .expect("dangling symlink should be created");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let workspace_task = CString::new("workspace_update").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);

    assert!(fs::symlink_metadata(user.join("legacy.table.bin")).is_err());
    assert!(fs::symlink_metadata(user.join("missing.table.bin")).is_err());
    assert!(user.join("local-link.table.bin").exists());
    assert!(user.join("build").join("default.schema.yaml").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn cleanup_trash_moves_librime_deployer_artifacts() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("cleanup-trash");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("shared config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("shared schema should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let cleanup_task = CString::new("cleanup_trash").expect("task name should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    for file_name in [
        "rime.log",
        "build.bin",
        "luna_pinyin.reverse.kct",
        "luna_pinyin.userdb.kct.old",
        "luna_pinyin.userdb.kct.snapshot",
    ] {
        fs::write(user.join(file_name), file_name).expect("cleanup fixture should be written");
    }
    fs::write(user.join("default.yaml"), "schema_list: []\n")
        .expect("kept config should be written");

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(cleanup_task.as_ptr()), TRUE);

    for file_name in [
        "rime.log",
        "build.bin",
        "luna_pinyin.reverse.kct",
        "luna_pinyin.userdb.kct.old",
        "luna_pinyin.userdb.kct.snapshot",
    ] {
        assert!(!user.join(file_name).exists());
        assert!(user.join("trash").join(file_name).is_file());
    }
    assert!(user.join("default.yaml").is_file());

    fs::write(user.join("stale.bin"), "stale").expect("deploy fixture should be written");
    assert_eq!(RimeDeployWorkspace(), TRUE);
    assert!(user.join("trash").join("stale.bin").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn clean_old_log_files_removes_stale_app_logs() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("clean-old-log-files");
    let shared = root.join("shared");
    let user = root.join("user");
    let logs = root.join("logs");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&logs).expect("log dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("default schema should be written");
    let today_log = format!("rime_test{}.log", current_log_date_marker());
    for file_name in [
        "rime_test.20000101.log",
        "rime_test.20000102.log",
        "other_app.20000101.log",
        "rime_test.20000101.txt",
        &today_log,
    ] {
        fs::write(logs.join(file_name), file_name).expect("log fixture should be written");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink("rime_test.20000102.log", logs.join("rime_test.INFO"))
        .expect("active log symlink should be created");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let logs_c = CString::new(logs.to_string_lossy().as_ref()).expect("path should be valid");
    let app_name = CString::new("rime_test").expect("app name should be valid");
    let cleanup_task = CString::new("clean_old_log_files").expect("task should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    traits.app_name = app_name.as_ptr();
    traits.log_dir = logs_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };
    assert_eq!(RimeRunTask(cleanup_task.as_ptr()), TRUE);

    assert!(!logs.join("rime_test.20000101.log").exists());
    #[cfg(unix)]
    assert!(logs.join("rime_test.20000102.log").is_file());
    #[cfg(not(unix))]
    assert!(!logs.join("rime_test.20000102.log").exists());
    assert!(logs.join("other_app.20000101.log").is_file());
    assert!(logs.join("rime_test.20000101.txt").is_file());
    assert!(logs.join(today_log).is_file());

    fs::write(logs.join("rime_test.19991231.log"), "stale")
        .expect("maintenance log fixture should be written");
    assert_eq!(RimeStartMaintenance(TRUE), TRUE);
    assert!(!logs.join("rime_test.19991231.log").exists());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn maintenance_on_workspace_change_detects_yaml_modifications() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("detect-modifications");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("default.yaml"),
        "config_version: test\nschema_list:\n  - schema: default\n",
    )
    .expect("default config should be written");
    fs::write(
        shared.join("default.schema.yaml"),
        "schema:\n  schema_id: default\n  name: Default\n",
    )
    .expect("default schema should be written");
    fs::write(
        user.join("user.yaml"),
        "var:\n  last_build_time: 2147483647\n",
    )
    .expect("user config should be written");
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();

    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };
    assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), FALSE);

    fs::write(user.join("user.yaml"), "var:\n  last_build_time: 0\n")
        .expect("user config should be updated");
    assert_eq!(RimeStartMaintenanceOnWorkspaceChange(), TRUE);
    assert!(user.join("build").join("default.yaml").is_file());
    assert!(user.join("build").join("default.schema.yaml").is_file());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn sync_user_data_emits_librime_deploy_notifications() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("sync-notification-events");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(user.join("default.yaml"), "config_version: test\n")
        .expect("user config should be written");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };
    notification_events_lock().clear();
    let context_object = 0x6b_usize as *mut c_void;

    RimeSetNotificationHandler(Some(record_notification), context_object);
    assert_eq!(RimeSyncUserData(), TRUE);

    let events = notification_events_lock();
    assert_eq!(
        *events,
        vec![
            NotificationEvent {
                context_object: 0x6b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "start".to_owned(),
            },
            NotificationEvent {
                context_object: 0x6b,
                session_id: 0,
                message_type: "deploy".to_owned(),
                message_value: "success".to_owned(),
            },
        ]
    );
    drop(events);

    RimeSetNotificationHandler(None, std::ptr::null_mut());
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_schema_expands_librime_punctuator_import_preset() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-punctuator-import-preset");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("symbols.yaml"),
        "\
punctuator:
  half_shape:
    \"/\": \"、\"
  symbols:
    \"/fh\": [\"©\", \"®\"]
",
    )
    .expect("preset config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - punct_translator
    - echo_translator
punctuator:
  import_preset: symbols
  half_shape:
    \"/\": \"schema-slash\"
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };

    let config_file = CString::new("luna.schema.yaml").expect("file name should be valid");
    let version_key = CString::new("schema/version").expect("version key should be valid");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("luna.schema.yaml"))
            .expect("staged schema should be readable"),
    )
    .expect("staged schema should parse");
    let half_shape = find_config_value(&staged, "punctuator/half_shape")
        .and_then(Value::as_mapping)
        .expect("half-shape punctuation map should be staged");
    assert_eq!(
        half_shape
            .get(Value::String("/".to_owned()))
            .and_then(Value::as_str),
        Some("schema-slash")
    );
    let symbols = find_config_value(&staged, "punctuator/symbols")
        .and_then(Value::as_mapping)
        .expect("symbol punctuation map should be staged");
    let slash_fh = symbols
        .get(Value::String("/fh".to_owned()))
        .and_then(Value::as_sequence)
        .expect("imported /fh symbol list should be staged");
    assert_eq!(slash_fh.first().and_then(Value::as_str), Some("©"));
    assert!(
        find_config_value(&staged, "__build_info/timestamps/symbols")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let candidate_texts = |input: &str| {
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as i32, 0), TRUE);
        }
        let mut context = empty_context();
        // SAFETY: context points to writable storage initialized with positive
        // `data_size`.
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(
                context.menu.candidates,
                context.menu.num_candidates as usize,
            )
        };
        let texts = candidates
            .iter()
            .map(|candidate| {
                // SAFETY: candidate text pointers are populated by
                // `RimeGetContext`.
                unsafe { CStr::from_ptr(candidate.text) }
                    .to_str()
                    .expect("candidate text should be valid UTF-8")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        // SAFETY: nested pointers were allocated by `RimeGetContext` above.
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        RimeClearComposition(session_id);
        texts
    };

    assert_eq!(candidate_texts("/"), ["schema-slash", "/"]);
    assert_eq!(candidate_texts("/fh"), ["©", "®", "/fh"]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_schema_expands_librime_recognizer_import_preset() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-recognizer-import-preset");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("recognizers.yaml"),
        "\
recognizer:
  use_space: true
  patterns:
    reverse_lookup: \"`[a-z ]*$\"
",
    )
    .expect("preset config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - recognizer
  segmentors:
    - abc_segmentor
    - matcher
  translators:
    - reverse_lookup_translator
    - echo_translator
recognizer:
  import_preset: recognizers
  patterns:
    local_tag: \"^local$\"
reverse_lookup:
  dictionary: stroke
  prefix: \"`\"
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("stroke.dict.yaml"),
        "\
---
name: stroke
version: '0.1'
sort: original
...

火山\thuo shan
",
    )
    .expect("reverse lookup dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };

    let config_file = CString::new("luna.schema.yaml").expect("file name should be valid");
    let version_key = CString::new("schema/version").expect("version key should be valid");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("luna.schema.yaml"))
            .expect("staged schema should be readable"),
    )
    .expect("staged schema should parse");
    assert_eq!(
        find_config_value(&staged, "recognizer/use_space").and_then(Value::as_bool),
        Some(true)
    );
    let patterns = find_config_value(&staged, "recognizer/patterns")
        .and_then(Value::as_mapping)
        .expect("recognizer patterns should be staged");
    assert_eq!(
        patterns
            .get(Value::String("reverse_lookup".to_owned()))
            .and_then(Value::as_str),
        Some("`[a-z ]*$")
    );
    assert_eq!(
        patterns
            .get(Value::String("local_tag".to_owned()))
            .and_then(Value::as_str),
        Some("^local$")
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/recognizers")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in "`huo shan".chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }

    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive
    // `data_size`.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let texts = candidates
        .iter()
        .map(|candidate| {
            // SAFETY: candidate text pointers are populated by `RimeGetContext`.
            unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned()
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    assert_eq!(texts, ["火山".to_owned(), "`huo shan".to_owned()]);

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn deploy_schema_expands_librime_key_binder_import_preset() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-key-binder-import-preset");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::write(
        shared.join("key_bindings.yaml"),
        "\
key_binder:
  bindings:
    - { when: always, accept: Control+Shift+1, toggle: ascii_mode }
",
    )
    .expect("preset config should be written");
    fs::write(
        shared.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  processors:
    - key_binder
  translators:
    - echo_translator
key_binder:
  import_preset: key_bindings
  bindings:
    - { when: always, accept: Control+Shift+2, toggle: full_shape }
",
    )
    .expect("schema config should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeDeployerInitialize(&traits) };

    let config_file = CString::new("luna.schema.yaml").expect("file name should be valid");
    let version_key = CString::new("schema/version").expect("version key should be valid");
    assert_eq!(
        RimeDeployConfigFile(config_file.as_ptr(), version_key.as_ptr()),
        TRUE
    );
    let staged: Value = serde_yaml::from_str(
        &fs::read_to_string(staging.join("luna.schema.yaml"))
            .expect("staged schema should be readable"),
    )
    .expect("staged schema should parse");
    let bindings = find_config_value(&staged, "key_binder/bindings")
        .and_then(Value::as_sequence)
        .expect("key bindings should be staged");
    assert_eq!(
        bindings
            .first()
            .and_then(|binding| find_config_value(binding, "toggle"))
            .and_then(Value::as_str),
        Some("ascii_mode")
    );
    assert_eq!(
        bindings
            .get(1)
            .and_then(|binding| find_config_value(binding, "toggle"))
            .and_then(Value::as_str),
        Some("full_shape")
    );
    assert!(
        find_config_value(&staged, "__build_info/timestamps/key_bindings")
            .and_then(Value::as_i64)
            .is_some_and(|timestamp| timestamp > 0)
    );

    let session_id = RimeCreateSession();
    let schema_id = CString::new("luna").expect("schema id should be valid");
    let ascii_mode = CString::new("ascii_mode").expect("option name should be valid");
    let full_shape = CString::new("full_shape").expect("option name should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '1' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    assert_eq!(
        RimeProcessKey(session_id, '2' as c_int, K_CONTROL_MASK | K_SHIFT_MASK),
        TRUE
    );
    // SAFETY: option names are valid NUL-terminated strings.
    assert_eq!(
        unsafe { RimeGetOption(session_id, ascii_mode.as_ptr()) },
        TRUE
    );
    assert_eq!(
        unsafe { RimeGetOption(session_id, full_shape.as_ptr()) },
        TRUE
    );

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
