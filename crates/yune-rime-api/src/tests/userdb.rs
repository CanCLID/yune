use super::*;

#[test]
fn userdb_backup_restore_exports_typed_metadata_and_records() {
    let _guard = test_guard();
    let root = unique_temp_dir("userdb-typed-roundtrip");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    struct CurrentDirGuard(PathBuf);
    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }
    let current_dir_guard =
        CurrentDirGuard(env::current_dir().expect("current dir should be available"));
    env::set_current_dir(&root).expect("test cwd should move under temp root");
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

    let snapshot = root
        .join("sync")
        .join("unknown")
        .join("luna_pinyin.userdb.txt");
    let dict_for_backup = CString::new("luna_pinyin").expect("dict name should be valid");
    // SAFETY: pointer references a valid NUL-terminated string for the call.
    assert_eq!(
        unsafe { RimeLeversBackupUserDict(dict_for_backup.as_ptr()) },
        TRUE
    );
    let snapshot_text = fs::read_to_string(&snapshot).expect("snapshot should be readable");
    assert!(snapshot_text.contains("/db_name\tluna_pinyin\n"));
    assert!(snapshot_text.contains("/db_type\tuserdb\n"));
    assert!(snapshot_text.contains("ni hao \t你好\tc=3 d=3 t=1\n"));

    fs::remove_file(user.join("luna_pinyin.userdb")).expect("store should be removable");
    let snapshot_c = CString::new(snapshot.to_string_lossy().as_ref()).expect("path is valid");
    // SAFETY: pointer references a valid NUL-terminated string for the call.
    assert_eq!(
        unsafe { RimeLeversRestoreUserDict(snapshot_c.as_ptr()) },
        TRUE
    );
    let restored =
        fs::read_to_string(user.join("luna_pinyin.userdb")).expect("store should be restored");
    assert!(restored.contains("ni hao \t你好\tc=3 d=3 t=1\n"));

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    drop(current_dir_guard);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn userdb_learning_persists_session_commits_and_reloads_candidates() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("userdb-learning-session");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("learn.schema.yaml"),
        "schema:\n  schema_id: learn\n  name: Learn\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: learn\n",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("learn.dict.yaml"),
        "---\nname: learn\nversion: '1'\nsort: original\ncolumns: [code, text, weight]\n...\nni\t你\t10\nni hao\t你好\t9\n",
    )
    .expect("dictionary should be written");

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let session_id = RimeCreateSession();
    let schema = CString::new("learn").expect("schema id should be valid");
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(session_id, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(session_id, 'i' as c_int, 0), TRUE);
    assert_eq!(RimeCommitComposition(session_id), TRUE);
    assert_eq!(RimeDestroySession(session_id), TRUE);

    let stored = fs::read_to_string(user.join("learn.userdb")).expect("store should be readable");
    assert!(
        stored.contains("ni \t你\tc=1"),
        "typed update should persist: {stored}"
    );
    assert!(
        stored.contains("d="),
        "typed update should include dee: {stored}"
    );
    assert!(
        stored.contains("t="),
        "typed update should include tick: {stored}"
    );
    let seeded_store = format!("{stored}ni hao \t你好\tc=1 d=1 t=1\n");
    fs::write(user.join("learn.userdb"), seeded_store)
        .expect("predictive store entry should be appended");

    let reloaded_session = RimeCreateSession();
    assert_eq!(
        unsafe { RimeSelectSchema(reloaded_session, schema.as_ptr()) },
        TRUE
    );
    assert_eq!(RimeProcessKey(reloaded_session, 'n' as c_int, 0), TRUE);
    assert_eq!(RimeProcessKey(reloaded_session, 'i' as c_int, 0), TRUE);
    let candidates =
        super::super::session_candidates_snapshot(reloaded_session).expect("session should exist");
    let learned_index = candidates
        .iter()
        .position(|candidate| {
            candidate.source == CandidateSource::UserTable && candidate.text == "你"
        })
        .expect("learned exact userdb candidate should be present");
    let table_index = candidates
        .iter()
        .position(|candidate| candidate.source == CandidateSource::Table && candidate.text == "你")
        .expect("table candidate should remain present");
    assert!(
        learned_index < table_index,
        "learned candidate should rank before table duplicate: {candidates:?}"
    );
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.text == "你好" && candidate.comment == "~hao"),
        "predictive userdb candidate should be present: {candidates:?}"
    );

    assert_eq!(RimeDestroySession(reloaded_session), TRUE);
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

    let malformed = root.join("malformed.userdb.txt");
    fs::write(
        &malformed,
        "/db_name\t../bad\n/db_type\tuserdb\nni hao\t你好\tc=9 d=9 t=9\n",
    )
    .expect("malformed snapshot should be written");
    let before = fs::read_dir(&user).expect("user dir should exist").count();
    let malformed_c = CString::new(malformed.to_string_lossy().as_ref()).expect("path is valid");
    // SAFETY: pointer references a valid NUL-terminated string for the call.
    assert_eq!(
        unsafe { RimeLeversRestoreUserDict(malformed_c.as_ptr()) },
        FALSE
    );
    assert_eq!(
        fs::read_dir(&user)
            .expect("user dir should still exist")
            .count(),
        before
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    let _ = fs::remove_dir_all(root);
}

