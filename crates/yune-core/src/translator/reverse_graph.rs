use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use crate::{RimePrismRuntimePayload, TableEntry};

const COMPLETION_CREDIBILITY: f32 = -2.995_732_3;
const PRISM_EXPAND_SEARCH_LIMIT: usize = 512;
const MAX_REVERSE_GRAPH_PATH_EXPANSIONS: usize = 65_536;
const MAX_REVERSE_GRAPH_PATHS: usize = 4_096;
const MAX_REVERSE_GRAPH_DIRECT_ROW_WORK: usize = 65_536;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum SpellingType {
    Normal,
    Fuzzy,
    Abbreviation,
    Completion,
    Ambiguous,
    Invalid,
}

#[derive(Clone, Debug)]
struct GraphChoice {
    syllable_id: usize,
    code: String,
    spelling_type: SpellingType,
    correction: bool,
    credibility: f32,
}

#[derive(Clone, Debug)]
struct GraphEdge {
    choices: Vec<GraphChoice>,
    best_non_correction_type: SpellingType,
}

impl Default for GraphEdge {
    fn default() -> Self {
        Self {
            choices: Vec::new(),
            best_non_correction_type: SpellingType::Invalid,
        }
    }
}

type GraphEdges = BTreeMap<usize, BTreeMap<usize, GraphEdge>>;
type CompletedCodePath = (String, f32, usize, Vec<ReverseGraphPathSegment>);
type RankedReverseGraphRow = (ReverseGraphRow, usize, usize);

#[derive(Clone, Debug)]
pub(super) struct ReverseGraphRow {
    pub(super) entry: TableEntry,
    pub(super) comparison_weight: f32,
}

#[derive(Clone, Debug)]
pub(super) struct ReverseGraphLookup {
    pub(super) rows: Vec<ReverseGraphRow>,
    pub(super) paths: Vec<ReverseGraphPath>,
    pub(super) normal_prefix_quality: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ReverseGraphPath {
    pub(super) path_order: usize,
    pub(super) segments: Vec<ReverseGraphPathSegment>,
}

#[derive(Clone, Debug)]
pub(super) struct ReverseGraphPathSegment {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) code: String,
    pub(super) credibility: f32,
}

