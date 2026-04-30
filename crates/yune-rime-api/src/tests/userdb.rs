use super::*;

#[test]
fn userdb_import_export_round_trips_typed_commits_dee_and_tick_values() {
    let _guard = test_guard();
    let root = unique_temp_dir("userdb-typed-roundtrip");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    let import = root.join("import.txt");
    let export = root.join("export.txt");
    fs::write(&import, "你好\tni hao\t3\n").expect("table import should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let dict = CString::new("luna_pinyin").expect("dict name should be valid");
    let import_c = CString::new(import.to_string_lossy().as_ref()).expect("path is valid");
    let export_c = CString::new(export.to_string_lossy().as_ref()).expect("path is valid");
    // SAFETY: pointers reference valid NUL-terminated strings for the calls.
    assert_eq!(
        unsafe { RimeLeversImportUserDict(dict.as_ptr(), import_c.as_ptr()) },
        1
    );
    // SAFETY: pointers reference valid NUL-terminated strings for the calls.
    assert_eq!(
        unsafe { RimeLeversExportUserDict(dict.as_ptr(), export_c.as_ptr()) },
        1
    );

    let stored =
        fs::read_to_string(user.join("luna_pinyin.userdb")).expect("store should be readable");
    assert!(
        stored.contains("c=3"),
        "stored value should preserve commits: {stored}"
    );
    assert!(
        stored.contains("d=3"),
        "stored value should preserve dee: {stored}"
    );
    assert!(
        stored.contains("t=1"),
        "stored value should preserve tick: {stored}"
    );

    let exported = fs::read_to_string(export).expect("export should be readable");
    assert_eq!(exported, "你好\tni hao\t3\n");

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    let _ = fs::remove_dir_all(root);
}

#[test]
fn userdb_rejects_malformed_logical_names_before_store_creation() {
    let _guard = test_guard();
    let root = unique_temp_dir("userdb-invalid-names");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    let import = root.join("import.txt");
    fs::write(&import, "你好\tni hao\t3\n").expect("table import should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let import_c = CString::new(import.to_string_lossy().as_ref()).expect("path is valid");
    for name in [
        "../x",
        "/tmp/x",
        "x.userdb",
        "x.userdb.txt",
        "C:\\x",
        "~x",
        "",
        ".",
        "..",
    ] {
        let name_c = CString::new(name).expect("dict name should be representable");
        // SAFETY: pointers reference valid NUL-terminated strings for the call.
        assert_eq!(
            unsafe { RimeLeversImportUserDict(name_c.as_ptr(), import_c.as_ptr()) },
            -1,
            "{name:?}"
        );
    }

    assert!(fs::read_dir(&user)
        .expect("user dir should exist")
        .next()
        .is_none());

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    let _ = fs::remove_dir_all(root);
}

#[test]
fn interrupted_userdb_temp_write_keeps_last_committed_store_readable() {
    let _guard = test_guard();
    let root = unique_temp_dir("userdb-interrupted-write");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    let store = user.join("luna_pinyin.userdb");
    fs::write(&store, "# yune userdb\n/db_name\tluna_pinyin\n/db_type\tuserdb\n/tick\t1\nni hao \t你好\tc=2 d=2 t=1\n")
        .expect("committed store should be written");
    fs::write(user.join("luna_pinyin.userdb.tmp"), "partial\n")
        .expect("temp store should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let dict = CString::new("luna_pinyin").expect("dict name should be valid");
    let export = root.join("export.txt");
    let export_c = CString::new(export.to_string_lossy().as_ref()).expect("path is valid");
    // SAFETY: pointers reference valid NUL-terminated strings for the call.
    assert_eq!(
        unsafe { RimeLeversExportUserDict(dict.as_ptr(), export_c.as_ptr()) },
        1
    );
    assert_eq!(
        fs::read_to_string(export).expect("export should be readable"),
        "你好\tni hao\t2\n"
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sync_user_data_merges_plain_userdb_snapshots_and_backs_up_current_state() {
    let _guard = test_guard();
    let root = unique_temp_dir("rime-sync-user-data");
    let user = root.join("user");
    let peer_sync = user.join("sync").join("peer");
    fs::create_dir_all(&user).expect("user dir should be created");
    fs::create_dir_all(&peer_sync).expect("peer sync dir should be created");
    struct CurrentDirGuard(PathBuf);
    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }
    let current_dir_guard =
        CurrentDirGuard(env::current_dir().expect("current dir should be available"));
    env::set_current_dir(&root).expect("test cwd should move under temp root");

    fs::write(user.join("luna_pinyin.userdb"), "ni hao\t你好\t1\n")
        .expect("local user dict should be written");
    fs::write(user.join("default.yaml"), "config_version: '1.0'\n")
        .expect("user config should be written");
    fs::write(user.join("notes.txt"), "local notes\n").expect("text file should be written");
    fs::write(
        user.join("generated.yaml"),
        "customization: abc123\nschema:\n  name: Generated\n",
    )
    .expect("generated customized copy should be written");
    fs::write(
        peer_sync.join("luna_pinyin.userdb.txt"),
        "ni hao\t你好\t1\nzhong guo\t中国\t2\n",
    )
    .expect("peer snapshot should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    assert_eq!(RimeFindSession(session_id), TRUE);
    assert_eq!(RimeSyncUserData(), TRUE);
    assert_eq!(RimeFindSession(session_id), FALSE);

    let merged =
        fs::read_to_string(user.join("luna_pinyin.userdb")).expect("dict should be readable");
    assert!(merged.contains("/db_name\tluna_pinyin\n"));
    assert!(merged.contains("/db_type\tuserdb\n"));
    assert!(merged.contains("ni hao \t你好\tc=1 d=1 t=1\n"));
    assert!(merged.contains("zhong guo \t中国\tc=2 d=2 t=1\n"));

    let installation_metadata = fs::read_to_string(user.join("installation.yaml"))
        .expect("installation metadata should be written during sync");
    let installation_metadata: Value =
        serde_yaml::from_str(&installation_metadata).expect("installation metadata should parse");
    let installation_id = find_config_value(&installation_metadata, "installation_id")
        .and_then(Value::as_str)
        .expect("installation id should be available");
    let sync_user_dir = user.join("sync").join(installation_id);
    let backup = fs::read_to_string(sync_user_dir.join("luna_pinyin.userdb.txt"))
        .expect("current user snapshot should be written");
    assert!(backup.contains("/db_name\tluna_pinyin\n"));
    assert!(backup.contains("/db_type\tuserdb\n"));
    assert!(backup.contains("ni hao \t你好\tc=1 d=1 t=1\n"));
    assert!(backup.contains("zhong guo \t中国\tc=2 d=2 t=1\n"));

    assert_eq!(
        fs::read_to_string(sync_user_dir.join("default.yaml"))
            .expect("user config backup should be readable"),
        "config_version: '1.0'\n"
    );
    assert_eq!(
        fs::read_to_string(sync_user_dir.join("notes.txt"))
            .expect("text backup should be readable"),
        "local notes\n"
    );
    assert!(!sync_user_dir.join("generated.yaml").exists());

    let task_name = CString::new("user_dict_sync").expect("task name should be valid");
    assert_eq!(RimeRunTask(task_name.as_ptr()), TRUE);
    fs::remove_file(sync_user_dir.join("default.yaml")).expect("config backup should be removable");
    let backup_config_task =
        CString::new("backup_config_files").expect("task name should be valid");
    assert_eq!(RimeRunTask(backup_config_task.as_ptr()), TRUE);
    assert_eq!(
        fs::read_to_string(sync_user_dir.join("default.yaml"))
            .expect("task should recreate config backup"),
        "config_version: '1.0'\n"
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    drop(current_dir_guard);
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
