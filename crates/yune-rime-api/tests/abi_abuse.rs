use std::{
    ffi::{CStr, CString},
    fs, mem, ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    thread,
};

use proptest::prelude::*;
use yune_rime_api::{
    rime_get_api, rime_levers_get_api, RimeCandidateListBegin, RimeCandidateListEnd,
    RimeCandidateListIterator, RimeCandidateListNext, RimeCleanupAllSessions, RimeConfigClose,
    RimeConfigGetBool, RimeConfigGetCString, RimeConfigGetInt, RimeConfigInit,
    RimeConfigLoadString, RimeCreateSession, RimeDestroySession, RimeFinalize, RimeFindSession,
    RimeFreeContext, RimeGetContext, RimeGetInput, RimeGetKeycodeByName, RimeGetModifierByName,
    RimeGetSharedDataDir, RimeInitialize, RimeJoinMaintenanceThread, RimeProcessKey, RimeRunTask,
    RimeSchemaOpen, RimeSelectSchema, RimeSessionId, RimeTraits, FALSE, TRUE,
};

#[test]
fn abi_guard_inventory_matches_no_mangle_exports() {
    let _guard = test_guard();
    let root = repo_root();
    let src = root.join("crates/yune-rime-api/src");
    let mut exports = Vec::new();
    collect_rs_files(&src, &mut exports);

    let mut unguarded = Vec::new();
    let mut no_mangle_count = 0usize;
    for file in exports {
        let text = fs::read_to_string(&file).expect("source file should be readable");
        let lines = text.lines().collect::<Vec<_>>();
        let mut index = 0usize;
        while index < lines.len() {
            if lines[index].trim() != "#[no_mangle]" {
                index += 1;
                continue;
            }
            no_mangle_count += 1;
            let start = index;
            let mut body_start = index + 1;
            while body_start < lines.len() && !lines[body_start].contains('{') {
                body_start += 1;
            }
            let mut depth = brace_delta(lines[body_start]);
            let mut end = body_start + 1;
            while end < lines.len() {
                depth += brace_delta(lines[end]);
                if depth == 0 {
                    break;
                }
                end += 1;
            }
            let body = lines[start..=end].join("\n");
            if !body.contains("crate::ffi_guard::guard") {
                unguarded.push(format!(
                    "{}:{}",
                    file.strip_prefix(&root).unwrap_or(&file).display(),
                    start + 1
                ));
            }
            index = end + 1;
        }
    }

    assert_eq!(
        no_mangle_count,
        phase0_ledger_export_count(&root),
        "source export count should stay in lockstep with the Phase 0 ABI ledger"
    );
    assert!(
        unguarded.is_empty(),
        "all no_mangle exports must use ffi_guard: {unguarded:?}"
    );
}

#[test]
fn abi_null_and_out_of_order_calls_return_failure_without_poisoning_followup() {
    let _guard = test_guard();
    unsafe { RimeInitialize(&empty_traits()) };

    assert_eq!(
        unsafe { RimeSchemaOpen(ptr::null(), ptr::null_mut()) },
        FALSE
    );
    let api = unsafe { &*rime_get_api() };
    assert_eq!(
        unsafe {
            (api.user_config_open.expect("user_config_open slot"))(ptr::null(), ptr::null_mut())
        },
        FALSE
    );
    assert_eq!(unsafe { RimeConfigInit(ptr::null_mut()) }, FALSE);
    assert_eq!(
        unsafe { RimeConfigLoadString(ptr::null_mut(), ptr::null()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigGetBool(ptr::null_mut(), ptr::null(), ptr::null_mut()) },
        FALSE
    );
    assert_eq!(
        unsafe { RimeConfigGetInt(ptr::null_mut(), ptr::null(), ptr::null_mut()) },
        FALSE
    );
    assert!(unsafe { RimeConfigGetCString(ptr::null_mut(), ptr::null()) }.is_null());
    assert_eq!(unsafe { RimeConfigClose(ptr::null_mut()) }, FALSE);

    assert_eq!(RimeDestroySession(usize::MAX), FALSE);
    assert_eq!(RimeFindSession(usize::MAX), FALSE);
    assert_eq!(RimeProcessKey(usize::MAX, i32::MIN, i32::MAX), FALSE);
    assert_eq!(unsafe { RimeSelectSchema(usize::MAX, ptr::null()) }, FALSE);
    assert_eq!(
        unsafe { RimeGetContext(usize::MAX, ptr::null_mut()) },
        FALSE
    );
    assert_eq!(unsafe { RimeFreeContext(ptr::null_mut()) }, FALSE);
    assert!(RimeGetInput(usize::MAX).is_null());

    let mut iterator = RimeCandidateListIterator {
        ptr: ptr::null_mut(),
        index: 99,
        candidate: yune_rime_api::RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        },
    };
    assert_eq!(
        unsafe { RimeCandidateListBegin(usize::MAX, &mut iterator) },
        FALSE
    );
    assert_eq!(unsafe { RimeCandidateListNext(ptr::null_mut()) }, FALSE);
    unsafe { RimeCandidateListEnd(ptr::null_mut()) };

    assert!(!rime_get_api().is_null());
    assert!(!rime_levers_get_api().is_null());

    let session = RimeCreateSession();
    assert_ne!(session, 0);
    assert_eq!(RimeFindSession(session), TRUE);
    assert_eq!(RimeDestroySession(session), TRUE);
    RimeFinalize();
}

