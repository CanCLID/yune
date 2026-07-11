use super::*;

use yune_core::{encode_octagram_key, DartsDoubleArray};

const TYPEDUCK_M28: &str = include_str!(
    "../../../yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json"
);
const CANONICAL_JYUTPING_BEING: &str = include_str!(
    "../../../yune-core/tests/fixtures/upstream-jyutping/canonical-rime-cantonese/jyutping-m59-being-whole-input.json"
);
const CANGJIE_COMPOSITION: &str =
    include_str!("../../../yune-core/tests/fixtures/upstream-1.17.0/cangjie5-composition.json");
const LUNA_COMPOSITION: &str = include_str!(
    "../../../yune-core/tests/fixtures/upstream-1.17.0/m59-luna-leading-single-composition.json"
);
const DOUBLE_PINYIN_WHOLE_INPUT: &str = include_str!(
    "../../../yune-core/tests/fixtures/upstream-1.17.0/double-pinyin-m59-whole-input.json"
);
const BOPOMOFO_WHOLE_INPUT: &str =
    include_str!("../../../yune-core/tests/fixtures/upstream-1.17.0/bopomofo-m59-whole-input.json");
const CORRECTION_SPELLING_ORACLE: &str =
    include_str!("../../../yune-core/tests/fixtures/upstream-1.17.0/m59-correction-spelling.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Assets {
    Tracked,
    CanonicalJyutping,
    DoublePinyin,
    Bopomofo,
}

