// Owner: m21-plan-typeduck-web-product-comparison.md M21-GAP-01. This ignored
// probe captures the Yune real-browser-assets candidate surface with engine
// source labels; it does not add public ABI or TypeDuck-Web exports.
#[test]
#[ignore = "diagnostic: writes M21-GAP-01 Yune real-assets candidate evidence"]
fn m21_gap_01_sentence_composition_probe() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("m21-gap-01-sentence-composition");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    m21_gap_01_write_real_browser_assets(&shared, &staging);

    let shared_c = CString::new(shared.to_string_lossy().as_ref()).expect("path is valid");
    let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path is valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared_c.as_ptr();
    traits.user_data_dir = user_c.as_ptr();
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeSetup(&traits) };
    // SAFETY: traits points to valid storage and strings live for the call.
    unsafe { RimeInitialize(&traits) };

    let session_id = RimeCreateSession();
    let schema_id = CString::new("jyut6ping3_mobile").expect("schema id should be valid");
    // SAFETY: schema id is a valid NUL-terminated string.
    assert_eq!(
        unsafe { RimeSelectSchema(session_id, schema_id.as_ptr()) },
        TRUE
    );

    let inputs = [
        "loengnincin",
        "leoicijyu",
        "ngohaigo",
        "loengjathau",
        "geijatcin",
        "gamjatheoi",
    ];
    let cases = inputs
        .iter()
        .map(|input| m21_gap_01_capture_case(session_id, input))
        .collect::<Vec<_>>();
    let evidence = serde_json::json!({
        "capture": {
            "scenario": "M21-GAP-01 Yune real-assets sentence-composition probe",
            "schema": "jyut6ping3_mobile",
            "candidate_source": "session_candidates_snapshot after RimeGetContext on the real RimeApi path",
            "asset_root": "apps/yune-web/source/public/schema",
            "dictionary": "jyut6ping3.dict.yaml",
            "lookup_dictionary": "jyut6ping3_scolar.dict.yaml",
            "pinned_yune_commit": std::env::var("YUNE_M21_GAP_01_COMMIT").unwrap_or_else(|_| "unknown".to_owned()),
            "notes": [
                "fallback_gate=fired_returned_sentence when a sentence source row is present under the default sentence-only-as-fallback path",
                "fallback_gate=not_fired_base_candidates_nonempty when non-sentence candidates prevent sentence fallback",
                "correction rows are not separately encoded in CandidateSource; this corpus is not a typo/correction probe"
            ]
        },
        "inputs": inputs,
        "cases": cases,
    });
    let output =
        serde_json::to_string_pretty(&evidence).expect("M21 evidence should serialize as JSON");
    if let Ok(output_path) = std::env::var("YUNE_M21_GAP_01_OUTPUT") {
        fs::write(&output_path, output)
            .unwrap_or_else(|error| panic!("failed to write {output_path}: {error}"));
    } else {
        println!("M21_GAP_01_YUNE_JSON_BEGIN");
        println!("{output}");
        println!("M21_GAP_01_YUNE_JSON_END");
    }

    assert_eq!(RimeDestroySession(session_id), TRUE);
    let reset_traits = empty_traits();
    // SAFETY: reset traits points to valid storage.
    unsafe { RimeSetup(&reset_traits) };
    fs::remove_dir_all(root).expect("temp dirs should be removed");
}
