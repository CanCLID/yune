use super::*;
use crate::remaining_gear_deferrals_snapshot;

#[test]
fn dictionary_data_prefers_fresh_compiled_payloads_and_matches_source_order() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("dictionary-data-compiled");
    let fixture = DictionaryDataFixture::new(&root, true);
    fixture.setup_runtime();

    let source_candidates = fixture.candidates_for_schema("source_luna", "ba");
    let compiled_candidates = fixture.candidates_for_schema("luna", "ba");

    assert_eq!(compiled_candidates[..2], source_candidates[..2]);
    assert_eq!(compiled_candidates[..2], [("八".to_owned(), "ba".to_owned()), ("爸".to_owned(), "ba".to_owned())]);
    assert!(
        remaining_gear_deferrals_snapshot(fixture.last_session_id())
            .expect("session should exist")
            .is_empty()
    );
    fixture.cleanup();
}

#[test]
fn dictionary_data_falls_back_to_source_when_compiled_is_missing_or_corrupt() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("dictionary-data-fallback");
    let fixture = DictionaryDataFixture::new(&root, true);
    fixture.setup_runtime();
    fs::remove_file(fixture.shared.join("luna.table.bin")).expect("compiled table removed");
    assert_eq!(
        fixture.candidates_for_schema("luna", "ba")[..2],
        [("八".to_owned(), "ba".to_owned()), ("爸".to_owned(), "ba".to_owned())]
    );

    fs::write(fixture.shared.join("luna.table.bin"), [0xff, 0x00]).expect("corrupt table written");
    fs::write(fixture.shared.join("luna.prism.bin"), [0xff, 0x00]).expect("corrupt prism written");
    fs::write(fixture.shared.join("luna.reverse.bin"), [0xff, 0x00]).expect("corrupt reverse written");
    assert_eq!(
        fixture.candidates_for_schema("luna", "ba")[..2],
        [("八".to_owned(), "ba".to_owned()), ("爸".to_owned(), "ba".to_owned())]
    );
    let deferrals = remaining_gear_deferrals_snapshot(fixture.last_session_id())
        .expect("session should exist");
    assert!(deferrals
        .iter()
        .any(|deferral| deferral.gear == "dictionary_source_fallback"));
    fixture.cleanup();
}

#[test]
fn dictionary_data_records_no_usable_path_without_empty_success() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("dictionary-data-no-usable");
    let fixture = DictionaryDataFixture::new(&root, true);
    fixture.setup_runtime();
    fs::write(fixture.shared.join("luna.table.bin"), [0xff, 0x00]).expect("corrupt table written");
    fs::write(fixture.shared.join("luna.prism.bin"), [0xff, 0x00]).expect("corrupt prism written");
    fs::write(fixture.shared.join("luna.reverse.bin"), [0xff, 0x00]).expect("corrupt reverse written");
    fs::remove_file(fixture.shared.join("luna.dict.yaml")).expect("source removed");

    let candidates = fixture.candidates_for_schema("luna", "ba");
    assert_eq!(candidates, [("ba".to_owned(), "echo".to_owned())]);
    let deferrals = remaining_gear_deferrals_snapshot(fixture.last_session_id())
        .expect("session should exist");
    assert!(deferrals.iter().any(|deferral| {
        deferral.gear == "dictionary_load" && deferral.current_yune_behavior.contains("NoUsablePath")
    }));
    fixture.cleanup();
}

#[test]
fn dictionary_data_rejects_unsafe_resource_ids_before_lookup() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    for dictionary_id in ["../evil", "/absolute", "a/b", "a\\b", "C:evil", "evil\0id"] {
        let root = unique_temp_dir("dictionary-data-resource-id");
        let fixture = DictionaryDataFixture::new(&root, false);
        fixture.write_schema("luna", dictionary_id);
        fixture.setup_runtime();
        let candidates = fixture.candidates_for_schema("luna", "ba");
        assert_eq!(candidates, [("ba".to_owned(), "echo".to_owned())]);
        let deferrals = remaining_gear_deferrals_snapshot(fixture.last_session_id())
            .expect("session should exist");
        assert!(deferrals.iter().any(|deferral| {
            deferral.gear == "dictionary_load" && deferral.current_yune_behavior.contains("InvalidResourceId")
        }));
        fixture.cleanup();
    }
}