#[test]
fn userdb_recovery_interrupted_temp_write_keeps_last_committed_store_readable() {
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
fn userdb_sync_merges_plain_snapshots_and_backs_up_current_state() {
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

    fs::write(
        user.join("luna_pinyin.userdb"),
        "ni hao\t你好\t1\nshuo\t说\t1\n",
    )
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
        "# Rime user dictionary\n/db_name\tluna_pinyin\n/db_type\tuserdb\n/tick\t5\n/user_id\tpeer\nni hao\t你好\tc=4 d=4 t=2\nshuo\t说\tc=-7 d=7 t=3\nzhong guo\t中国\tc=2 d=2 t=5\n",
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
    assert!(merged.contains("ni hao \t你好\tc=4 d=4 t=5\n"));
    assert!(merged.contains("shuo \t说\tc=-7 d=7 t=5\n"));
    assert!(merged.contains("zhong guo \t中国\tc=2 d=2 t=5\n"));

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
    assert!(backup.contains(&format!("/user_id\t{installation_id}\n")));
    assert!(!backup.contains("/user_id\tpeer\n"));
    assert!(backup.contains("ni hao \t你好\tc=4 d=4 t=5\n"));
    assert!(backup.contains("shuo \t说\tc=-7 d=7 t=5\n"));
    assert!(backup.contains("zhong guo \t中国\tc=2 d=2 t=5\n"));

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

#[test]
fn levers_user_dict_iterator_lists_userdb_entries() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-user-dicts");
    let user = root.join("user");
    fs::create_dir_all(user.join("luna_pinyin.userdb"))
        .expect("leveldb-style user dict dir should be created");
    fs::write(user.join("essay.userdb"), "").expect("user dict file should be written");
    fs::write(user.join("legacy.userdb.txt"), "")
        .expect("plain legacy user dict should not match current userdb extension");
    fs::write(user.join("default.yaml"), "").expect("unrelated file should be ignored");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let iterator_init = api
        .user_dict_iterator_init
        .expect("user dict iterator init should be available");
    let iterator_destroy = api
        .user_dict_iterator_destroy
        .expect("user dict iterator destroy should be available");
    let next_user_dict = api
        .next_user_dict
        .expect("next user dict should be available");

    let mut iterator = crate::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 0,
    };
    // SAFETY: iterator points to writable storage.
    assert_eq!(unsafe { iterator_init(&mut iterator) }, TRUE);
    assert!(!iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);

    // SAFETY: iterator was initialized by the levers API.
    let first = unsafe { next_user_dict(&mut iterator) };
    assert!(!first.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(unsafe { CStr::from_ptr(first) }.to_str(), Ok("essay"));
    // SAFETY: iterator remains initialized.
    let second = unsafe { next_user_dict(&mut iterator) };
    assert!(!second.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(
        unsafe { CStr::from_ptr(second) }.to_str(),
        Ok("luna_pinyin")
    );
    // SAFETY: iterator is exhausted but valid.
    assert!(unsafe { next_user_dict(&mut iterator) }.is_null());

    // SAFETY: iterator was initialized by this shim.
    unsafe { iterator_destroy(&mut iterator) };
    assert!(iterator.ptr.is_null());
    assert_eq!(iterator.i, 0);

    // SAFETY: null inputs are explicitly rejected/no-oped.
    assert_eq!(unsafe { iterator_init(std::ptr::null_mut()) }, FALSE);
    assert!(unsafe { next_user_dict(std::ptr::null_mut()) }.is_null());
    unsafe { iterator_destroy(std::ptr::null_mut()) };

    fs::remove_file(user.join("essay.userdb")).expect("user dict file should be removed");
    fs::remove_dir_all(user.join("luna_pinyin.userdb")).expect("user dict dir should be removed");
    let mut empty_iterator = crate::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 7,
    };
    // SAFETY: iterator points to writable storage; no .userdb entries remain.
    assert_eq!(unsafe { iterator_init(&mut empty_iterator) }, FALSE);
    assert!(empty_iterator.ptr.is_null());
    assert_eq!(empty_iterator.i, 7);

    fs::write(user.join("cached.userdb"), "").expect("cached user dict should be written");
    let mut cached_iterator = crate::RimeUserDictIterator {
        ptr: std::ptr::null_mut(),
        i: 0,
    };
    // SAFETY: iterator points to writable storage.
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, TRUE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    fs::remove_file(user.join("cached.userdb")).expect("cached user dict should be removed");
    // SAFETY: librime leaves an existing iterator untouched when a re-scan
    // finds no user dictionaries.
    assert_eq!(unsafe { iterator_init(&mut cached_iterator) }, FALSE);
    assert!(!cached_iterator.ptr.is_null());
    assert_eq!(cached_iterator.i, 0);
    // SAFETY: cached_iterator still owns the previous snapshot.
    let cached = unsafe { next_user_dict(&mut cached_iterator) };
    assert!(!cached.is_null());
    // SAFETY: returned pointer is owned by the iterator and valid until destroy.
    assert_eq!(unsafe { CStr::from_ptr(cached) }.to_str(), Ok("cached"));
    // SAFETY: cached_iterator was initialized by this shim.
    unsafe { iterator_destroy(&mut cached_iterator) };

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn levers_user_dict_file_operations_handle_plain_userdb_files() {
    let _guard = test_guard();
    let root = unique_temp_dir("levers-user-dict-files");
    let user = root.join("user");
    fs::create_dir_all(&user).expect("user dir should be created");
    struct CurrentDirGuard(PathBuf);
    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.0);
        }
    }
    let current_dir_guard =
        CurrentDirGuard(env::current_dir().expect("current dir should be available"));
    env::set_current_dir(&root).expect("test cwd should move under temp root");
    fs::write(
        user.join("luna_pinyin.userdb"),
        "# comment\nni hao\t你好\t1\n\nzhong guo\t中国\t2\n",
    )
    .expect("plain user dict should be written");

    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };

    let levers_name = CString::new("levers").expect("module name should be valid");
    // SAFETY: lookup name is a valid NUL-terminated string.
    let module = unsafe { RimeFindModule(levers_name.as_ptr()) };
    assert!(!module.is_null());
    // SAFETY: built-in module storage is process-lifetime.
    let module = unsafe { &*module };
    let api = module.get_api.expect("levers get_api should be set")().cast::<RimeLeversApi>();
    assert!(!api.is_null());
    // SAFETY: levers get_api returns a process-lifetime RimeLeversApi object.
    let api = unsafe { &*api };
    let backup_user_dict = api
        .backup_user_dict
        .expect("backup user dict should be available");
    let restore_user_dict = api
        .restore_user_dict
        .expect("restore user dict should be available");
    let export_user_dict = api
        .export_user_dict
        .expect("export user dict should be available");
    let import_user_dict = api
        .import_user_dict
        .expect("import user dict should be available");

    let dict_name = CString::new("luna_pinyin").expect("dict name is valid");
    // SAFETY: dict name is a valid NUL-terminated string.
    assert_eq!(unsafe { backup_user_dict(dict_name.as_ptr()) }, TRUE);
    let snapshot = root
        .join("sync")
        .join("unknown")
        .join("luna_pinyin.userdb.txt");
    let snapshot_text = fs::read_to_string(&snapshot).expect("snapshot should be readable");
    assert!(snapshot_text.contains("/db_name\tluna_pinyin\n"));
    assert!(snapshot_text.contains("/db_type\tuserdb\n"));
    assert!(snapshot_text.contains("ni hao \t你好\tc=1 d=1 t=1\n"));
    assert!(snapshot_text.contains("zhong guo \t中国\tc=2 d=2 t=1\n"));

    let export_path = root.join("luna_export.tsv");
    let export_path_c =
        CString::new(export_path.to_string_lossy().as_ref()).expect("path is valid");
    // SAFETY: pointers are valid NUL-terminated strings.
    assert_eq!(
        unsafe { export_user_dict(dict_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    assert_eq!(
        fs::read_to_string(&export_path).expect("export should be readable"),
        "你好\tni hao\t1\n中国\tzhong guo\t2\n"
    );

    fs::write(&export_path, "新\txin\t3\n词\tci\t4\n").expect("import file should be updated");
    let imported_name = CString::new("imported").expect("dict name is valid");
    // SAFETY: pointers are valid NUL-terminated strings.
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), export_path_c.as_ptr()) },
        2
    );
    let imported =
        fs::read_to_string(user.join("imported.userdb")).expect("import should be readable");
    assert!(imported.contains("xin \t新\tc=3 d=3 t=1\n"));
    assert!(imported.contains("ci \t词\tc=4 d=4 t=1\n"));

    let snapshot_c = CString::new(snapshot.to_string_lossy().as_ref()).expect("path is valid");
    fs::remove_file(user.join("luna_pinyin.userdb"))
        .expect("user dict should be removable before restore");
    // SAFETY: snapshot path is a valid NUL-terminated string.
    assert_eq!(unsafe { restore_user_dict(snapshot_c.as_ptr()) }, TRUE);
    assert!(user.join("luna_pinyin.userdb").is_file());

    let missing = CString::new("missing").expect("dict name is valid");
    // SAFETY: null and missing inputs are explicitly rejected.
    assert_eq!(unsafe { backup_user_dict(std::ptr::null()) }, FALSE);
    assert_eq!(unsafe { backup_user_dict(missing.as_ptr()) }, FALSE);
    assert_eq!(unsafe { restore_user_dict(std::ptr::null()) }, FALSE);
    assert_eq!(
        unsafe { export_user_dict(std::ptr::null(), export_path_c.as_ptr()) },
        -1
    );
    assert_eq!(
        unsafe { import_user_dict(imported_name.as_ptr(), std::ptr::null()) },
        -1
    );

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    drop(current_dir_guard);
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