/// Reproduces the pinned reverse-lookup `Syllabifier("", true, strict)` path.
///
/// The completion flag here is intentionally unconditional. librime's reverse
/// translator defaults `enable_completion` to false for its direct dictionary
/// lookup, but its non-completion branch constructs a syllabifier whose final
/// incomplete syllable is completed with a 512-key `Prism::ExpandSearch`.
pub(super) fn lookup_reverse_graph(
    input: &str,
    entries: &[TableEntry],
    entries_by_code: &HashMap<String, Vec<usize>>,
    prism: &RimePrismRuntimePayload,
    syllabary_codes: &[String],
    strict_spelling: bool,
    sort_by_weight: bool,
) -> Option<ReverseGraphLookup> {
    if input.is_empty() || !sort_by_weight {
        return None;
    }

    let mut vertices = BTreeMap::<usize, SpellingType>::new();
    let mut edges = GraphEdges::new();
    let mut queue = BTreeSet::from([(0usize, SpellingType::Normal)]);
    let mut farthest = 0usize;

    while let Some(vertex) = queue.pop_first() {
        let (current_pos, path_type) = vertex;
        if vertices.contains_key(&current_pos) {
            continue;
        }
        vertices.insert(current_pos, path_type);
        farthest = farthest.max(current_pos);

        let suffix = &input[current_pos..];
        let matches = prism.common_prefix_canonical_codes(suffix, syllabary_codes, usize::MAX);
        for (consumed, lookup) in matches {
            if consumed == 0 || !suffix.is_char_boundary(consumed) {
                continue;
            }
            let end_pos = current_pos.saturating_add(consumed);
            if end_pos > input.len() {
                continue;
            }
            let spelling_type = lookup_spelling_type(lookup.spelling_type);
            if strict_spelling
                && current_pos == 0
                && end_pos == input.len()
                && spelling_type != SpellingType::Normal
            {
                continue;
            }
            let edge = edges
                .entry(current_pos)
                .or_default()
                .entry(end_pos)
                .or_default();
            record_graph_choice(
                edge,
                GraphChoice {
                    syllable_id: lookup.syllable_id,
                    code: lookup.code.to_owned(),
                    spelling_type,
                    correction: lookup.correction,
                    credibility: lookup.credibility,
                },
            );
        }

        let Some(end_vertices) = edges.get_mut(&current_pos) else {
            continue;
        };
        end_vertices.retain(|end_pos, edge| {
            edge.choices.sort_by_key(|choice| choice.syllable_id);
            if edge.choices.is_empty() {
                return false;
            }
            queue.insert((*end_pos, path_type.max(edge.best_non_correction_type)));
            true
        });
    }

    let farthest_type = *vertices.get(&farthest)?;
    let last_type = farthest_type.max(SpellingType::Fuzzy);
    let mut good = BTreeSet::from([farthest]);
    let starts = vertices
        .range(..farthest)
        .map(|(position, _)| *position)
        .rev()
        .collect::<Vec<_>>();
    for start in starts {
        if let Some(end_vertices) = edges.get_mut(&start) {
            end_vertices.retain(|end, edge| {
                if !good.contains(end) {
                    return false;
                }
                edge.choices
                    .retain(|choice| choice.correction || choice.spelling_type <= last_type);
                !edge.choices.is_empty()
            });
        }
        let stale = vertices
            .get(&start)
            .map_or(true, |spelling_type| *spelling_type > last_type)
            || edges.get(&start).map_or(true, BTreeMap::is_empty);
        if stale {
            vertices.remove(&start);
            edges.remove(&start);
        } else {
            good.insert(start);
        }
    }

    let normal_prefix_quality = vertices
        .last_key_value()
        .is_some_and(|(_, spelling_type)| *spelling_type == SpellingType::Normal);

    let mut interpreted_end = farthest;
    if farthest < input.len() {
        let mut completion_choices = Vec::new();
        for lookup in prism.predictive_canonical_codes_with_limit(
            &input[farthest..],
            syllabary_codes,
            PRISM_EXPAND_SEARCH_LIMIT,
        ) {
            if !predictive_completion_type_allowed(lookup.spelling_type) {
                continue;
            }
            merge_graph_choice(
                &mut completion_choices,
                GraphChoice {
                    syllable_id: lookup.syllable_id,
                    code: lookup.code.to_owned(),
                    spelling_type: SpellingType::Completion,
                    correction: lookup.correction,
                    credibility: lookup.credibility + COMPLETION_CREDIBILITY,
                },
            );
        }
        if !completion_choices.is_empty() {
            completion_choices.sort_by_key(|choice| choice.syllable_id);
            edges.entry(farthest).or_default().insert(
                input.len(),
                GraphEdge {
                    choices: completion_choices,
                    best_non_correction_type: SpellingType::Completion,
                },
            );
            interpreted_end = input.len();
        }
    }

    if interpreted_end != input.len() {
        return None;
    }

    let paths = collect_code_paths(
        input.len(),
        &edges,
        MAX_REVERSE_GRAPH_PATH_EXPANSIONS,
        MAX_REVERSE_GRAPH_PATHS,
    )?;
    if paths.is_empty() {
        return None;
    }

    let mut rows = collect_direct_rows(
        &paths,
        entries,
        entries_by_code,
        MAX_REVERSE_GRAPH_DIRECT_ROW_WORK,
    )?;
    rows.sort_by(
        |(left, left_path, left_entry), (right, right_path, right_entry)| {
            right
                .comparison_weight
                .partial_cmp(&left.comparison_weight)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left_path.cmp(right_path))
                .then_with(|| left_entry.cmp(right_entry))
        },
    );
    let mut seen = HashSet::<(String, String)>::new();
    rows.retain(|(row, _, _)| seen.insert((row.entry.code.clone(), row.entry.text.clone())));

    Some(ReverseGraphLookup {
        rows: rows.into_iter().map(|(row, _, _)| row).collect(),
        paths: paths
            .into_iter()
            .map(|(_, _, path_order, segments)| ReverseGraphPath {
                path_order,
                segments,
            })
            .collect(),
        normal_prefix_quality,
    })
}

fn lookup_spelling_type(spelling_type: i32) -> SpellingType {
    match spelling_type {
        0 => SpellingType::Normal,
        1 => SpellingType::Fuzzy,
        2 => SpellingType::Abbreviation,
        3 => SpellingType::Completion,
        4 => SpellingType::Ambiguous,
        _ => SpellingType::Invalid,
    }
}

