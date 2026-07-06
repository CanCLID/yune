use std::{
    ffi::{c_void, CStr, CString},
    fs, mem,
    os::raw::c_int,
    path::{Path, PathBuf},
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use yune_core::{build_reverse_bin, build_table_bin, TableDictionary};
use yune_rime_api::{
    RimeCleanupAllSessions, RimeComposition, RimeConfig, RimeConfigClose, RimeConfigGetString,
    RimeContext, RimeCreateSession, RimeDestroySession, RimeFreeContext, RimeGetContext,
    RimeInitialize, RimeMenu, RimeProcessKey, RimeSchemaOpen, RimeSelectSchema, RimeSessionId,
    RimeSetup, RimeStartMaintenance, RimeTraits, FALSE, TRUE,
};

const WINDOWS_BOUNDARY_NGOHAIG: &str = include_str!(
    "../../yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-windows-boundary-ngohaig.json"
);

fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
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

fn empty_context() -> RimeContext {
    RimeContext {
        data_size: mem::size_of::<RimeContext>() as i32,
        composition: RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        },
        menu: RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        },
        commit_text_preview: ptr::null_mut(),
        select_labels: ptr::null_mut(),
    }
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after UNIX epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("yune-{name}-{}-{nanos}", std::process::id()))
}

fn windows_boundary_fixture() -> Value {
    serde_json::from_str(WINDOWS_BOUNDARY_NGOHAIG)
        .expect("Windows boundary fixture should be valid JSON")
}

fn windows_boundary_case(fixture: &Value) -> &Value {
    fixture["cases"]
        .as_array()
        .expect("Windows boundary cases should be an array")
        .first()
        .expect("Windows boundary fixture should capture ngohaig")
}

fn selected_candidate_comment(case: &Value, index: usize) -> &str {
    case["selected_candidates"]
        .as_array()
        .expect("selected candidates should be an array")[index]["comment"]
        .as_str()
        .expect("candidate comment should be a string")
}