#[test]
fn dictionary_data_malformed_payloads_are_schema_visible_fallbacks() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("too-short", vec![0]),
        ("bad-version", bad_version_table_fixture()),
        ("out-of-bounds", out_of_bounds_table_fixture()),
        ("huge-count", huge_count_table_fixture()),
        ("invalid-utf8", invalid_utf8_table_fixture()),
        ("missing-section", missing_section_table_fixture()),
    ];

    for (case, table_bytes) in cases {
        let root = unique_temp_dir(&format!("dictionary-data-malformed-{case}"));
        let fixture = DictionaryDataFixture::new(&root, true);
        fixture.setup_runtime();
        fs::write(fixture.shared.join("luna.table.bin"), table_bytes).expect("malformed table written");
        assert_eq!(
            fixture.candidates_for_schema("luna", "ba")[..2],
            [("八".to_owned(), "ba".to_owned()), ("爸".to_owned(), "ba".to_owned())]
        );
        fixture.cleanup();
    }
}

struct DictionaryDataFixture<'a> {
    root: &'a std::path::Path,
    shared: std::path::PathBuf,
    user: std::path::PathBuf,
    staging: std::path::PathBuf,
    last_session: std::cell::Cell<super::super::RimeSessionId>,
}

impl<'a> DictionaryDataFixture<'a> {
    fn new(root: &'a std::path::Path, full: bool) -> Self {
        let shared = root.join("shared");
        let user = root.join("user");
        let staging = user.join("build");
        fs::create_dir_all(&shared).expect("shared dir should be created");
        fs::create_dir_all(&staging).expect("staging dir should be created");
        let fixture = Self {
            root,
            shared,
            user,
            staging,
            last_session: std::cell::Cell::new(0),
        };
        if full {
            fixture.write_schema("luna", "luna");
            fixture.write_schema("source_luna", "luna");
            fixture.write_source_dictionary();
            fixture.write_compiled_artifacts();
        }
        fixture
    }

    fn write_schema(&self, schema_id: &str, dictionary_id: &str) {
        fs::write(
            self.staging.join(format!("{schema_id}.schema.yaml")),
            format!(
                "\
schema:\n  schema_id: {schema_id}\n  name: {schema_id}\nengine:\n  translators:\n    - table_translator\n    - echo_translator\ntranslator:\n  dictionary: \"{}\"\n",
                dictionary_id.replace('\\', "\\\\").replace('\0', "\\0")
            ),
        )
        .expect("schema should be written");
    }

    fn write_source_dictionary(&self) {
        fs::write(
            self.shared.join("luna.dict.yaml"),
            "\
---\nname: luna\nversion: '0.1'\nsort: by_weight\n...\n\n八\tba\t2\n爸\tba\t1\n",
        )
        .expect("source dictionary should be written");
    }

    fn write_compiled_artifacts(&self) {
        let source = fs::read_to_string(self.shared.join("luna.dict.yaml"))
            .expect("source dictionary should be readable");
        fs::write(
            self.shared.join("luna.table.bin"),
            compiled_table_fixture(yune_core::rime_dict_source_checksum(0, [source.as_bytes()], None)),
        )
        .expect("compiled table should be written");
        fs::write(self.shared.join("luna.prism.bin"), compiled_prism_fixture())
            .expect("compiled prism should be written");
        fs::write(self.shared.join("luna.reverse.bin"), compiled_reverse_fixture())
            .expect("compiled reverse should be written");
    }

    fn setup_runtime(&self) {
        let shared_c = CString::new(self.shared.to_string_lossy().as_ref()).expect("path is valid");
        let user_c = CString::new(self.user.to_string_lossy().as_ref()).expect("path is valid");
        let mut traits = empty_traits();
        traits.shared_data_dir = shared_c.as_ptr();
        traits.user_data_dir = user_c.as_ptr();
        unsafe { RimeSetup(&traits) };
    }

    fn candidates_for_schema(&self, schema_id: &str, input: &str) -> Vec<(String, String)> {
        let session_id = RimeCreateSession();
        self.last_session.set(session_id);
        let schema_id = CString::new(schema_id).expect("schema id should be valid");
        assert_eq!(unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) }, TRUE);
        for ch in input.chars() {
            assert_eq!(RimeProcessKey(session_id, ch as c_int, 0), TRUE);
        }
        let mut context = empty_context();
        assert_eq!(unsafe { RimeGetContext(session_id, &mut context) }, TRUE);
        let candidates = unsafe {
            std::slice::from_raw_parts(context.menu.candidates, context.menu.num_candidates as usize)
        };
        let result = candidates
            .iter()
            .map(|candidate| {
                let text = unsafe { CStr::from_ptr(candidate.text) }
                    .to_string_lossy()
                    .into_owned();
                let comment = if candidate.comment.is_null() {
                    String::new()
                } else {
                    unsafe { CStr::from_ptr(candidate.comment) }
                        .to_string_lossy()
                        .into_owned()
                };
                (text, comment)
            })
            .collect::<Vec<_>>();
        assert_eq!(unsafe { RimeFreeContext(&mut context) }, TRUE);
        result
    }

    fn last_session_id(&self) -> super::super::RimeSessionId {
        self.last_session.get()
    }

    fn cleanup(&self) {
        let reset_traits = empty_traits();
        unsafe { RimeSetup(&reset_traits) };
        let _ = fs::remove_dir_all(self.root);
    }
}