fn predictive_completion_type_allowed(spelling_type: i32) -> bool {
    (0..2).contains(&spelling_type)
}

fn merge_graph_choice(choices: &mut Vec<GraphChoice>, incoming: GraphChoice) {
    if let Some(existing) = choices
        .iter_mut()
        .find(|choice| choice.syllable_id == incoming.syllable_id)
    {
        if incoming.spelling_type < existing.spelling_type {
            existing.spelling_type = incoming.spelling_type;
        }
        // Syllabifier's SpellingMap keeps the first descriptor's properties
        // and only improves the spelling type for a repeated syllable ID.
        return;
    }
    choices.push(incoming);
}

fn record_graph_choice(edge: &mut GraphEdge, incoming: GraphChoice) {
    if !incoming.correction {
        edge.best_non_correction_type = edge.best_non_correction_type.min(incoming.spelling_type);
    }
    merge_graph_choice(&mut edge.choices, incoming);
}

fn collect_code_paths(
    end: usize,
    edges: &GraphEdges,
    max_expansions: usize,
    max_paths: usize,
) -> Option<Vec<CompletedCodePath>> {
    let mut queue = VecDeque::from([(0usize, String::new(), 0.0f32, Vec::new())]);
    let mut completed = Vec::<(String, f32, usize, Vec<ReverseGraphPathSegment>)>::new();
    let mut expansions = 0usize;

    while let Some((position, prefix, prefix_credibility, segments)) = queue.pop_front() {
        let Some(end_vertices) = edges.get(&position) else {
            continue;
        };
        let transposed = transposed_edge_choices(end_vertices);
        for (next, choice) in transposed {
            expansions = expansions.saturating_add(1);
            if expansions > max_expansions {
                return None;
            }
            let mut code = String::with_capacity(prefix.len() + choice.code.len());
            code.push_str(&prefix);
            code.push_str(&choice.code);
            let credibility = prefix_credibility + choice.credibility;
            let mut next_segments = segments.clone();
            next_segments.push(ReverseGraphPathSegment {
                start: position,
                end: next,
                code: choice.code.clone(),
                credibility: choice.credibility,
            });
            if next == end {
                if completed.len() >= max_paths {
                    return None;
                }
                completed.push((code, credibility, completed.len(), next_segments));
            } else if next < end {
                queue.push_back((next, code, credibility, next_segments));
            }
        }
    }
    Some(completed)
}

fn collect_direct_rows(
    paths: &[CompletedCodePath],
    entries: &[TableEntry],
    entries_by_code: &HashMap<String, Vec<usize>>,
    max_row_work: usize,
) -> Option<Vec<RankedReverseGraphRow>> {
    let mut rows = Vec::new();
    let mut row_work = 0usize;
    for (code, credibility, path_order, _) in paths {
        for entry_order in entries_by_code.get(code).into_iter().flatten().copied() {
            row_work = row_work.saturating_add(1);
            if row_work > max_row_work {
                return None;
            }
            let Some(entry) = entries.get(entry_order) else {
                continue;
            };
            rows.push((
                ReverseGraphRow {
                    entry: entry.clone(),
                    comparison_weight: entry.weight + *credibility,
                },
                *path_order,
                entry_order,
            ));
        }
    }
    Some(rows)
}