#[test]
fn abi_cross_thread_session_lookup_is_tolerated() {
    let _guard = test_guard();
    unsafe { RimeInitialize(&empty_traits()) };
    let session = RimeCreateSession();
    assert_ne!(session, 0);
    let handle = thread::spawn(move || {
        assert_eq!(RimeFindSession(session), TRUE);
        let _ = RimeProcessKey(session, 'n' as i32, 0);
    });
    handle
        .join()
        .expect("cross-thread ABI calls should not panic");
    assert_eq!(RimeDestroySession(session), TRUE);
    RimeFinalize();
}

proptest! {
    #[test]
    fn abi_random_logical_strings_do_not_panic(input in "[A-Za-z0-9_./\\\\:-]{0,64}") {
        let _guard = test_guard();
        unsafe { RimeInitialize(&empty_traits()) };
        if let Ok(c_input) = CString::new(input) {
            let _ = RimeRunTask(c_input.as_ptr());
            let _ = unsafe { RimeGetKeycodeByName(c_input.as_ptr()) };
            let _ = unsafe { RimeGetModifierByName(c_input.as_ptr()) };
            let _ = unsafe { RimeSelectSchema(usize::MAX as RimeSessionId, c_input.as_ptr()) };
        }
        RimeFinalize();
    }
}

#[test]
fn abi_followup_runtime_path_call_survives_abuse_suite() {
    let _guard = test_guard();
    unsafe { RimeInitialize(&empty_traits()) };
    let shared = RimeGetSharedDataDir();
    assert!(!shared.is_null());
    assert!(!unsafe { CStr::from_ptr(shared) }
        .to_string_lossy()
        .is_empty());
    RimeJoinMaintenanceThread();
    RimeCleanupAllSessions();
    RimeFinalize();
}

fn empty_traits() -> RimeTraits {
    RimeTraits {
        data_size: mem::size_of::<RimeTraits>() as i32,
        shared_data_dir: ptr::null(),
        user_data_dir: ptr::null(),
        distribution_name: ptr::null(),
        distribution_code_name: ptr::null(),
        distribution_version: ptr::null(),
        app_name: ptr::null(),
        modules: ptr::null(),
        min_log_level: 0,
        log_dir: ptr::null(),
        prebuilt_data_dir: ptr::null(),
        staging_dir: ptr::null(),
    }
}

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate should live under repo/crates/yune-rime-api")
        .to_path_buf()
}

fn collect_rs_files(dir: &std::path::Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).expect("source dir should be readable") {
        let entry = entry.expect("source entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, output);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            output.push(path);
        }
    }
}

fn brace_delta(line: &str) -> i32 {
    line.matches('{').count() as i32 - line.matches('}').count() as i32
}

fn phase0_ledger_export_count(root: &std::path::Path) -> usize {
    let ledger = root.join(
        "docs/reports/evidence/m56-productization-hardening/phase-0-inventory/abi-entry-ledger.csv",
    );
    fs::read_to_string(ledger)
        .expect("Phase 0 ABI ledger should be readable")
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .count()
}