fn compiled_table_fixture(checksum: u32) -> Vec<u8> {
    let mut bytes = vec![0; 68];
    put_c_string(&mut bytes, 0, b"Rime::Table/4.0");
    put_u32_le(&mut bytes, 32, checksum);
    put_u32_le(&mut bytes, 36, 1);
    put_u32_le(&mut bytes, 40, 2);
    let syllabary_offset = bytes.len();
    bytes.resize(syllabary_offset + 8, 0);
    put_u32_le(&mut bytes, syllabary_offset, 1);
    let code_offset = append_c_string(&mut bytes, "ba");
    put_offset(&mut bytes, syllabary_offset + 4, code_offset);
    let index_offset = bytes.len();
    bytes.resize(index_offset + 16, 0);
    put_u32_le(&mut bytes, index_offset, 1);
    put_u32_le(&mut bytes, index_offset + 4, 2);
    let entries_offset = bytes.len();
    bytes.resize(entries_offset + 16, 0);
    let ba_offset = append_c_string(&mut bytes, "八");
    let ba2_offset = append_c_string(&mut bytes, "爸");
    put_offset(&mut bytes, entries_offset, ba_offset);
    put_f32_le(&mut bytes, entries_offset + 4, 2.0);
    put_offset(&mut bytes, entries_offset + 8, ba2_offset);
    put_f32_le(&mut bytes, entries_offset + 12, 1.0);
    put_offset(&mut bytes, index_offset + 8, entries_offset);
    put_offset(&mut bytes, 44, syllabary_offset);
    put_offset(&mut bytes, 48, index_offset);
    bytes
}

fn compiled_prism_fixture() -> Vec<u8> {
    let mut bytes = vec![0; 320];
    put_c_string(&mut bytes, 0, b"Rime::Prism/4.0");
    let spelling_map_offset = bytes.len();
    bytes.resize(spelling_map_offset + 4, 0);
    put_u32_le(&mut bytes, spelling_map_offset, 0);
    put_offset(&mut bytes, 56, spelling_map_offset);
    bytes
}

fn compiled_reverse_fixture() -> Vec<u8> {
    let mut bytes = vec![0; 64];
    put_c_string(&mut bytes, 0, b"Rime::Reverse/4.0");
    bytes.extend_from_slice(b"YUNE-REVERSE\0");
    put_u32_le_extend(&mut bytes, 0);
    bytes
}

fn bad_version_table_fixture() -> Vec<u8> {
    let mut bytes = compiled_table_fixture(0);
    put_c_string(&mut bytes, 0, b"Rime::Table/3.0");
    bytes
}

fn out_of_bounds_table_fixture() -> Vec<u8> {
    let mut bytes = compiled_table_fixture(0);
    put_i32_le(&mut bytes, 44, i32::MAX);
    bytes
}

fn huge_count_table_fixture() -> Vec<u8> {
    let mut bytes = compiled_table_fixture(0);
    put_u32_le(&mut bytes, 79, u32::MAX);
    bytes
}

fn invalid_utf8_table_fixture() -> Vec<u8> {
    let mut bytes = compiled_table_fixture(0);
    let last = bytes.len() - 1;
    bytes[last - 1] = 0xff;
    bytes
}

fn missing_section_table_fixture() -> Vec<u8> {
    let mut bytes = compiled_table_fixture(0);
    put_i32_le(&mut bytes, 44, 0);
    bytes
}

fn put_c_string(bytes: &mut [u8], offset: usize, value: &[u8]) {
    bytes[offset..offset + value.len()].copy_from_slice(value);
}

fn put_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_i32_le(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_f32_le(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_bits().to_le_bytes());
}

fn put_offset(bytes: &mut [u8], field_offset: usize, target: usize) {
    let raw = i32::try_from(target as isize - field_offset as isize)
        .expect("fixture offset should fit i32");
    put_i32_le(bytes, field_offset, raw);
}

fn append_c_string(bytes: &mut Vec<u8>, value: &str) -> usize {
    let offset = bytes.len();
    bytes.extend_from_slice(value.as_bytes());
    bytes.push(0);
    offset
}

fn put_u32_le_extend(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