fn transposed_edge_choices(
    end_vertices: &BTreeMap<usize, GraphEdge>,
) -> Vec<(usize, &GraphChoice)> {
    let mut by_syllable = BTreeMap::<usize, Vec<(usize, &GraphChoice)>>::new();
    // Syllabifier::Transpose visits longer end positions first, then stores
    // those properties under a syllable-id-ordered index.
    for (end, edge) in end_vertices.iter().rev() {
        for choice in &edge.choices {
            by_syllable
                .entry(choice.syllable_id)
                .or_default()
                .push((*end, choice));
        }
    }
    by_syllable.into_values().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_prism_bin, parse_rime_prism_bin_payload};

    #[test]
    fn exact_prefix_plus_final_completion_matches_reverse_lookup_phrase_family() {
        let syllabary = ["ta", "ka", "kan", "ke", "ken", "kong", "kuai"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 0, 0))
            .expect("test prism should parse")
            .into();
        let entries = vec![
            TableEntry::new("takong", "踏空", 892.0),
            TableEntry::new("take", "塔克", 544.0),
            TableEntry::new("takan", "踏勘", 543.0),
            TableEntry::new("takuai", "他快", 72.0),
            TableEntry::new("taken", "他肯", 1.0),
            TableEntry::new("ta", "他", 10_000.0),
        ];

        let index = index_entries(&entries);
        let lookup = lookup_reverse_graph("tak", &entries, &index, &prism, &syllabary, false, true)
            .expect("ta plus k completion should span the whole input");

        assert!(lookup.normal_prefix_quality);
        assert_eq!(
            lookup
                .rows
                .iter()
                .map(|row| row.entry.text.as_str())
                .collect::<Vec<_>>(),
            ["踏空", "塔克", "踏勘", "他快", "他肯"]
        );
        assert!(lookup
            .rows
            .iter()
            .all(|row| row.comparison_weight < row.entry.weight));
    }

    #[test]
    fn incomplete_graph_without_a_predictive_tail_is_rejected() {
        let syllabary = ["ta"].into_iter().map(str::to_owned).collect::<Vec<_>>();
        let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 0, 0))
            .expect("test prism should parse")
            .into();

        assert!(lookup_reverse_graph(
            "tax",
            &[TableEntry::new("ta", "他", 1.0)],
            &HashMap::from([("ta".to_owned(), vec![0])]),
            &prism,
            &syllabary,
            false,
            true,
        )
        .is_none());
    }

    #[test]
    fn path_expansion_budget_fails_closed_without_recursive_growth() {
        let choice = |syllable_id: usize, code: &str| GraphChoice {
            syllable_id,
            code: code.to_owned(),
            spelling_type: SpellingType::Normal,
            correction: false,
            credibility: 0.0,
        };
        let edges = BTreeMap::from([
            (
                0,
                BTreeMap::from([(
                    1,
                    GraphEdge {
                        choices: vec![choice(0, "a"), choice(1, "b")],
                        best_non_correction_type: SpellingType::Normal,
                    },
                )]),
            ),
            (
                1,
                BTreeMap::from([(
                    2,
                    GraphEdge {
                        choices: vec![choice(2, "c"), choice(3, "d")],
                        best_non_correction_type: SpellingType::Normal,
                    },
                )]),
            ),
        ]);

        assert!(collect_code_paths(2, &edges, 3, 4).is_none());
        assert_eq!(
            collect_code_paths(2, &edges, 6, 4)
                .expect("six edge expansions should fit the declared budget")
                .len(),
            4
        );
        assert!(
            collect_code_paths(2, &edges, 6, 3).is_none(),
            "a complete graph above the accepted-path budget must fail closed"
        );
    }

    #[test]
    fn direct_row_work_budget_fails_closed_without_truncating_a_path_family() {
        let paths = vec![
            ("a".to_owned(), 0.0, 0, Vec::new()),
            ("b".to_owned(), 0.0, 1, Vec::new()),
        ];
        let entries = vec![
            TableEntry::new("a", "a1", 4.0),
            TableEntry::new("a", "a2", 3.0),
            TableEntry::new("b", "b1", 2.0),
            TableEntry::new("b", "b2", 1.0),
        ];
        let index = index_entries(&entries);

        assert!(collect_direct_rows(&paths, &entries, &index, 3).is_none());
        assert_eq!(
            collect_direct_rows(&paths, &entries, &index, 4)
                .expect("the complete direct-row family fits its budget")
                .len(),
            4
        );
    }

    #[test]
    fn predictive_tail_rejects_abbreviation_and_later_spelling_types() {
        assert!(predictive_completion_type_allowed(0));
        assert!(predictive_completion_type_allowed(1));
        for spelling_type in 2..=5 {
            assert!(!predictive_completion_type_allowed(spelling_type));
        }
    }

    #[test]
    fn transposed_paths_visit_longer_end_before_shorter_end() {
        let edge = |syllable_id: usize, code: &str| GraphEdge {
            choices: vec![GraphChoice {
                syllable_id,
                code: code.to_owned(),
                spelling_type: SpellingType::Normal,
                correction: false,
                credibility: 0.0,
            }],
            best_non_correction_type: SpellingType::Normal,
        };
        let edges = BTreeMap::from([
            (0, BTreeMap::from([(1, edge(0, "a")), (2, edge(0, "a"))])),
            (1, BTreeMap::from([(2, edge(1, "b"))])),
        ]);
        assert_eq!(
            collect_code_paths(2, &edges, 16, 16)
                .expect("small graph should stay within its budget")
                .into_iter()
                .map(|(code, _, _, _)| code)
                .collect::<Vec<_>>(),
            ["a", "ab"]
        );
    }

    #[test]
    fn table_query_breadth_first_completion_can_finish_before_queued_lower_id_path() {
        let edge = |syllable_id: usize, code: &str| GraphEdge {
            choices: vec![GraphChoice {
                syllable_id,
                code: code.to_owned(),
                spelling_type: SpellingType::Normal,
                correction: false,
                credibility: 0.0,
            }],
            best_non_correction_type: SpellingType::Normal,
        };
        let edges = BTreeMap::from([
            // The lower syllable ID is visited first, but its continuation is
            // queued. The higher-ID direct path reaches the terminal collector
            // before that queued continuation, matching Table::Query's FIFO.
            (0, BTreeMap::from([(1, edge(0, "a")), (2, edge(1, "b"))])),
            (1, BTreeMap::from([(2, edge(2, "c"))])),
        ]);
        assert_eq!(
            collect_code_paths(2, &edges, 16, 16)
                .expect("small graph should stay within its budget")
                .into_iter()
                .map(|(code, _, _, _)| code)
                .collect::<Vec<_>>(),
            ["b", "ac"]
        );
    }

    #[test]
    fn original_order_dictionary_fails_closed_from_weight_sorted_graph_path() {
        let syllabary = ["ta"].into_iter().map(str::to_owned).collect::<Vec<_>>();
        let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 0, 0))
            .expect("test prism should parse")
            .into();
        let entries = [TableEntry::new("ta", "first", 1.0)];
        assert!(lookup_reverse_graph(
            "ta",
            &entries,
            &index_entries(&entries),
            &prism,
            &syllabary,
            false,
            false,
        )
        .is_none());
    }

    #[test]
    fn equal_weight_completion_paths_follow_syllable_id_not_key_breadth_first_order() {
        let syllabary = ["kao", "ke", "ta"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let prism = parse_rime_prism_bin_payload(build_prism_bin(&syllabary, &[], 0, 0))
            .expect("test prism should parse")
            .into();
        // `ke` is the shorter prism key and is encountered first by
        // ExpandSearch BFS; `kao` has the earlier lexical syllable ID and must
        // be queried first after Syllabifier transposes the graph.
        let entries = vec![
            TableEntry::new("take", "shorter-key", 72.0),
            TableEntry::new("takao", "earlier-syllable-id", 72.0),
        ];
        let lookup = lookup_reverse_graph(
            "tak",
            &entries,
            &index_entries(&entries),
            &prism,
            &syllabary,
            false,
            true,
        )
        .expect("ta plus k completion should produce both equal-weight rows");
        assert_eq!(
            lookup
                .rows
                .iter()
                .map(|row| row.entry.text.as_str())
                .collect::<Vec<_>>(),
            ["earlier-syllable-id", "shorter-key"]
        );
    }

    #[test]
    fn correction_first_duplicate_keeps_first_properties_but_queues_noncorrection_type() {
        let mut edge = GraphEdge::default();
        record_graph_choice(
            &mut edge,
            GraphChoice {
                syllable_id: 7,
                code: "ta".to_owned(),
                spelling_type: SpellingType::Fuzzy,
                correction: true,
                credibility: -4.0,
            },
        );
        record_graph_choice(
            &mut edge,
            GraphChoice {
                syllable_id: 7,
                code: "ta".to_owned(),
                spelling_type: SpellingType::Normal,
                correction: false,
                credibility: 0.0,
            },
        );

        assert_eq!(edge.best_non_correction_type, SpellingType::Normal);
        assert_eq!(edge.choices.len(), 1);
        assert!(edge.choices[0].correction);
        assert_eq!(edge.choices[0].credibility, -4.0);
        assert_eq!(edge.choices[0].spelling_type, SpellingType::Normal);
    }

    fn index_entries(entries: &[TableEntry]) -> HashMap<String, Vec<usize>> {
        let mut index = HashMap::<String, Vec<usize>>::new();
        for (position, entry) in entries.iter().enumerate() {
            index.entry(entry.code.clone()).or_default().push(position);
        }
        index
    }
}