fn code_first_lookup_yaml_from_oracle_comments(name: &str, comments: &[&str]) -> String {
    let rows = comments
        .iter()
        .flat_map(|comment| comment.split('\u{000c}').skip(1))
        .flat_map(|records| records.split('\r'))
        .filter_map(|record| {
            record
                .strip_prefix("1,")
                .or_else(|| record.strip_prefix("0,"))
        })
        .filter_map(|fields| {
            let (text, payload) = fields.split_once(',')?;
            Some(format!("{payload}\t{text}"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("---\nname: {name}\nversion: '0.1'\nsort: original\n...\n\n{rows}\n")
}

fn lookup_dictionary_from_oracle_comments(case: &Value) -> TableDictionary {
    TableDictionary::parse_typeduck_lookup_dict_yaml(&code_first_lookup_yaml_from_oracle_comments(
        "jyut6ping3_scolar",
        &[
            selected_candidate_comment(case, 0),
            selected_candidate_comment(case, 2),
            selected_candidate_comment(case, 3),
        ],
    ))
    .expect("Windows boundary oracle comments should parse into lookup rows")
}

fn write_windows_boundary_runtime(root: &Path) -> (PathBuf, PathBuf) {
    let fixture = windows_boundary_fixture();
    let case = windows_boundary_case(&fixture);
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("jyut6ping3.schema.yaml"),
        "\
schema:
  schema_id: jyut6ping3
  name: jyut6ping3
engine:
  translators:
    - table_translator
  filters:
    - dictionary_lookup_filter
translator:
  dictionary: jyut6ping3
  enable_completion: true
  enable_sentence: true
  comment_format:
    - xform/^/\\f/
speller:
  algebra:
    - derive/\\d//
dictionary_lookup_filter:
  dictionary: jyut6ping3_scolar
",
    )
    .expect("schema config should be written");
    fs::write(
        shared.join("jyut6ping3.dict.yaml"),
        format!(
            "---\n\
name: jyut6ping3\n\
version: '0.1'\n\
sort: original\n\
...\n\
\n\
{}\tngo5hai6\n\
{}\tgo3\n\
{}\tngo5hai2\n\
{}\tngo5\n\
{}\tngo4\n\
{}\to1\n",
            "\u{6211}\u{4fc2}", "\u{500b}", "\u{6211}\u{55ba}", "\u{6211}", "\u{4fc4}", "\u{67ef}"
        ),
    )
    .expect("target dictionary should be written");
    fs::write(
        shared.join("jyut6ping3_scolar.dict.yaml"),
        code_first_lookup_yaml_from_oracle_comments(
            "jyut6ping3_scolar",
            &[
                selected_candidate_comment(case, 0),
                selected_candidate_comment(case, 2),
                selected_candidate_comment(case, 3),
            ],
        ),
    )
    .expect("lookup dictionary should be written");
    (shared, user)
}

fn write_windows_boundary_runtime_with_compiled_lookup(root: &Path) -> (PathBuf, PathBuf) {
    let fixture = windows_boundary_fixture();
    let case = windows_boundary_case(&fixture);
    let (shared, user) = write_windows_boundary_runtime(root);
    let lookup_source = shared.join("jyut6ping3_scolar.dict.yaml");
    fs::remove_file(&lookup_source)
        .expect("lookup source should be removed for compiled-path test");
    let lookup_dictionary = lookup_dictionary_from_oracle_comments(case);
    let staging = user.join("build");
    fs::write(
        staging.join("jyut6ping3_scolar.table.bin"),
        build_table_bin(&lookup_dictionary, 0),
    )
    .expect("compiled lookup table should be written");
    fs::write(
        staging.join("jyut6ping3_scolar.reverse.bin"),
        build_reverse_bin(&lookup_dictionary, 0),
    )
    .expect("compiled lookup reverse should be written");
    (shared, user)
}

fn setup_runtime(shared: &Path, user: &Path, initialize: bool) {
    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("shared path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("user path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    if initialize {
        // SAFETY: traits points to valid storage and strings live for this call.
        unsafe { RimeInitialize(&traits) };
    } else {
        // SAFETY: traits points to valid storage and strings live for this call.
        unsafe { RimeSetup(&traits) };
    }
}

fn select_schema_and_process(session_id: RimeSessionId, input: &str) {
    let schema_id = CString::new("jyut6ping3").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );
    for ch in input.chars() {
        assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
    }
}

fn candidate_rows(session_id: RimeSessionId) -> Vec<(String, String)> {
    let mut context = empty_context();
    // SAFETY: context points to writable storage initialized with positive data_size.
    assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
    let candidates = unsafe {
        std::slice::from_raw_parts(
            context.menu.candidates,
            context.menu.num_candidates as usize,
        )
    };
    let rows = candidates
        .iter()
        .map(|candidate| {
            let text = unsafe { CStr::from_ptr(candidate.text) }
                .to_str()
                .expect("candidate text should be valid UTF-8")
                .to_owned();
            let comment = if candidate.comment.is_null() {
                String::new()
            } else {
                unsafe { CStr::from_ptr(candidate.comment) }
                    .to_str()
                    .expect("candidate comment should be valid UTF-8")
                    .to_owned()
            };
            (text, comment)
        })
        .collect::<Vec<_>>();
    // SAFETY: nested pointers were allocated by `RimeGetContext` above.
    assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
    rows
}

#[test]
fn yune_abi_jyut6ping3_ngohaig_comments_match_v112() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("typeduck-windows-boundary");
    let (shared, user) = write_windows_boundary_runtime(&root);
    setup_runtime(&shared, &user, true);

    let session_id = RimeCreateSession();
    select_schema_and_process(session_id, "ngohaig");
    let rows = candidate_rows(session_id);
    let fixture = windows_boundary_fixture();
    let case = windows_boundary_case(&fixture);

    assert!(
        rows.len() >= 4,
        "ABI path should produce the first four Windows boundary candidates: {rows:?}"
    );
    for (index, (text, comment)) in rows.iter().take(4).enumerate() {
        assert_eq!(
            text,
            case["selected_candidates"][index]["text"]
                .as_str()
                .expect("candidate text should be a string")
        );
        assert_eq!(comment, selected_candidate_comment(case, index));
        assert!(
            comment.as_bytes().starts_with(&[0x0c, 0x0d, b'1', b',']),
            "candidate {index} raw ABI comment should start with TypeDuck rich-comment bytes"
        );
        assert!(
            !comment.starts_with("\\f"),
            "candidate {index} raw ABI comment should not expose a literal backslash-f"
        );
    }

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn yune_abi_compiled_lookup_jyut6ping3_ngohaig_comments_match_v112() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("typeduck-windows-boundary-compiled");
    let (shared, user) = write_windows_boundary_runtime_with_compiled_lookup(&root);
    setup_runtime(&shared, &user, true);

    let session_id = RimeCreateSession();
    select_schema_and_process(session_id, "ngohaig");
    let rows = candidate_rows(session_id);
    let fixture = windows_boundary_fixture();
    let case = windows_boundary_case(&fixture);

    assert!(
        rows.len() >= 4,
        "compiled lookup path should produce the first four Windows boundary candidates: {rows:?}"
    );
    for (index, (text, comment)) in rows.iter().take(4).enumerate() {
        assert_eq!(
            text,
            case["selected_candidates"][index]["text"]
                .as_str()
                .expect("candidate text should be a string")
        );
        assert_eq!(comment, selected_candidate_comment(case, index));
    }

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn yune_abi_repeated_jyut6ping3_session_lifecycle_stays_responsive() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("typeduck-windows-boundary-lifecycle");
    let (shared, user) = write_windows_boundary_runtime(&root);
    setup_runtime(&shared, &user, true);
    let _maintenance_result = RimeStartMaintenance(FALSE);

    for _ in 0..2 {
        let session_id = RimeCreateSession();
        assert_ne!(session_id, 0);
        select_schema_and_process(session_id, "ngohaig");
        let rows = candidate_rows(session_id);
        assert!(
            rows.iter()
                .any(|(text, _)| text == "\u{6211}\u{4fc2}\u{500b}"),
            "session should stay responsive through select/process/get-context"
        );
        assert_eq!(RimeDestroySession(session_id), TRUE);
    }

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}

#[test]
fn yune_abi_schema_open_tolerates_uninitialized_config_slot_like_typeduck_windows() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("typeduck-windows-boundary-schema-open");
    let (shared, user) = write_windows_boundary_runtime(&root);
    setup_runtime(&shared, &user, true);

    let schema_id = CString::new("jyut6ping3").expect("schema id should be valid");
    let schema_name_key = CString::new("schema/name").expect("config key should be valid");
    let foreign = Box::into_raw(Box::new(0xC0DEC0DEusize)).cast::<c_void>();
    let mut config = RimeConfig { ptr: foreign };
    let mut output = vec![0 as std::os::raw::c_char; 64];

    assert_eq!(
        unsafe { RimeSchemaOpen(schema_id.as_ptr(), &mut config) },
        TRUE
    );
    assert_ne!(
        config.ptr, foreign,
        "RimeSchemaOpen should overwrite a foreign caller slot without freeing it"
    );
    assert_eq!(
        unsafe {
            RimeConfigGetString(
                &mut config,
                schema_name_key.as_ptr(),
                output.as_mut_ptr(),
                output.len(),
            )
        },
        TRUE
    );
    assert_eq!(
        unsafe { CStr::from_ptr(output.as_ptr()) }.to_str(),
        Ok("jyut6ping3")
    );
    assert_eq!(unsafe { RimeConfigClose(&mut config) }, TRUE);
    unsafe {
        drop(Box::from_raw(foreign.cast::<usize>()));
    }

    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