#[derive(Clone, Copy, Debug)]
struct SelectionStep {
    target: &'static str,
    remainder: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct ReachabilityCase {
    label: &'static str,
    schema_id: &'static str,
    dictionary_id: &'static str,
    prism_id: &'static str,
    input: &'static str,
    expected_final: &'static str,
    steps: &'static [SelectionStep],
    assets: Assets,
}

const PRODUCT_STEPS: &[SelectionStep] = &[
    SelectionStep {
        target: "測",
        remainder: "sijathaacoenggeoizi",
    },
    SelectionStep {
        target: "是",
        remainder: "jathaacoenggeoizi",
    },
    SelectionStep {
        target: "日",
        remainder: "haacoenggeoizi",
    },
    SelectionStep {
        target: "下",
        remainder: "coenggeoizi",
    },
    SelectionStep {
        target: "場",
        remainder: "geoizi",
    },
    SelectionStep {
        target: "句",
        remainder: "zi",
    },
    SelectionStep {
        target: "子",
        remainder: "",
    },
];
const CANONICAL_STEPS: &[SelectionStep] = &[
    SelectionStep {
        target: "畀",
        remainder: "ng",
    },
    SelectionStep {
        target: "嗯",
        remainder: "",
    },
];
const CANGJIE_STEPS: &[SelectionStep] = &[
    SelectionStep {
        target: "粵",
        remainder: "qtt",
    },
    SelectionStep {
        target: "拼",
        remainder: "",
    },
];
const LUNA_STEPS: &[SelectionStep] = &[
    SelectionStep {
        target: "莫",
        remainder: "boyi",
    },
    SelectionStep {
        target: "伯",
        remainder: "yi",
    },
    SelectionStep {
        target: "洢",
        remainder: "",
    },
];
const DOUBLE_PINYIN_STEPS: &[SelectionStep] = &[
    SelectionStep {
        target: "好",
        remainder: "nivs",
    },
    SelectionStep {
        target: "逆",
        remainder: "vs",
    },
    SelectionStep {
        target: "鐘",
        remainder: "",
    },
];
const BOPOMOFO_STEPS: &[SelectionStep] = &[
    SelectionStep {
        target: "好",
        remainder: "su3j06",
    },
    SelectionStep {
        target: "你",
        remainder: "j06",
    },
    SelectionStep {
        target: "玩",
        remainder: "",
    },
];
const STROKE_STEPS: &[SelectionStep] = &[
    SelectionStep {
        target: "\u{597d}",
        remainder: "h",
    },
    SelectionStep {
        target: "\u{4e00}",
        remainder: "",
    },
];

const CASES: &[ReachabilityCase] = &[
    ReachabilityCase {
        label: "product-jyutping",
        schema_id: "jyut6ping3_mobile",
        dictionary_id: "jyut6ping3",
        prism_id: "jyut6ping3_mobile",
        input: "caksijathaacoenggeoizi",
        expected_final: "測是日下場句子",
        steps: PRODUCT_STEPS,
        assets: Assets::Tracked,
    },
    ReachabilityCase {
        label: "canonical-jyutping",
        schema_id: "jyut6ping3",
        dictionary_id: "jyut6ping3_m59_oracle_slice",
        prism_id: "jyut6ping3_m59_oracle_slice",
        input: "being",
        expected_final: "畀嗯",
        steps: CANONICAL_STEPS,
        assets: Assets::CanonicalJyutping,
    },
    ReachabilityCase {
        label: "cangjie5",
        schema_id: "cangjie5",
        dictionary_id: "cangjie5",
        prism_id: "cangjie5",
        input: "hwmvsqtt",
        expected_final: "粵拼",
        steps: CANGJIE_STEPS,
        assets: Assets::Tracked,
    },
    ReachabilityCase {
        label: "luna-pinyin",
        schema_id: "luna_pinyin",
        dictionary_id: "luna_pinyin",
        prism_id: "luna_pinyin",
        input: "moboyi",
        expected_final: "莫伯洢",
        steps: LUNA_STEPS,
        assets: Assets::Tracked,
    },
    ReachabilityCase {
        label: "luna-pinyin-octagram",
        schema_id: "luna_pinyin_octagram",
        dictionary_id: "luna_pinyin",
        prism_id: "luna_pinyin",
        input: "moboyi",
        expected_final: "莫伯洢",
        steps: LUNA_STEPS,
        assets: Assets::Tracked,
    },
    ReachabilityCase {
        label: "double-pinyin",
        schema_id: "double_pinyin",
        dictionary_id: "double_pinyin_m59_oracle_slice",
        prism_id: "double_pinyin_m59_oracle_slice",
        input: "hknivs",
        expected_final: "好逆鐘",
        steps: DOUBLE_PINYIN_STEPS,
        assets: Assets::DoublePinyin,
    },
    ReachabilityCase {
        label: "bopomofo",
        schema_id: "bopomofo",
        dictionary_id: "bopomofo_m59_oracle_slice",
        prism_id: "bopomofo_m59_oracle_slice",
        input: "cl3su3j06",
        expected_final: "好你玩",
        steps: BOPOMOFO_STEPS,
        assets: Assets::Bopomofo,
    },
    ReachabilityCase {
        label: "stroke-null-map",
        schema_id: "stroke",
        dictionary_id: "stroke",
        prism_id: "stroke",
        input: "zphzzhh",
        expected_final: "\u{597d}\u{4e00}",
        steps: STROKE_STEPS,
        assets: Assets::Tracked,
    },
];

#[test]
fn m59_schema_general_reachability_deployment_matrix_default_on_and_explicit_false() {
    let _guard = test_guard();
    assert_fixture_provenance_and_source_absence();

    // Keep the synthetic Standard Jyutping lane last so a failure there does
    // not hide independent deploy regressions in the other seven schemas.
    for case in CASES
        .iter()
        .filter(|case| case.assets != Assets::CanonicalJyutping)
        .chain(
            CASES
                .iter()
                .filter(|case| case.assets == Assets::CanonicalJyutping),
        )
    {
        run_default_on(case);
        run_explicit_false(case);
    }
}

#[test]
fn m59_correction_oracle_source_and_compiled_deploy_paths_match_complete_order() {
    let _guard = test_guard();
    let oracle: Value = serde_json::from_str(CORRECTION_SPELLING_ORACLE)
        .expect("curated correction oracle should parse");
    let expected = oracle["candidates"]
        .as_array()
        .expect("correction oracle candidates should be an array")
        .iter()
        .map(|candidate| {
            (
                candidate["text"].as_str().expect("oracle text"),
                candidate["comment"].as_str().expect("oracle comment"),
            )
        })
        .collect::<Vec<_>>();

    for compiled in [false, true] {
        let lane = if compiled { "compiled" } else { "source" };
        let runtime = YuneWebRuntime::create_with_schema(
            &format!("m59-correction-oracle-{lane}"),
            "correction_oracle",
        );
        stage_correction_oracle_sources(&runtime, &oracle);
        deploy_public_demo_schema(&runtime, "correction_oracle");

        if compiled {
            let task = CString::new("workspace_update:correction_oracle")
                .expect("workspace task should be valid");
            assert_eq!(RimeRunTask(task.as_ptr()), TRUE);
            assert_dictionary_rebuilt_from_source(
                &workspace_dictionary_rebuild_reports(),
                "correction_oracle",
            );
            for suffix in ["table.bin", "prism.bin", "reverse.bin", "poet.bin"] {
                let file_name = format!("correction_oracle.{suffix}");
                let source = runtime.user.join("build").join(&file_name);
                if source.exists() {
                    fs::copy(source, runtime.shared.join(file_name))
                        .expect("rebuilt correction artifact should stage as prebuilt");
                }
            }
            deploy_public_demo_schema(&runtime, "correction_oracle");
        }

        let state = unsafe {
            yune_web_init(
                runtime.shared_c.as_ptr(),
                runtime.user_c.as_ptr(),
                runtime.schema_id_c.as_ptr(),
            )
        };
        assert!(!state.is_null());
        let inspector = CString::new("yune_inspector").expect("option should be valid");
        assert_eq!(
            unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
            TRUE
        );
        let page = process_input(state, "cu");
        let actual = page["context"]["candidates"]
            .as_array()
            .expect("correction candidate page should be an array")
            .iter()
            .map(|candidate| {
                (
                    candidate["text"].as_str().expect("candidate text"),
                    candidate["comment"].as_str().expect("candidate comment"),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{lane} correction deploy order");
        let storage = &page["context"]["debug"]["storage"];
        if compiled {
            assert_schema_storage_byte_backed("correction_oracle", storage);
        } else {
            assert_eq!(
                storage["source_fallback"].as_bool(),
                Some(false),
                "a source-only lane has no rejected compiled artifact and must not invent a fallback diagnostic: {storage:?}"
            );
            assert!(
                storage["source_fallbacks"]
                    .as_array()
                    .is_some_and(Vec::is_empty),
                "source-only correction deployment must not report compiled rejection rows: {storage:?}"
            );
            let selected = storage["selected"]
                .as_array()
                .expect("source correction lane should expose selected storage");
            assert_eq!(selected.len(), 1, "source correction translator count");
            assert_eq!(selected[0]["selected_storage"].as_str(), Some("owned_heap"));
            assert_eq!(selected[0]["mapping_mode"].as_str(), Some("owned_heap"));
            assert_eq!(selected[0]["is_marisa_backed"].as_bool(), Some(false));
            assert_eq!(selected[0]["byte_source_len"].as_u64(), Some(0));
            assert_eq!(selected[0]["stored_entry_count"].as_u64(), Some(3));
        }

        unsafe { yune_web_cleanup(state) };
        runtime.remove();
    }
}

#[test]
fn m59_product_zi_selection_consumes_the_full_surface_spelling() {
    let _guard = test_guard();
    let case = &CASES[0];
    let runtime = YuneWebRuntime::create_with_schema("m59-product-zi-span", case.schema_id);
    stage_and_deploy(&runtime, case, true);
    assert_deployed_flag(&runtime, case, None);

    let state = init_with_inspector(&runtime, case);
    let first_page = process_input(state, "zi");
    assert_schema_storage_byte_backed(case.schema_id, &first_page["context"]["debug"]["storage"]);
    let (target_page, index) =
        m59_luna_page_to_target(state, first_page, "子", "product zi surface-span control");
    assert_eq!(target_page["context"]["input"].as_str(), Some("zi"));
    assert_eq!(
        target_page["context"]["debug"]["spelling_algebra"][0]["expanded_codes"],
        serde_json::json!(["zi", "zi1", "zi2", "zi3", "zi6"])
    );
    let target = &target_page["context"]["candidates"][index];
    assert_eq!(target["text"].as_str(), Some("子"));
    assert_eq!(target["source"].as_str(), Some("table"));
    assert!(
        target["comment"]
            .as_str()
            .is_some_and(|comment| comment.contains(",子,zi2,")),
        "selected product row must retain canonical raw-code metadata: {target:?}"
    );

    let selected = response_json(unsafe { yune_web_select_candidate(state, index) });
    assert_eq!(selected["commits"], serde_json::json!(["子"]));
    assert_eq!(
        selected["context"]["input"],
        Value::String(String::new()),
        "zi -> 子 must consume both surface characters, never the shorter z abbreviation span: {selected:?}"
    );
    assert_eq!(selected["status"]["is_composing"], Value::Bool(false));

    let two_syllables = process_input(state, "sizi");
    let (_si_page, si_index) =
        m59_luna_page_to_target(state, two_syllables, "是", "product si|zi span control");
    let after_si = response_json(unsafe { yune_web_select_candidate(state, si_index) });
    assert_eq!(after_si["commits"], serde_json::json!(["是"]));
    assert_eq!(after_si["context"]["input"].as_str(), Some("zi"));
    let (zi_page, zi_index) =
        m59_luna_page_to_target(state, after_si, "子", "product si|zi span control");
    assert_eq!(zi_page["context"]["input"].as_str(), Some("zi"));
    let after_zi = response_json(unsafe { yune_web_select_candidate(state, zi_index) });
    assert_eq!(after_zi["commits"], serde_json::json!(["子"]));
    assert_eq!(
        after_zi["context"]["input"],
        Value::String(String::new()),
        "after partial selection, zi -> 子 must still consume the full surface spelling"
    );
    assert_eq!(after_zi["status"]["is_composing"], Value::Bool(false));

    let geoi_zi = process_input(state, "geoizi");
    let (_geoi_page, geoi_index) =
        m59_luna_page_to_target(state, geoi_zi, "句", "product geoi|zi span control");
    let after_geoi = response_json(unsafe { yune_web_select_candidate(state, geoi_index) });
    assert_eq!(after_geoi["commits"], serde_json::json!(["句"]));
    assert_eq!(after_geoi["context"]["input"].as_str(), Some("zi"));
    let (zi_page, zi_index) =
        m59_luna_page_to_target(state, after_geoi, "子", "product geoi|zi span control");
    assert_eq!(zi_page["context"]["input"].as_str(), Some("zi"));
    let after_zi = response_json(unsafe { yune_web_select_candidate(state, zi_index) });
    assert_eq!(after_zi["commits"], serde_json::json!(["子"]));
    assert_eq!(
        after_zi["context"]["input"],
        Value::String(String::new()),
        "after geoi -> 句, zi -> 子 must still consume the full surface spelling"
    );
    assert_eq!(after_zi["status"]["is_composing"], Value::Bool(false));

    for (label, input, steps) in [
        ("coeng|geoi|zi", "coenggeoizi", &PRODUCT_STEPS[4..]),
        ("haa|coeng|geoi|zi", "haacoenggeoizi", &PRODUCT_STEPS[3..]),
        (
            "jat|haa|coeng|geoi|zi",
            "jathaacoenggeoizi",
            &PRODUCT_STEPS[2..],
        ),
        (
            "si|jat|haa|coeng|geoi|zi",
            "sijathaacoenggeoizi",
            &PRODUCT_STEPS[1..],
        ),
        (
            "cak|si|jat|haa|coeng|geoi|zi",
            "caksijathaacoenggeoizi",
            PRODUCT_STEPS,
        ),
    ] {
        let mut page = process_input(state, input);
        for step in steps {
            let (_found_page, index) = m59_luna_page_to_target(state, page, step.target, label);
            let selected = response_json(unsafe { yune_web_select_candidate(state, index) });
            assert_eq!(selected["commits"], serde_json::json!([step.target]));
            assert_eq!(
                selected["context"]["input"],
                Value::String(step.remainder.to_owned()),
                "{label}: selecting {} must consume its full surface spelling",
                step.target
            );
            page = selected;
        }
    }

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

fn run_default_on(case: &ReachabilityCase) {
    let runtime = YuneWebRuntime::create_with_schema(
        &format!("m59-reach-{}-default-on", case.label),
        case.schema_id,
    );
    stage_and_deploy(&runtime, case, true);
    assert_deployed_flag(&runtime, case, None);

    let state = init_with_inspector(&runtime, case);
    let mut page = process_input(state, case.input);
    assert_schema_storage_byte_backed(case.schema_id, &page["context"]["debug"]["storage"]);
    if case.schema_id == "stroke" {
        assert_stroke_identity_owner_boundary(&page["context"]["debug"]["storage"]);
    }

    let mut committed = String::new();
    for step in case.steps {
        let (found_page, index) = m59_luna_page_to_target(
            state,
            page,
            step.target,
            &format!("{} default-on", case.label),
        );
        let found = &found_page["context"]["candidates"][index];
        assert_eq!(
            found["text"],
            Value::String(step.target.to_owned()),
            "{} should page to the exact standalone target",
            case.label
        );
        let selected = response_json(unsafe { yune_web_select_candidate(state, index) });
        assert_eq!(
            selected["commits"],
            Value::Array(vec![Value::String(step.target.to_owned())]),
            "{} selecting {} should commit exactly that candidate",
            case.label,
            step.target
        );
        committed.push_str(step.target);
        assert_selection_remainder(case, step, &selected);
        page = selected;
    }
    assert_eq!(
        committed, case.expected_final,
        "{} should reproduce the expected selected composition/output",
        case.label
    );

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

fn run_explicit_false(case: &ReachabilityCase) {
    let runtime = YuneWebRuntime::create_with_schema(
        &format!("m59-reach-{}-explicit-false", case.label),
        case.schema_id,
    );
    stage_and_deploy(&runtime, case, false);
    assert_deployed_flag(&runtime, case, Some(false));

    let state = init_with_inspector(&runtime, case);
    let page = process_input(state, case.input);
    assert_schema_storage_byte_backed(case.schema_id, &page["context"]["debug"]["storage"]);
    if case.schema_id == "stroke" {
        assert_stroke_identity_owner_boundary(&page["context"]["debug"]["storage"]);
    }

    if case.schema_id == "jyut6ping3_mobile" {
        // The shipped product profile independently enables prefix_fallback.
        // Disabling only M59's schema-general reachability switch must therefore
        // leave this control row reachable through the pre-existing mechanism.
        let mut current = page;
        let mut committed = String::new();
        for step in case.steps {
            let (found_page, index) = m59_luna_page_to_target(
                state,
                current,
                step.target,
                "product explicit-false prefix_fallback control",
            );
            let _ = found_page;
            let selected = response_json(unsafe { yune_web_select_candidate(state, index) });
            assert_eq!(
                selected["commits"],
                Value::Array(vec![Value::String(step.target.to_owned())])
            );
            committed.push_str(step.target);
            assert_selection_remainder(case, step, &selected);
            current = selected;
        }
        assert_eq!(committed, case.expected_final);
    } else if case.schema_id == "luna_pinyin" {
        // The corrected ScriptTranslation phrase iterator independently emits
        // the first-syllable rows even when M59's supplemental injection is
        // disabled. Lock the explicit-false output to the pinned librime page
        // instead of treating the oracle-owned `莫` row as injected behavior.
        let all_candidates = all_candidate_texts(state, page);
        let oracle: Value = serde_json::from_str(LUNA_COMPOSITION)
            .expect("pinned Luna composition fixture should parse");
        let expected_page = oracle["inputs"][case.input]["page_0"]
            .as_array()
            .expect("pinned Luna input should have a page-0 list")
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("pinned Luna page text should be a string")
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            &all_candidates[..expected_page.len()],
            expected_page,
            "Luna explicit-false must retain the independently oracle-owned phrase stream"
        );
    } else {
        let standalone_target = case
            .steps
            .first()
            .expect("every matrix row should have a leading target")
            .target;
        let all_candidates = all_candidate_texts(state, page);
        assert!(
            !all_candidates.iter().any(|text| text == standalone_target),
            "{} explicit-false control must remove injected standalone target {}; candidates were {:?}",
            case.label,
            standalone_target,
            all_candidates
        );
        // A whole sentence candidate is allowed to remain: this control proves
        // only that the independently configured reachability injection is off.
    }

    unsafe { yune_web_cleanup(state) };
    runtime.remove();
}

fn init_with_inspector(
    runtime: &YuneWebRuntime,
    case: &ReachabilityCase,
) -> *mut yune_rime_api::YuneWebState {
    let state = unsafe {
        yune_web_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null(), "{} should initialize", case.label);
    let inspector = CString::new("yune_inspector").expect("option name should be valid");
    assert_eq!(
        unsafe { yune_web_set_option(state, inspector.as_ptr(), TRUE) },
        TRUE,
        "{} should enable inspector output",
        case.label
    );
    state
}

fn assert_selection_remainder(case: &ReachabilityCase, step: &SelectionStep, selected: &Value) {
    assert_eq!(
        selected["context"]["input"],
        Value::String(step.remainder.to_owned()),
        "{} selecting {} should consume the surface spelling and retain {}",
        case.label,
        step.target,
        step.remainder
    );
    assert_eq!(
        selected["status"]["is_composing"],
        Value::Bool(!step.remainder.is_empty()),
        "{} composing status should track the exact remainder",
        case.label
    );
}

fn assert_stroke_identity_owner_boundary(storage: &Value) {
    let rows = storage["memory_owner_rows"]
        .as_array()
        .expect("Stroke inspector should expose memory owner rows");
    for owner_name in [
        "translator.leading_fetch_seed",
        "translator.leading_fetch_index",
        "prism.spelling_map",
    ] {
        let matching = rows
            .iter()
            .filter(|row| row["owner"].as_str() == Some(owner_name))
            .collect::<Vec<_>>();
        assert!(
            !matching.is_empty(),
            "Stroke should report {owner_name}: {rows:?}"
        );
        assert!(
            matching.iter().all(|row| {
                row["estimated_bytes"].as_u64() == Some(0)
                    && row["item_count"].as_u64() == Some(0)
            }),
            "Stroke {owner_name} must remain zero-allocation after the first multi-character input: {matching:?}"
        );
    }
    assert!(
        rows.iter().any(|row| {
            row["owner"].as_str() == Some("prism.double_array_units")
                && row["estimated_bytes"]
                    .as_u64()
                    .is_some_and(|bytes| bytes > 0)
                && row["storage"]
                    .as_str()
                    .is_some_and(|value| value.starts_with("byte_backed:"))
        }),
        "Stroke Darts must remain byte-backed while the identity map stays implicit: {rows:?}"
    );
}

fn all_candidate_texts(state: *mut yune_rime_api::YuneWebState, mut page: Value) -> Vec<String> {
    let mut output = Vec::new();
    for turn in 0..10_000 {
        output.extend(
            page["context"]["candidates"]
                .as_array()
                .expect("candidate page should be an array")
                .iter()
                .filter_map(|candidate| candidate["text"].as_str().map(str::to_owned)),
        );
        if page["context"]["is_last_page"].as_bool() == Some(true) {
            return output;
        }
        let next = response_json(unsafe { yune_web_flip_page(state, FALSE) });
        assert_eq!(
            next["handled"],
            Value::Bool(true),
            "candidate enumeration stopped before the declared last page at turn {turn}"
        );
        page = next;
    }
    panic!("candidate enumeration exceeded 10,000 pages without reaching the declared last page")
}

fn stage_and_deploy(runtime: &YuneWebRuntime, case: &ReachabilityCase, default_on: bool) {
    match case.assets {
        Assets::Tracked => stage_tracked(runtime, case, default_on),
        Assets::CanonicalJyutping | Assets::DoublePinyin | Assets::Bopomofo => {
            stage_synthetic_oracle_slice(runtime, case, default_on)
        }
    }
    deploy_public_demo_schema(runtime, case.schema_id);
    let task = CString::new(format!("workspace_update:{}", case.schema_id))
        .expect("workspace task name should be valid");
    assert_eq!(
        RimeRunTask(task.as_ptr()),
        TRUE,
        "{} should resolve source-current artifacts through the real workspace deployment path",
        case.label
    );
    let reports = workspace_dictionary_rebuild_reports();
    assert!(
        !reports.is_empty(),
        "{} should emit rebuild reports",
        case.label
    );
    for report in &reports {
        assert_eq!(
            (
                report.report.table,
                report.report.prism,
                report.report.reverse
            ),
            (
                RimeDictArtifactStatus::Rebuilt,
                RimeDictArtifactStatus::Rebuilt,
                RimeDictArtifactStatus::Rebuilt
            ),
            "{} clean disposable workspace must compile every reported dictionary from source: {report:?}",
            case.label
        );
    }
    assert_dictionary_rebuilt_from_source(&reports, case.dictionary_id);
    stage_workspace_compiled_artifacts(runtime, case, &reports);
    // Re-run the schema deployment with the source-current artifacts staged
    // as prebuilt data. Runtime init must select these bytes, never a stale
    // prebuilt or the still-present source dictionary's owned-heap fallback.
    deploy_public_demo_schema(runtime, case.schema_id);
}

fn stage_workspace_compiled_artifacts(
    runtime: &YuneWebRuntime,
    case: &ReachabilityCase,
    reports: &[yune_rime_api::WorkspaceDictionaryRebuildReport],
) {
    let build = runtime.user.join("build");
    let mut artifact_names = BTreeSet::new();
    for report in reports {
        for suffix in ["table.bin", "reverse.bin", "prism.bin", "poet.bin"] {
            artifact_names.insert(format!("{}.{suffix}", report.dictionary_id));
        }
    }
    artifact_names.insert(format!("{}.prism.bin", case.prism_id));
    for artifact_name in artifact_names {
        let source = build.join(&artifact_name);
        if source.exists() {
            fs::copy(&source, runtime.shared.join(&artifact_name)).unwrap_or_else(|error| {
                panic!(
                    "{} source-current {artifact_name} should stage as a deterministic prebuilt asset: {error}",
                    case.label
                )
            });
        }
    }
    for required in [
        format!("{}.table.bin", case.dictionary_id),
        format!("{}.reverse.bin", case.dictionary_id),
        format!("{}.prism.bin", case.prism_id),
    ] {
        assert!(
            runtime.shared.join(&required).is_file(),
            "{} must stage exact report-owned artifact {required}",
            case.label
        );
    }
}

fn stage_correction_oracle_sources(runtime: &YuneWebRuntime, oracle: &Value) {
    let default =
        "config_version: m59-correction-oracle\nschema_list:\n  - schema: correction_oracle\n";
    fs::write(runtime.shared.join("default.yaml"), default)
        .expect("correction shared default should be written");
    fs::write(runtime.user.join("build/default.yaml"), default)
        .expect("correction deployed default should be written");

    let mut dictionary = String::from(
        "---\nname: correction_oracle\nversion: '1.0'\nsort: by_weight\ncolumns: [text, code, weight]\n...\n\n",
    );
    for row in oracle["dictionary_rows"]
        .as_array()
        .expect("correction dictionary rows should be present")
    {
        dictionary.push_str(row["text"].as_str().expect("dictionary text"));
        dictionary.push('\t');
        dictionary.push_str(row["code"].as_str().expect("dictionary code"));
        dictionary.push('\t');
        dictionary.push_str(
            &row["weight"]
                .as_i64()
                .expect("dictionary weight")
                .to_string(),
        );
        dictionary.push('\n');
    }
    fs::write(
        runtime.shared.join("correction_oracle.dict.yaml"),
        dictionary,
    )
    .expect("correction dictionary should be written");

    let schema = r#"schema:
  schema_id: correction_oracle
  name: M59 correction spelling oracle
  version: '1.0'
switches:
  - name: ascii_mode
    reset: 0
    states: [Chinese, ASCII]
engine:
  processors: [speller, selector, navigator, express_editor]
  segmentors: [ascii_segmentor, abc_segmentor, fallback_segmentor]
  translators: [script_translator]
speller:
  alphabet: abcdefghijklmnopqrstuvwxyz
  delimiter: " '"
  algebra:
    - derive/^cuo$/cu/correction
translator:
  dictionary: correction_oracle
  enable_completion: false
  enable_sentence: false
  enable_correction: false
  spelling_hints: 9
  always_show_comments: true
menu:
  page_size: 5
"#;
    fs::write(runtime.shared.join("correction_oracle.schema.yaml"), schema)
        .expect("correction schema should be written");
}

fn stage_tracked(runtime: &YuneWebRuntime, case: &ReachabilityCase, default_on: bool) {
    let schema_root = browser_app_schema_root();
    copy_clean_schema_sources(&schema_root, &runtime.shared);

    let staging = runtime.user.join("build");
    for file_name in ["default.yaml", "jyut6ping3_mobile.schema.yaml"] {
        fs::copy(
            schema_root.join("build").join(file_name),
            staging.join(file_name),
        )
        .expect("tracked build preload should be copied");
    }
    let schema_path = runtime
        .shared
        .join(format!("{}.schema.yaml", case.schema_id));
    let source =
        fs::read_to_string(&schema_path).expect("selected tracked schema should be staged");
    write_schema_flag(&schema_path, &source, default_on);

    if case.schema_id == "luna_pinyin_octagram" {
        // This is a valid compiled grammar payload, not a dummy header. Its
        // single entry is deliberately unrelated to the fixture phrase: the
        // matrix proves the real Octagram schema can load its declared grammar
        // while the reachability behavior remains dictionary/algebra-owned.
        fs::write(
            runtime.shared.join("zh-hant-t-essay-bgw.gram"),
            synthetic_octagram_gram(&[("今天會議", 500_000)]),
        )
        .expect("synthetic valid Octagram grammar should be written");
    }
}

fn stage_synthetic_oracle_slice(
    runtime: &YuneWebRuntime,
    case: &ReachabilityCase,
    default_on: bool,
) {
    let fixture: Value = serde_json::from_str(match case.assets {
        Assets::CanonicalJyutping => CANONICAL_JYUTPING_BEING,
        Assets::DoublePinyin => DOUBLE_PINYIN_WHOLE_INPUT,
        Assets::Bopomofo => BOPOMOFO_WHOLE_INPUT,
        Assets::Tracked => unreachable!("tracked assets use their shipped schema"),
    })
    .expect("whole-input oracle fixture should parse");
    let rows = if case.assets == Assets::CanonicalJyutping {
        fixture["source_lexicon"]["source_dictionary_rows"]
            .as_array()
            .expect("canonical fixture should preserve source dictionary rows")
            .iter()
            .map(|row| {
                row["row"]
                    .as_str()
                    .expect("canonical source row should preserve exact text")
            })
            .collect::<Vec<_>>()
    } else {
        fixture["capture"]["source_dictionary_rows"]
            .as_array()
            .expect("oracle fixture should preserve source dictionary rows")
            .iter()
            .map(|row| row.as_str().expect("source row should be text"))
            .collect::<Vec<_>>()
    };
    let algebra = if case.assets == Assets::CanonicalJyutping {
        vec![r"derive/\d//"]
    } else {
        fixture["capture"]["speller_algebra_rules"]
            .as_array()
            .expect("oracle fixture should preserve speller algebra")
            .iter()
            .map(|rule| rule.as_str().expect("algebra rule should be text"))
            .collect::<Vec<_>>()
    };

    let dictionary_id = format!("{}_m59_oracle_slice", case.schema_id);
    let default_config = format!(
        "config_version: m59-oracle-slice\nschema_list:\n  - schema: {}\n",
        case.schema_id
    );
    fs::write(runtime.shared.join("default.yaml"), &default_config)
        .expect("synthetic shared default config should be written");
    fs::write(runtime.user.join("build/default.yaml"), default_config)
        .expect("synthetic deployed default config should be written");

    let mut dictionary = format!(
        "---\nname: {dictionary_id}\nversion: 'M59'\nsort: original\nuse_preset_vocabulary: false\n...\n\n"
    );
    for row in rows {
        dictionary.push_str(row);
        dictionary.push('\n');
    }
    fs::write(
        runtime.shared.join(format!("{dictionary_id}.dict.yaml")),
        dictionary,
    )
    .expect("synthetic oracle-slice dictionary should be written");

    let alphabet = match case.assets {
        Assets::CanonicalJyutping | Assets::DoublePinyin => "zyxwvutsrqponmlkjihgfedcba",
        Assets::Bopomofo => "1qaz2wsxedcrfv5tgbyhnujm8ik,9ol.0p;/- 6347",
        Assets::Tracked => unreachable!(),
    };
    let config = serde_json::json!({
        "schema": {
            "schema_id": case.schema_id,
            "name": format!("M59 oracle slice {}", case.schema_id),
        },
        "switches": [{"name": "ascii_mode", "reset": 0}],
        "menu": {"page_size": 5},
        "engine": {
            "processors": ["speller", "selector", "navigator", "express_editor"],
            "segmentors": ["abc_segmentor", "fallback_segmentor"],
            "translators": ["script_translator"],
        },
        "speller": {
            "alphabet": alphabet,
            "delimiter": " '",
            "algebra": algebra,
        },
        "translator": {
            "dictionary": dictionary_id,
            "enable_completion": true,
            "enable_sentence": true,
            "enable_user_dict": false,
            "combine_candidates": true,
        },
    });
    let mut source = serde_yaml::to_string(&config).expect("synthetic schema should serialize");
    let schema_path = runtime
        .shared
        .join(format!("{}.schema.yaml", case.schema_id));
    if !default_on {
        source = inject_explicit_false(&source);
    }
    fs::write(schema_path, source).expect("synthetic oracle-slice schema should be written");
}

fn write_schema_flag(path: &Path, source: &str, default_on: bool) {
    assert!(
        !source.contains("leading_syllable_reachability:"),
        "matrix default-on rows must exercise the schema-general default, not a per-schema true flag"
    );
    let selected = if default_on {
        source.to_owned()
    } else {
        inject_explicit_false(source)
    };
    fs::write(path, selected).expect("selected schema flag should be staged");
}

fn inject_explicit_false(source: &str) -> String {
    assert!(
        !source.contains("leading_syllable_reachability:"),
        "explicit-false control should change only an otherwise absent flag"
    );
    let marker = "\ntranslator:\n";
    assert!(
        source.contains(marker),
        "schema should expose the primary translator block"
    );
    let patched = source.replacen(
        marker,
        "\ntranslator:\n  leading_syllable_reachability: false\n",
        1,
    );
    assert_eq!(
        patched.replacen("  leading_syllable_reachability: false\n", "", 1),
        source,
        "explicit-false patch must alter only the named translator flag"
    );
    patched
}

fn assert_deployed_flag(runtime: &YuneWebRuntime, case: &ReachabilityCase, expected: Option<bool>) {
    let path = runtime
        .user
        .join("build")
        .join(format!("{}.schema.yaml", case.schema_id));
    let deployed: Value = serde_yaml::from_str(
        &fs::read_to_string(&path).expect("deployed schema should be readable"),
    )
    .expect("deployed schema should parse");
    assert_eq!(
        deployed
            .get("translator")
            .and_then(|translator| translator.get("leading_syllable_reachability"))
            .and_then(config_bool_like),
        expected,
        "{} should deploy the intended default/explicit-false setting",
        case.label
    );
    if case.schema_id == "jyut6ping3_mobile" {
        assert_eq!(
            deployed
                .get("translator")
                .and_then(|translator| translator.get("prefix_fallback"))
                .and_then(config_bool_like),
            Some(true),
            "product explicit-false control depends on its independent shipped prefix_fallback"
        );
    }
    if case.assets == Assets::CanonicalJyutping {
        assert!(
            deployed.pointer("/yune/profile").is_none(),
            "canonical Jyutping must remain the Standard lane without a TypeDuck marker"
        );
        assert!(
            deployed.pointer("/translator/prefix_fallback").is_none(),
            "canonical explicit-false must isolate leading_syllable_reachability"
        );
    }
}

fn assert_fixture_provenance_and_source_absence() {
    let schema_root = browser_app_schema_root();

    let product: Value = serde_json::from_str(TYPEDUCK_M28).expect("M28 fixture should parse");
    assert_eq!(product["input"].as_str(), Some(CASES[0].input));
    assert_eq!(
        product["captured_final_flow"]["final_commit_text"].as_str(),
        Some(CASES[0].expected_final)
    );
    // The shipped product dictionary is already flattened (no YAML imports),
    // and its schema also enables the shipped preset vocabulary.  Absence must
    // therefore cover both actual product lexicon inputs, not only the primary
    // dictionary file.
    for path in [
        schema_root.join("jyut6ping3.dict.yaml"),
        schema_root.join("essay.txt"),
    ] {
        assert_no_exact_term(&path, CASES[0].expected_final);
    }

    let canonical: Value =
        serde_json::from_str(CANONICAL_JYUTPING_BEING).expect("canonical fixture should parse");
    assert_eq!(
        canonical["oracle"]["commit"].as_str(),
        Some("33e78140250125871856cdc5b42ddc6a5fcd3cd4")
    );
    assert_eq!(
        canonical["source_lexicon"]["source_commit"].as_str(),
        Some("c99b16e44d2df77a5cb8fb0867dd2bab7a112cb0")
    );
    assert_eq!(
        canonical["source_lexicon"]["source_tree"].as_str(),
        Some("eb193fb80675ffa60df3c32bf24afa7d7f68617a")
    );
    assert_eq!(
        canonical["source_lexicon"]["source_dictionary_rows"]
            .as_array()
            .expect("canonical source rows should be present")
            .iter()
            .map(|row| row["row"].as_str().expect("source row should be text"))
            .collect::<Vec<_>>(),
        vec![
            "畀\tbei2",
            "畀\tbei3\t3%",
            "嗯\tng2\t3%",
            "嗯\tng4\t3%",
            "嗯\tng5\t3%",
            "嗯\tng6",
        ],
        "canonical deployment slice must be built only from hash-bound fixture rows"
    );
    assert_whole_input_oracle_row(
        &canonical["source_lexicon"]["whole_input_oracle_rows"][0],
        &CASES[1],
    );
    assert_no_exact_term(
        &schema_root.join("jyut6ping3.dict.yaml"),
        CASES[1].expected_final,
    );

    let cangjie: Value =
        serde_json::from_str(CANGJIE_COMPOSITION).expect("Cangjie fixture should parse");
    let cangjie_row = cangjie["composition_rows"]
        .as_array()
        .expect("Cangjie fixture should contain composition rows")
        .iter()
        .find(|row| row["input"].as_str() == Some(CASES[2].input))
        .expect("Cangjie fixture should contain hwmvsqtt");
    assert_eq!(
        cangjie_row["target"].as_str(),
        Some(CASES[2].expected_final)
    );
    assert_no_exact_term(
        &schema_root.join("cangjie5.dict.yaml"),
        CASES[2].expected_final,
    );

    let luna: Value = serde_json::from_str(LUNA_COMPOSITION).expect("Luna fixture should parse");
    assert_eq!(
        luna["compositions"]["moboyi"]["final_commit"].as_str(),
        Some(CASES[3].expected_final)
    );
    for path in [
        schema_root.join("luna_pinyin.dict.yaml"),
        schema_root.join("essay.txt"),
    ] {
        assert_no_exact_term(&path, CASES[3].expected_final);
    }

    for (fixture, case) in [
        (DOUBLE_PINYIN_WHOLE_INPUT, &CASES[5]),
        (BOPOMOFO_WHOLE_INPUT, &CASES[6]),
    ] {
        let parsed: Value = serde_json::from_str(fixture).expect("oracle fixture should parse");
        assert_whole_input_oracle_row(&parsed["capture"]["whole_input_oracle_rows"][0], case);
        assert_captured_source_dictionary_rows_exclude_target(&parsed["capture"], case);
    }

    let stroke_case = CASES
        .iter()
        .find(|case| case.schema_id == "stroke")
        .expect("matrix should contain the tracked Stroke row");
    let stroke_path = schema_root.join("stroke.dict.yaml");
    let stroke_source =
        fs::read_to_string(&stroke_path).expect("tracked Stroke source should be readable");
    for row in ["\u{597d}\tzphzzh", "\u{4e00}\th"] {
        assert!(
            stroke_source.lines().any(|line| line == row),
            "Stroke matrix must be grounded in the exact tracked source row {row}"
        );
    }
    let absent_code = "zphzzhh";
    assert!(
        stroke_source
            .lines()
            .all(|line| line.split('\t').nth(1) != Some(absent_code)),
        "Stroke control requires the first code zphzzh to be the longest recognized proper prefix; {absent_code} must be absent"
    );
    for path in [stroke_path, schema_root.join("essay.txt")] {
        assert_no_exact_term(&path, stroke_case.expected_final);
    }
}

fn assert_captured_source_dictionary_rows_exclude_target(capture: &Value, case: &ReachabilityCase) {
    let rows = capture["source_dictionary_rows"]
        .as_array()
        .expect("capture should preserve source dictionary rows");
    assert!(
        rows.iter().all(|row| {
            row.as_str()
                .expect("captured source dictionary row should be text")
                .split('\t')
                .next()
                != Some(case.expected_final)
        }),
        "captured source dictionary rows must directly prove whole output {} is absent",
        case.expected_final
    );
}

fn assert_whole_input_oracle_row(row: &Value, case: &ReachabilityCase) {
    assert_eq!(row["input"].as_str(), Some(case.input));
    assert_eq!(row["oracle_top"].as_str(), Some(case.expected_final));
    assert_eq!(row["source_dictionary_exact_term_count"].as_u64(), Some(0));
    assert_eq!(row["source_vocabulary_exact_term_count"].as_u64(), Some(0));
    assert_eq!(row["source_lexicon_absent"].as_bool(), Some(true));
}

fn assert_no_exact_term(path: &Path, target: &str) {
    let source = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} should be readable: {error}", path.display()));
    assert!(
        !source.lines().any(|line| {
            !line.trim_start().starts_with('#')
                && line.split('\t').next().is_some_and(|text| text == target)
        }),
        "whole output {target} must be absent from source lexicon {}",
        path.display()
    );
}

fn synthetic_octagram_gram(entries: &[(&str, u32)]) -> Vec<u8> {
    let encoded = entries
        .iter()
        .map(|(key, value)| (encode_octagram_key(key), *value))
        .collect::<Vec<_>>();
    let double_array =
        DartsDoubleArray::build_bytes(&encoded).expect("synthetic gram keys should build");
    let mut bytes = vec![0; 44];
    bytes[.."Rime::Grammar/1.0".len()].copy_from_slice(b"Rime::Grammar/1.0");
    bytes[36..40].copy_from_slice(&(double_array.units().len() as u32).to_le_bytes());
    bytes[40..44].copy_from_slice(&4_i32.to_le_bytes());
    for unit in double_array.units() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    bytes
}
