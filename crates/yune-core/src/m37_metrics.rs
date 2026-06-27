use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    OnceLock,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct M37MetricsSnapshot {
    pub process_key_calls: u64,
    pub process_key_ns: u64,
    pub translator_calls: u64,
    pub translator_ns: u64,
    pub lookup_views_visited: u64,
    pub owned_candidates_materialized: u64,
    pub owned_candidate_materialization_ns: u64,
    pub candidates_sorted: u64,
    pub candidate_sort_ns: u64,
    pub userdb_merge_ns: u64,
    pub filter_pipeline_ns: u64,
    pub ranker_pipeline_ns: u64,
    pub ai_merge_ns: u64,
    pub candidates_stored: u64,
    pub context_full_snapshot_candidates_cloned: u64,
    pub context_page_snapshot_candidates_cloned: u64,
    pub abi_get_context_calls: u64,
    pub abi_get_context_ns: u64,
    pub abi_candidates_exported: u64,
    pub abi_free_context_calls: u64,
    pub abi_free_context_ns: u64,
    pub candidate_request_bounded_calls: u64,
    pub candidate_request_unbounded_calls: u64,
    pub candidate_request_page_limit_total: u64,
    pub candidate_request_surplus_total: u64,
    pub bounded_iterator_calls: u64,
    pub bounded_iterator_limit_total: u64,
    pub bounded_iterator_selected_total: u64,
    pub bounded_iterator_full_count_total: u64,
    pub full_list_translation_calls: u64,
    pub full_list_fallback_count: u64,
    pub exact_lookup_calls: u64,
    pub exact_lookup_ns: u64,
    pub exact_lookup_candidates: u64,
    pub prefix_lookup_calls: u64,
    pub prefix_lookup_ns: u64,
    pub prefix_lookup_candidates: u64,
    pub heap_exact_lookup_calls: u64,
    pub heap_prefix_lookup_calls: u64,
    pub no_marisa_compact_exact_lookup_calls: u64,
    pub no_marisa_compact_prefix_lookup_calls: u64,
    pub rsmarisa_exact_lookup_calls: u64,
    pub rsmarisa_prefix_lookup_calls: u64,
    pub prism_lookup_calls: u64,
    pub prism_lookup_ns: u64,
    pub prism_lookup_codes: u64,
    pub abi_c_string_allocations: u64,
    pub abi_c_string_bytes: u64,
    pub abi_c_string_allocation_ns: u64,
    pub sentence_candidate_calls: u64,
    pub sentence_candidate_ns: u64,
    pub sentence_substrings_considered: u64,
    pub sentence_exact_lookup_calls: u64,
    pub sentence_exact_lookup_ns: u64,
    pub sentence_exact_lookup_candidates: u64,
    pub sentence_prefix_lookup_calls: u64,
    pub sentence_prefix_lookup_ns: u64,
    pub sentence_prefix_lookup_candidates: u64,
    pub sentence_entry_matches_collected: u64,
    pub sentence_path_clones: u64,
    pub sentence_path_replacements: u64,
    pub sentence_paths_pruned: u64,
    pub sentence_max_live_paths: u64,
    pub sentence_result_candidates: u64,
    pub upstream_sentence_model_calls: u64,
    pub upstream_sentence_model_ns: u64,
    pub upstream_sentence_model_candidates: u64,
    pub upstream_sentence_model_code_prefix_checks: u64,
    pub upstream_sentence_model_table_entries_considered: u64,
    pub upstream_sentence_model_vocabulary_entries_considered: u64,
    pub upstream_sentence_model_graph_edges: u64,
    pub upstream_sentence_model_index_build_calls: u64,
    pub upstream_sentence_model_index_build_ns: u64,
    pub upstream_sentence_model_exact_range_index_hits: u64,
    pub upstream_sentence_model_exact_range_index_misses: u64,
    pub upstream_sentence_model_prefix_filter_hits: u64,
    pub upstream_sentence_model_prefix_filter_misses: u64,
    pub upstream_sentence_model_prefix_filter_early_breaks: u64,
    pub upstream_sentence_model_reachable_starts_visited: u64,
    pub upstream_sentence_model_unreachable_starts_skipped: u64,
    pub upstream_sentence_model_phrase_index_walk_calls: u64,
    pub upstream_sentence_model_phrase_index_nodes_visited: u64,
    pub upstream_sentence_model_phrase_index_entry_ranges_emitted: u64,
    pub upstream_sentence_model_partition_point_fallback_calls: u64,
    pub upstream_sentence_model_graph_rebuild_calls: u64,
    pub upstream_sentence_model_graph_rebuild_ns: u64,
    pub upstream_sentence_model_incremental_reuse_hits: u64,
    pub upstream_sentence_model_incremental_extend_ns: u64,
    pub upstream_sentence_model_incremental_discarded_rebuild_chars: u64,
    pub prefix_fallback_calls: u64,
    pub prefix_fallback_ns: u64,
    pub prefix_fallback_views_visited: u64,
    pub prefix_fallback_candidates: u64,
    pub dynamic_correction_calls: u64,
    pub dynamic_correction_ns: u64,
    pub dynamic_correction_codes_considered: u64,
    pub dynamic_correction_candidates: u64,
    pub abbreviation_span_discovery_calls: u64,
    pub abbreviation_span_discovery_ns: u64,
    pub abbreviation_span_candidates_considered: u64,
    pub abbreviation_span_codes_emitted: u64,
    pub abbreviation_model_has_code_calls: u64,
    pub abbreviation_model_has_code_ns: u64,
    pub abbreviation_code_span_graph_build_ns: u64,
    pub abbreviation_sentence_ranking_ns: u64,
    pub abbreviation_preedit_format_ns: u64,
    pub abbreviation_candidate_format_ns: u64,
    pub short_key_candidate_rows_scanned: u64,
    pub short_key_candidates_materialized: u64,
    pub short_key_candidates_cloned: u64,
    pub short_key_filter_ns: u64,
    pub short_key_sort_rank_ns: u64,
    pub short_key_comment_quality_ns: u64,
    pub short_key_first_page_materialize_ns: u64,
    pub track_b_spelling_expansions_considered: u64,
    pub track_b_spelling_expansion_ns: u64,
    pub track_b_exact_lookup_calls: u64,
    pub track_b_exact_lookup_ns: u64,
    pub track_b_prefix_lookup_calls: u64,
    pub track_b_prefix_lookup_ns: u64,
    pub track_b_candidates_materialized: u64,
    pub track_b_first_page_materialize_ns: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct M37SentenceCandidateMetrics {
    pub duration: Duration,
    pub result_candidates: usize,
    pub substrings_considered: usize,
    pub exact_lookup_calls: usize,
    pub exact_lookup_ns: Duration,
    pub exact_lookup_candidates: usize,
    pub prefix_lookup_calls: usize,
    pub prefix_lookup_ns: Duration,
    pub prefix_lookup_candidates: usize,
    pub entry_matches_collected: usize,
    pub path_clones: usize,
    pub path_replacements: usize,
    pub paths_pruned: usize,
    pub max_live_paths: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct M40SentenceLookupMetrics {
    pub exact_range_index_hits: usize,
    pub exact_range_index_misses: usize,
    pub prefix_filter_hits: usize,
    pub prefix_filter_misses: usize,
    pub prefix_filter_early_breaks: usize,
    pub reachable_starts_visited: usize,
    pub unreachable_starts_skipped: usize,
    pub phrase_index_walk_calls: usize,
    pub phrase_index_nodes_visited: usize,
    pub phrase_index_entry_ranges_emitted: usize,
    pub partition_point_fallback_calls: usize,
    pub graph_rebuild_duration: Duration,
    pub incremental_reuse_hits: usize,
    pub incremental_extend_duration: Duration,
    pub incremental_discarded_rebuild_chars: usize,
}

#[derive(Default)]
struct M37Metrics {
    enabled: AtomicBool,
    process_key_calls: AtomicU64,
    process_key_ns: AtomicU64,
    translator_calls: AtomicU64,
    translator_ns: AtomicU64,
    lookup_views_visited: AtomicU64,
    owned_candidates_materialized: AtomicU64,
    owned_candidate_materialization_ns: AtomicU64,
    candidates_sorted: AtomicU64,
    candidate_sort_ns: AtomicU64,
    userdb_merge_ns: AtomicU64,
    filter_pipeline_ns: AtomicU64,
    ranker_pipeline_ns: AtomicU64,
    ai_merge_ns: AtomicU64,
    candidates_stored: AtomicU64,
    context_full_snapshot_candidates_cloned: AtomicU64,
    context_page_snapshot_candidates_cloned: AtomicU64,
    abi_get_context_calls: AtomicU64,
    abi_get_context_ns: AtomicU64,
    abi_candidates_exported: AtomicU64,
    abi_free_context_calls: AtomicU64,
    abi_free_context_ns: AtomicU64,
    candidate_request_bounded_calls: AtomicU64,
    candidate_request_unbounded_calls: AtomicU64,
    candidate_request_page_limit_total: AtomicU64,
    candidate_request_surplus_total: AtomicU64,
    bounded_iterator_calls: AtomicU64,
    bounded_iterator_limit_total: AtomicU64,
    bounded_iterator_selected_total: AtomicU64,
    bounded_iterator_full_count_total: AtomicU64,
    full_list_translation_calls: AtomicU64,
    full_list_fallback_count: AtomicU64,
    exact_lookup_calls: AtomicU64,
    exact_lookup_ns: AtomicU64,
    exact_lookup_candidates: AtomicU64,
    prefix_lookup_calls: AtomicU64,
    prefix_lookup_ns: AtomicU64,
    prefix_lookup_candidates: AtomicU64,
    heap_exact_lookup_calls: AtomicU64,
    heap_prefix_lookup_calls: AtomicU64,
    no_marisa_compact_exact_lookup_calls: AtomicU64,
    no_marisa_compact_prefix_lookup_calls: AtomicU64,
    rsmarisa_exact_lookup_calls: AtomicU64,
    rsmarisa_prefix_lookup_calls: AtomicU64,
    prism_lookup_calls: AtomicU64,
    prism_lookup_ns: AtomicU64,
    prism_lookup_codes: AtomicU64,
    abi_c_string_allocations: AtomicU64,
    abi_c_string_bytes: AtomicU64,
    abi_c_string_allocation_ns: AtomicU64,
    sentence_candidate_calls: AtomicU64,
    sentence_candidate_ns: AtomicU64,
    sentence_substrings_considered: AtomicU64,
    sentence_exact_lookup_calls: AtomicU64,
    sentence_exact_lookup_ns: AtomicU64,
    sentence_exact_lookup_candidates: AtomicU64,
    sentence_prefix_lookup_calls: AtomicU64,
    sentence_prefix_lookup_ns: AtomicU64,
    sentence_prefix_lookup_candidates: AtomicU64,
    sentence_entry_matches_collected: AtomicU64,
    sentence_path_clones: AtomicU64,
    sentence_path_replacements: AtomicU64,
    sentence_paths_pruned: AtomicU64,
    sentence_max_live_paths: AtomicU64,
    sentence_result_candidates: AtomicU64,
    upstream_sentence_model_calls: AtomicU64,
    upstream_sentence_model_ns: AtomicU64,
    upstream_sentence_model_candidates: AtomicU64,
    upstream_sentence_model_code_prefix_checks: AtomicU64,
    upstream_sentence_model_table_entries_considered: AtomicU64,
    upstream_sentence_model_vocabulary_entries_considered: AtomicU64,
    upstream_sentence_model_graph_edges: AtomicU64,
    upstream_sentence_model_index_build_calls: AtomicU64,
    upstream_sentence_model_index_build_ns: AtomicU64,
    upstream_sentence_model_exact_range_index_hits: AtomicU64,
    upstream_sentence_model_exact_range_index_misses: AtomicU64,
    upstream_sentence_model_prefix_filter_hits: AtomicU64,
    upstream_sentence_model_prefix_filter_misses: AtomicU64,
    upstream_sentence_model_prefix_filter_early_breaks: AtomicU64,
    upstream_sentence_model_reachable_starts_visited: AtomicU64,
    upstream_sentence_model_unreachable_starts_skipped: AtomicU64,
    upstream_sentence_model_phrase_index_walk_calls: AtomicU64,
    upstream_sentence_model_phrase_index_nodes_visited: AtomicU64,
    upstream_sentence_model_phrase_index_entry_ranges_emitted: AtomicU64,
    upstream_sentence_model_partition_point_fallback_calls: AtomicU64,
    upstream_sentence_model_graph_rebuild_calls: AtomicU64,
    upstream_sentence_model_graph_rebuild_ns: AtomicU64,
    upstream_sentence_model_incremental_reuse_hits: AtomicU64,
    upstream_sentence_model_incremental_extend_ns: AtomicU64,
    upstream_sentence_model_incremental_discarded_rebuild_chars: AtomicU64,
    prefix_fallback_calls: AtomicU64,
    prefix_fallback_ns: AtomicU64,
    prefix_fallback_views_visited: AtomicU64,
    prefix_fallback_candidates: AtomicU64,
    dynamic_correction_calls: AtomicU64,
    dynamic_correction_ns: AtomicU64,
    dynamic_correction_codes_considered: AtomicU64,
    dynamic_correction_candidates: AtomicU64,
    abbreviation_span_discovery_calls: AtomicU64,
    abbreviation_span_discovery_ns: AtomicU64,
    abbreviation_span_candidates_considered: AtomicU64,
    abbreviation_span_codes_emitted: AtomicU64,
    abbreviation_model_has_code_calls: AtomicU64,
    abbreviation_model_has_code_ns: AtomicU64,
    abbreviation_code_span_graph_build_ns: AtomicU64,
    abbreviation_sentence_ranking_ns: AtomicU64,
    abbreviation_preedit_format_ns: AtomicU64,
    abbreviation_candidate_format_ns: AtomicU64,
    short_key_candidate_rows_scanned: AtomicU64,
    short_key_candidates_materialized: AtomicU64,
    short_key_candidates_cloned: AtomicU64,
    short_key_filter_ns: AtomicU64,
    short_key_sort_rank_ns: AtomicU64,
    short_key_comment_quality_ns: AtomicU64,
    short_key_first_page_materialize_ns: AtomicU64,
    track_b_spelling_expansions_considered: AtomicU64,
    track_b_spelling_expansion_ns: AtomicU64,
    track_b_exact_lookup_calls: AtomicU64,
    track_b_exact_lookup_ns: AtomicU64,
    track_b_prefix_lookup_calls: AtomicU64,
    track_b_prefix_lookup_ns: AtomicU64,
    track_b_candidates_materialized: AtomicU64,
    track_b_first_page_materialize_ns: AtomicU64,
}

fn metrics() -> &'static M37Metrics {
    static METRICS: OnceLock<M37Metrics> = OnceLock::new();
    METRICS.get_or_init(M37Metrics::default)
}

#[must_use]
pub fn m37_metrics_enabled() -> bool {
    metrics().enabled.load(Ordering::Relaxed)
}

pub fn m37_metrics_enable(enabled: bool) {
    metrics().enabled.store(enabled, Ordering::Relaxed);
}

pub fn m37_metrics_reset() {
    let metrics = metrics();
    metrics.process_key_calls.store(0, Ordering::Relaxed);
    metrics.process_key_ns.store(0, Ordering::Relaxed);
    metrics.translator_calls.store(0, Ordering::Relaxed);
    metrics.translator_ns.store(0, Ordering::Relaxed);
    metrics.lookup_views_visited.store(0, Ordering::Relaxed);
    metrics
        .owned_candidates_materialized
        .store(0, Ordering::Relaxed);
    metrics
        .owned_candidate_materialization_ns
        .store(0, Ordering::Relaxed);
    metrics.candidates_sorted.store(0, Ordering::Relaxed);
    metrics.candidate_sort_ns.store(0, Ordering::Relaxed);
    metrics.userdb_merge_ns.store(0, Ordering::Relaxed);
    metrics.filter_pipeline_ns.store(0, Ordering::Relaxed);
    metrics.ranker_pipeline_ns.store(0, Ordering::Relaxed);
    metrics.ai_merge_ns.store(0, Ordering::Relaxed);
    metrics.candidates_stored.store(0, Ordering::Relaxed);
    metrics
        .context_full_snapshot_candidates_cloned
        .store(0, Ordering::Relaxed);
    metrics
        .context_page_snapshot_candidates_cloned
        .store(0, Ordering::Relaxed);
    metrics.abi_get_context_calls.store(0, Ordering::Relaxed);
    metrics.abi_get_context_ns.store(0, Ordering::Relaxed);
    metrics.abi_candidates_exported.store(0, Ordering::Relaxed);
    metrics.abi_free_context_calls.store(0, Ordering::Relaxed);
    metrics.abi_free_context_ns.store(0, Ordering::Relaxed);
    metrics
        .candidate_request_bounded_calls
        .store(0, Ordering::Relaxed);
    metrics
        .candidate_request_unbounded_calls
        .store(0, Ordering::Relaxed);
    metrics
        .candidate_request_page_limit_total
        .store(0, Ordering::Relaxed);
    metrics
        .candidate_request_surplus_total
        .store(0, Ordering::Relaxed);
    metrics.bounded_iterator_calls.store(0, Ordering::Relaxed);
    metrics
        .bounded_iterator_limit_total
        .store(0, Ordering::Relaxed);
    metrics
        .bounded_iterator_selected_total
        .store(0, Ordering::Relaxed);
    metrics
        .bounded_iterator_full_count_total
        .store(0, Ordering::Relaxed);
    metrics
        .full_list_translation_calls
        .store(0, Ordering::Relaxed);
    metrics.full_list_fallback_count.store(0, Ordering::Relaxed);
    metrics.exact_lookup_calls.store(0, Ordering::Relaxed);
    metrics.exact_lookup_ns.store(0, Ordering::Relaxed);
    metrics.exact_lookup_candidates.store(0, Ordering::Relaxed);
    metrics.prefix_lookup_calls.store(0, Ordering::Relaxed);
    metrics.prefix_lookup_ns.store(0, Ordering::Relaxed);
    metrics.prefix_lookup_candidates.store(0, Ordering::Relaxed);
    metrics.heap_exact_lookup_calls.store(0, Ordering::Relaxed);
    metrics.heap_prefix_lookup_calls.store(0, Ordering::Relaxed);
    metrics
        .no_marisa_compact_exact_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics
        .no_marisa_compact_prefix_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics
        .rsmarisa_exact_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics
        .rsmarisa_prefix_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics.prism_lookup_calls.store(0, Ordering::Relaxed);
    metrics.prism_lookup_ns.store(0, Ordering::Relaxed);
    metrics.prism_lookup_codes.store(0, Ordering::Relaxed);
    metrics.abi_c_string_allocations.store(0, Ordering::Relaxed);
    metrics.abi_c_string_bytes.store(0, Ordering::Relaxed);
    metrics
        .abi_c_string_allocation_ns
        .store(0, Ordering::Relaxed);
    metrics.sentence_candidate_calls.store(0, Ordering::Relaxed);
    metrics.sentence_candidate_ns.store(0, Ordering::Relaxed);
    metrics
        .sentence_substrings_considered
        .store(0, Ordering::Relaxed);
    metrics
        .sentence_exact_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics.sentence_exact_lookup_ns.store(0, Ordering::Relaxed);
    metrics
        .sentence_exact_lookup_candidates
        .store(0, Ordering::Relaxed);
    metrics
        .sentence_prefix_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics
        .sentence_prefix_lookup_ns
        .store(0, Ordering::Relaxed);
    metrics
        .sentence_prefix_lookup_candidates
        .store(0, Ordering::Relaxed);
    metrics
        .sentence_entry_matches_collected
        .store(0, Ordering::Relaxed);
    metrics.sentence_path_clones.store(0, Ordering::Relaxed);
    metrics
        .sentence_path_replacements
        .store(0, Ordering::Relaxed);
    metrics.sentence_paths_pruned.store(0, Ordering::Relaxed);
    metrics.sentence_max_live_paths.store(0, Ordering::Relaxed);
    metrics
        .sentence_result_candidates
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_calls
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_ns
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_candidates
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_code_prefix_checks
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_table_entries_considered
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_vocabulary_entries_considered
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_graph_edges
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_index_build_calls
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_index_build_ns
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_exact_range_index_hits
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_exact_range_index_misses
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_prefix_filter_hits
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_prefix_filter_misses
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_prefix_filter_early_breaks
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_reachable_starts_visited
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_unreachable_starts_skipped
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_phrase_index_walk_calls
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_phrase_index_nodes_visited
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_phrase_index_entry_ranges_emitted
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_partition_point_fallback_calls
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_graph_rebuild_calls
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_graph_rebuild_ns
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_incremental_reuse_hits
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_incremental_extend_ns
        .store(0, Ordering::Relaxed);
    metrics
        .upstream_sentence_model_incremental_discarded_rebuild_chars
        .store(0, Ordering::Relaxed);
    metrics.prefix_fallback_calls.store(0, Ordering::Relaxed);
    metrics.prefix_fallback_ns.store(0, Ordering::Relaxed);
    metrics
        .prefix_fallback_views_visited
        .store(0, Ordering::Relaxed);
    metrics
        .prefix_fallback_candidates
        .store(0, Ordering::Relaxed);
    metrics.dynamic_correction_calls.store(0, Ordering::Relaxed);
    metrics.dynamic_correction_ns.store(0, Ordering::Relaxed);
    metrics
        .dynamic_correction_codes_considered
        .store(0, Ordering::Relaxed);
    metrics
        .dynamic_correction_candidates
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_span_discovery_calls
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_span_discovery_ns
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_span_candidates_considered
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_span_codes_emitted
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_model_has_code_calls
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_model_has_code_ns
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_code_span_graph_build_ns
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_sentence_ranking_ns
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_preedit_format_ns
        .store(0, Ordering::Relaxed);
    metrics
        .abbreviation_candidate_format_ns
        .store(0, Ordering::Relaxed);
    metrics
        .short_key_candidate_rows_scanned
        .store(0, Ordering::Relaxed);
    metrics
        .short_key_candidates_materialized
        .store(0, Ordering::Relaxed);
    metrics
        .short_key_candidates_cloned
        .store(0, Ordering::Relaxed);
    metrics.short_key_filter_ns.store(0, Ordering::Relaxed);
    metrics.short_key_sort_rank_ns.store(0, Ordering::Relaxed);
    metrics
        .short_key_comment_quality_ns
        .store(0, Ordering::Relaxed);
    metrics
        .short_key_first_page_materialize_ns
        .store(0, Ordering::Relaxed);
    metrics
        .track_b_spelling_expansions_considered
        .store(0, Ordering::Relaxed);
    metrics
        .track_b_spelling_expansion_ns
        .store(0, Ordering::Relaxed);
    metrics
        .track_b_exact_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics.track_b_exact_lookup_ns.store(0, Ordering::Relaxed);
    metrics
        .track_b_prefix_lookup_calls
        .store(0, Ordering::Relaxed);
    metrics.track_b_prefix_lookup_ns.store(0, Ordering::Relaxed);
    metrics
        .track_b_candidates_materialized
        .store(0, Ordering::Relaxed);
    metrics
        .track_b_first_page_materialize_ns
        .store(0, Ordering::Relaxed);
}

#[must_use]
pub fn m37_metrics_snapshot() -> M37MetricsSnapshot {
    let metrics = metrics();
    M37MetricsSnapshot {
        process_key_calls: metrics.process_key_calls.load(Ordering::Relaxed),
        process_key_ns: metrics.process_key_ns.load(Ordering::Relaxed),
        translator_calls: metrics.translator_calls.load(Ordering::Relaxed),
        translator_ns: metrics.translator_ns.load(Ordering::Relaxed),
        lookup_views_visited: metrics.lookup_views_visited.load(Ordering::Relaxed),
        owned_candidates_materialized: metrics
            .owned_candidates_materialized
            .load(Ordering::Relaxed),
        owned_candidate_materialization_ns: metrics
            .owned_candidate_materialization_ns
            .load(Ordering::Relaxed),
        candidates_sorted: metrics.candidates_sorted.load(Ordering::Relaxed),
        candidate_sort_ns: metrics.candidate_sort_ns.load(Ordering::Relaxed),
        userdb_merge_ns: metrics.userdb_merge_ns.load(Ordering::Relaxed),
        filter_pipeline_ns: metrics.filter_pipeline_ns.load(Ordering::Relaxed),
        ranker_pipeline_ns: metrics.ranker_pipeline_ns.load(Ordering::Relaxed),
        ai_merge_ns: metrics.ai_merge_ns.load(Ordering::Relaxed),
        candidates_stored: metrics.candidates_stored.load(Ordering::Relaxed),
        context_full_snapshot_candidates_cloned: metrics
            .context_full_snapshot_candidates_cloned
            .load(Ordering::Relaxed),
        context_page_snapshot_candidates_cloned: metrics
            .context_page_snapshot_candidates_cloned
            .load(Ordering::Relaxed),
        abi_get_context_calls: metrics.abi_get_context_calls.load(Ordering::Relaxed),
        abi_get_context_ns: metrics.abi_get_context_ns.load(Ordering::Relaxed),
        abi_candidates_exported: metrics.abi_candidates_exported.load(Ordering::Relaxed),
        abi_free_context_calls: metrics.abi_free_context_calls.load(Ordering::Relaxed),
        abi_free_context_ns: metrics.abi_free_context_ns.load(Ordering::Relaxed),
        candidate_request_bounded_calls: metrics
            .candidate_request_bounded_calls
            .load(Ordering::Relaxed),
        candidate_request_unbounded_calls: metrics
            .candidate_request_unbounded_calls
            .load(Ordering::Relaxed),
        candidate_request_page_limit_total: metrics
            .candidate_request_page_limit_total
            .load(Ordering::Relaxed),
        candidate_request_surplus_total: metrics
            .candidate_request_surplus_total
            .load(Ordering::Relaxed),
        bounded_iterator_calls: metrics.bounded_iterator_calls.load(Ordering::Relaxed),
        bounded_iterator_limit_total: metrics.bounded_iterator_limit_total.load(Ordering::Relaxed),
        bounded_iterator_selected_total: metrics
            .bounded_iterator_selected_total
            .load(Ordering::Relaxed),
        bounded_iterator_full_count_total: metrics
            .bounded_iterator_full_count_total
            .load(Ordering::Relaxed),
        full_list_translation_calls: metrics.full_list_translation_calls.load(Ordering::Relaxed),
        full_list_fallback_count: metrics.full_list_fallback_count.load(Ordering::Relaxed),
        exact_lookup_calls: metrics.exact_lookup_calls.load(Ordering::Relaxed),
        exact_lookup_ns: metrics.exact_lookup_ns.load(Ordering::Relaxed),
        exact_lookup_candidates: metrics.exact_lookup_candidates.load(Ordering::Relaxed),
        prefix_lookup_calls: metrics.prefix_lookup_calls.load(Ordering::Relaxed),
        prefix_lookup_ns: metrics.prefix_lookup_ns.load(Ordering::Relaxed),
        prefix_lookup_candidates: metrics.prefix_lookup_candidates.load(Ordering::Relaxed),
        heap_exact_lookup_calls: metrics.heap_exact_lookup_calls.load(Ordering::Relaxed),
        heap_prefix_lookup_calls: metrics.heap_prefix_lookup_calls.load(Ordering::Relaxed),
        no_marisa_compact_exact_lookup_calls: metrics
            .no_marisa_compact_exact_lookup_calls
            .load(Ordering::Relaxed),
        no_marisa_compact_prefix_lookup_calls: metrics
            .no_marisa_compact_prefix_lookup_calls
            .load(Ordering::Relaxed),
        rsmarisa_exact_lookup_calls: metrics.rsmarisa_exact_lookup_calls.load(Ordering::Relaxed),
        rsmarisa_prefix_lookup_calls: metrics.rsmarisa_prefix_lookup_calls.load(Ordering::Relaxed),
        prism_lookup_calls: metrics.prism_lookup_calls.load(Ordering::Relaxed),
        prism_lookup_ns: metrics.prism_lookup_ns.load(Ordering::Relaxed),
        prism_lookup_codes: metrics.prism_lookup_codes.load(Ordering::Relaxed),
        abi_c_string_allocations: metrics.abi_c_string_allocations.load(Ordering::Relaxed),
        abi_c_string_bytes: metrics.abi_c_string_bytes.load(Ordering::Relaxed),
        abi_c_string_allocation_ns: metrics.abi_c_string_allocation_ns.load(Ordering::Relaxed),
        sentence_candidate_calls: metrics.sentence_candidate_calls.load(Ordering::Relaxed),
        sentence_candidate_ns: metrics.sentence_candidate_ns.load(Ordering::Relaxed),
        sentence_substrings_considered: metrics
            .sentence_substrings_considered
            .load(Ordering::Relaxed),
        sentence_exact_lookup_calls: metrics.sentence_exact_lookup_calls.load(Ordering::Relaxed),
        sentence_exact_lookup_ns: metrics.sentence_exact_lookup_ns.load(Ordering::Relaxed),
        sentence_exact_lookup_candidates: metrics
            .sentence_exact_lookup_candidates
            .load(Ordering::Relaxed),
        sentence_prefix_lookup_calls: metrics.sentence_prefix_lookup_calls.load(Ordering::Relaxed),
        sentence_prefix_lookup_ns: metrics.sentence_prefix_lookup_ns.load(Ordering::Relaxed),
        sentence_prefix_lookup_candidates: metrics
            .sentence_prefix_lookup_candidates
            .load(Ordering::Relaxed),
        sentence_entry_matches_collected: metrics
            .sentence_entry_matches_collected
            .load(Ordering::Relaxed),
        sentence_path_clones: metrics.sentence_path_clones.load(Ordering::Relaxed),
        sentence_path_replacements: metrics.sentence_path_replacements.load(Ordering::Relaxed),
        sentence_paths_pruned: metrics.sentence_paths_pruned.load(Ordering::Relaxed),
        sentence_max_live_paths: metrics.sentence_max_live_paths.load(Ordering::Relaxed),
        sentence_result_candidates: metrics.sentence_result_candidates.load(Ordering::Relaxed),
        upstream_sentence_model_calls: metrics
            .upstream_sentence_model_calls
            .load(Ordering::Relaxed),
        upstream_sentence_model_ns: metrics.upstream_sentence_model_ns.load(Ordering::Relaxed),
        upstream_sentence_model_candidates: metrics
            .upstream_sentence_model_candidates
            .load(Ordering::Relaxed),
        upstream_sentence_model_code_prefix_checks: metrics
            .upstream_sentence_model_code_prefix_checks
            .load(Ordering::Relaxed),
        upstream_sentence_model_table_entries_considered: metrics
            .upstream_sentence_model_table_entries_considered
            .load(Ordering::Relaxed),
        upstream_sentence_model_vocabulary_entries_considered: metrics
            .upstream_sentence_model_vocabulary_entries_considered
            .load(Ordering::Relaxed),
        upstream_sentence_model_graph_edges: metrics
            .upstream_sentence_model_graph_edges
            .load(Ordering::Relaxed),
        upstream_sentence_model_index_build_calls: metrics
            .upstream_sentence_model_index_build_calls
            .load(Ordering::Relaxed),
        upstream_sentence_model_index_build_ns: metrics
            .upstream_sentence_model_index_build_ns
            .load(Ordering::Relaxed),
        upstream_sentence_model_exact_range_index_hits: metrics
            .upstream_sentence_model_exact_range_index_hits
            .load(Ordering::Relaxed),
        upstream_sentence_model_exact_range_index_misses: metrics
            .upstream_sentence_model_exact_range_index_misses
            .load(Ordering::Relaxed),
        upstream_sentence_model_prefix_filter_hits: metrics
            .upstream_sentence_model_prefix_filter_hits
            .load(Ordering::Relaxed),
        upstream_sentence_model_prefix_filter_misses: metrics
            .upstream_sentence_model_prefix_filter_misses
            .load(Ordering::Relaxed),
        upstream_sentence_model_prefix_filter_early_breaks: metrics
            .upstream_sentence_model_prefix_filter_early_breaks
            .load(Ordering::Relaxed),
        upstream_sentence_model_reachable_starts_visited: metrics
            .upstream_sentence_model_reachable_starts_visited
            .load(Ordering::Relaxed),
        upstream_sentence_model_unreachable_starts_skipped: metrics
            .upstream_sentence_model_unreachable_starts_skipped
            .load(Ordering::Relaxed),
        upstream_sentence_model_phrase_index_walk_calls: metrics
            .upstream_sentence_model_phrase_index_walk_calls
            .load(Ordering::Relaxed),
        upstream_sentence_model_phrase_index_nodes_visited: metrics
            .upstream_sentence_model_phrase_index_nodes_visited
            .load(Ordering::Relaxed),
        upstream_sentence_model_phrase_index_entry_ranges_emitted: metrics
            .upstream_sentence_model_phrase_index_entry_ranges_emitted
            .load(Ordering::Relaxed),
        upstream_sentence_model_partition_point_fallback_calls: metrics
            .upstream_sentence_model_partition_point_fallback_calls
            .load(Ordering::Relaxed),
        upstream_sentence_model_graph_rebuild_calls: metrics
            .upstream_sentence_model_graph_rebuild_calls
            .load(Ordering::Relaxed),
        upstream_sentence_model_graph_rebuild_ns: metrics
            .upstream_sentence_model_graph_rebuild_ns
            .load(Ordering::Relaxed),
        upstream_sentence_model_incremental_reuse_hits: metrics
            .upstream_sentence_model_incremental_reuse_hits
            .load(Ordering::Relaxed),
        upstream_sentence_model_incremental_extend_ns: metrics
            .upstream_sentence_model_incremental_extend_ns
            .load(Ordering::Relaxed),
        upstream_sentence_model_incremental_discarded_rebuild_chars: metrics
            .upstream_sentence_model_incremental_discarded_rebuild_chars
            .load(Ordering::Relaxed),
        prefix_fallback_calls: metrics.prefix_fallback_calls.load(Ordering::Relaxed),
        prefix_fallback_ns: metrics.prefix_fallback_ns.load(Ordering::Relaxed),
        prefix_fallback_views_visited: metrics
            .prefix_fallback_views_visited
            .load(Ordering::Relaxed),
        prefix_fallback_candidates: metrics.prefix_fallback_candidates.load(Ordering::Relaxed),
        dynamic_correction_calls: metrics.dynamic_correction_calls.load(Ordering::Relaxed),
        dynamic_correction_ns: metrics.dynamic_correction_ns.load(Ordering::Relaxed),
        dynamic_correction_codes_considered: metrics
            .dynamic_correction_codes_considered
            .load(Ordering::Relaxed),
        dynamic_correction_candidates: metrics
            .dynamic_correction_candidates
            .load(Ordering::Relaxed),
        abbreviation_span_discovery_calls: metrics
            .abbreviation_span_discovery_calls
            .load(Ordering::Relaxed),
        abbreviation_span_discovery_ns: metrics
            .abbreviation_span_discovery_ns
            .load(Ordering::Relaxed),
        abbreviation_span_candidates_considered: metrics
            .abbreviation_span_candidates_considered
            .load(Ordering::Relaxed),
        abbreviation_span_codes_emitted: metrics
            .abbreviation_span_codes_emitted
            .load(Ordering::Relaxed),
        abbreviation_model_has_code_calls: metrics
            .abbreviation_model_has_code_calls
            .load(Ordering::Relaxed),
        abbreviation_model_has_code_ns: metrics
            .abbreviation_model_has_code_ns
            .load(Ordering::Relaxed),
        abbreviation_code_span_graph_build_ns: metrics
            .abbreviation_code_span_graph_build_ns
            .load(Ordering::Relaxed),
        abbreviation_sentence_ranking_ns: metrics
            .abbreviation_sentence_ranking_ns
            .load(Ordering::Relaxed),
        abbreviation_preedit_format_ns: metrics
            .abbreviation_preedit_format_ns
            .load(Ordering::Relaxed),
        abbreviation_candidate_format_ns: metrics
            .abbreviation_candidate_format_ns
            .load(Ordering::Relaxed),
        short_key_candidate_rows_scanned: metrics
            .short_key_candidate_rows_scanned
            .load(Ordering::Relaxed),
        short_key_candidates_materialized: metrics
            .short_key_candidates_materialized
            .load(Ordering::Relaxed),
        short_key_candidates_cloned: metrics.short_key_candidates_cloned.load(Ordering::Relaxed),
        short_key_filter_ns: metrics.short_key_filter_ns.load(Ordering::Relaxed),
        short_key_sort_rank_ns: metrics.short_key_sort_rank_ns.load(Ordering::Relaxed),
        short_key_comment_quality_ns: metrics.short_key_comment_quality_ns.load(Ordering::Relaxed),
        short_key_first_page_materialize_ns: metrics
            .short_key_first_page_materialize_ns
            .load(Ordering::Relaxed),
        track_b_spelling_expansions_considered: metrics
            .track_b_spelling_expansions_considered
            .load(Ordering::Relaxed),
        track_b_spelling_expansion_ns: metrics
            .track_b_spelling_expansion_ns
            .load(Ordering::Relaxed),
        track_b_exact_lookup_calls: metrics.track_b_exact_lookup_calls.load(Ordering::Relaxed),
        track_b_exact_lookup_ns: metrics.track_b_exact_lookup_ns.load(Ordering::Relaxed),
        track_b_prefix_lookup_calls: metrics.track_b_prefix_lookup_calls.load(Ordering::Relaxed),
        track_b_prefix_lookup_ns: metrics.track_b_prefix_lookup_ns.load(Ordering::Relaxed),
        track_b_candidates_materialized: metrics
            .track_b_candidates_materialized
            .load(Ordering::Relaxed),
        track_b_first_page_materialize_ns: metrics
            .track_b_first_page_materialize_ns
            .load(Ordering::Relaxed),
    }
}

fn add(counter: &AtomicU64, value: u64) {
    if value != 0 && m37_metrics_enabled() {
        counter.fetch_add(value, Ordering::Relaxed);
    }
}

fn add_duration(counter: &AtomicU64, duration: Duration) {
    add(
        counter,
        duration.as_nanos().min(u128::from(u64::MAX)) as u64,
    );
}

fn max(counter: &AtomicU64, value: u64) {
    if value != 0 && m37_metrics_enabled() {
        counter.fetch_max(value, Ordering::Relaxed);
    }
}

pub fn m37_record_process_key(duration: Duration) {
    if m37_metrics_enabled() {
        metrics().process_key_calls.fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics().process_key_ns, duration);
    }
}

pub fn m37_record_translator(duration: Duration) {
    if m37_metrics_enabled() {
        metrics().translator_calls.fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics().translator_ns, duration);
    }
}

pub fn m37_record_lookup_view() {
    add(&metrics().lookup_views_visited, 1);
}

pub fn m37_record_owned_candidate_materialized() {
    add(&metrics().owned_candidates_materialized, 1);
}

pub fn m37_record_owned_candidate_materialization(duration: Duration) {
    if m37_metrics_enabled() {
        metrics()
            .owned_candidates_materialized
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics().owned_candidate_materialization_ns, duration);
    }
}

pub fn m37_record_candidates_sorted(count: usize) {
    add(&metrics().candidates_sorted, count as u64);
}

pub fn m37_record_candidate_sort(duration: Duration) {
    add_duration(&metrics().candidate_sort_ns, duration);
}

pub fn m37_record_userdb_merge(duration: Duration) {
    add_duration(&metrics().userdb_merge_ns, duration);
}

pub fn m37_record_filter_pipeline(duration: Duration) {
    add_duration(&metrics().filter_pipeline_ns, duration);
}

pub fn m37_record_ranker_pipeline(duration: Duration) {
    add_duration(&metrics().ranker_pipeline_ns, duration);
}

pub fn m37_record_ai_merge(duration: Duration) {
    add_duration(&metrics().ai_merge_ns, duration);
}

pub fn m37_record_candidates_stored(count: usize) {
    add(&metrics().candidates_stored, count as u64);
}

pub fn m37_record_context_full_snapshot_clone(count: usize) {
    add(
        &metrics().context_full_snapshot_candidates_cloned,
        count as u64,
    );
}

pub fn m37_record_context_page_snapshot_clone(count: usize) {
    add(
        &metrics().context_page_snapshot_candidates_cloned,
        count as u64,
    );
}

pub fn m37_record_abi_get_context(duration: Duration) {
    if m37_metrics_enabled() {
        metrics()
            .abi_get_context_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics().abi_get_context_ns, duration);
    }
}

pub fn m37_record_abi_candidates_exported(count: usize) {
    add(&metrics().abi_candidates_exported, count as u64);
}

pub fn m37_record_abi_free_context(duration: Duration) {
    if m37_metrics_enabled() {
        metrics()
            .abi_free_context_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics().abi_free_context_ns, duration);
    }
}

pub fn m37_record_candidate_request_bounded(page_limit: usize, surplus: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .candidate_request_bounded_calls
            .fetch_add(1, Ordering::Relaxed);
        add(
            &metrics.candidate_request_page_limit_total,
            page_limit as u64,
        );
        add(&metrics.candidate_request_surplus_total, surplus as u64);
    }
}

pub fn m37_record_candidate_request_unbounded() {
    if m37_metrics_enabled() {
        metrics()
            .candidate_request_unbounded_calls
            .fetch_add(1, Ordering::Relaxed);
    }
}

pub fn m37_record_bounded_iterator(limit: usize, selected: usize, full_count: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .bounded_iterator_calls
            .fetch_add(1, Ordering::Relaxed);
        add(&metrics.bounded_iterator_limit_total, limit as u64);
        add(&metrics.bounded_iterator_selected_total, selected as u64);
        add(
            &metrics.bounded_iterator_full_count_total,
            full_count as u64,
        );
    }
}

pub fn m37_record_full_list_translation() {
    add(&metrics().full_list_translation_calls, 1);
}

pub fn m37_record_full_list_fallback() {
    add(&metrics().full_list_fallback_count, 1);
}

fn record_exact_lookup(duration: Duration, candidates: usize) {
    let metrics = metrics();
    if m37_metrics_enabled() {
        metrics.exact_lookup_calls.fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.exact_lookup_ns, duration);
        add(&metrics.exact_lookup_candidates, candidates as u64);
    }
}

fn record_prefix_lookup(duration: Duration, candidates: usize) {
    let metrics = metrics();
    if m37_metrics_enabled() {
        metrics.prefix_lookup_calls.fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.prefix_lookup_ns, duration);
        add(&metrics.prefix_lookup_candidates, candidates as u64);
    }
}

pub fn m37_record_heap_exact_lookup(duration: Duration, candidates: usize) {
    record_exact_lookup(duration, candidates);
    add(&metrics().heap_exact_lookup_calls, 1);
}

pub fn m37_record_heap_prefix_lookup(duration: Duration, candidates: usize) {
    record_prefix_lookup(duration, candidates);
    add(&metrics().heap_prefix_lookup_calls, 1);
}

pub fn m37_record_no_marisa_compact_exact_lookup(duration: Duration, candidates: usize) {
    record_exact_lookup(duration, candidates);
    add(&metrics().no_marisa_compact_exact_lookup_calls, 1);
}

pub fn m37_record_no_marisa_compact_prefix_lookup(duration: Duration, candidates: usize) {
    record_prefix_lookup(duration, candidates);
    add(&metrics().no_marisa_compact_prefix_lookup_calls, 1);
}

pub fn m37_record_rsmarisa_exact_lookup(duration: Duration, candidates: usize) {
    record_exact_lookup(duration, candidates);
    add(&metrics().rsmarisa_exact_lookup_calls, 1);
}

pub fn m37_record_rsmarisa_prefix_lookup(duration: Duration, candidates: usize) {
    record_prefix_lookup(duration, candidates);
    add(&metrics().rsmarisa_prefix_lookup_calls, 1);
}

pub fn m37_record_prism_lookup(duration: Duration, codes: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics.prism_lookup_calls.fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.prism_lookup_ns, duration);
        add(&metrics.prism_lookup_codes, codes as u64);
    }
}

pub fn m37_record_abi_c_string_allocation(bytes: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .abi_c_string_allocations
            .fetch_add(1, Ordering::Relaxed);
        add(&metrics.abi_c_string_bytes, bytes as u64);
    }
}

pub fn m37_record_abi_c_string_allocation_duration(bytes: usize, duration: Duration) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .abi_c_string_allocations
            .fetch_add(1, Ordering::Relaxed);
        add(&metrics.abi_c_string_bytes, bytes as u64);
        add_duration(&metrics.abi_c_string_allocation_ns, duration);
    }
}

pub fn m37_record_sentence_candidate(duration: Duration, result_candidates: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .sentence_candidate_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.sentence_candidate_ns, duration);
        add(
            &metrics.sentence_result_candidates,
            result_candidates as u64,
        );
    }
}

pub fn m37_record_sentence_candidate_metrics(record: M37SentenceCandidateMetrics) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .sentence_candidate_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.sentence_candidate_ns, record.duration);
        add(
            &metrics.sentence_result_candidates,
            record.result_candidates as u64,
        );
        add(
            &metrics.sentence_substrings_considered,
            record.substrings_considered as u64,
        );
        add(
            &metrics.sentence_exact_lookup_calls,
            record.exact_lookup_calls as u64,
        );
        add_duration(&metrics.sentence_exact_lookup_ns, record.exact_lookup_ns);
        add(
            &metrics.sentence_exact_lookup_candidates,
            record.exact_lookup_candidates as u64,
        );
        add(
            &metrics.sentence_prefix_lookup_calls,
            record.prefix_lookup_calls as u64,
        );
        add_duration(&metrics.sentence_prefix_lookup_ns, record.prefix_lookup_ns);
        add(
            &metrics.sentence_prefix_lookup_candidates,
            record.prefix_lookup_candidates as u64,
        );
        add(
            &metrics.sentence_entry_matches_collected,
            record.entry_matches_collected as u64,
        );
        add(&metrics.sentence_path_clones, record.path_clones as u64);
        add(
            &metrics.sentence_path_replacements,
            record.path_replacements as u64,
        );
        add(&metrics.sentence_paths_pruned, record.paths_pruned as u64);
        max(
            &metrics.sentence_max_live_paths,
            record.max_live_paths as u64,
        );
    }
}

pub fn m37_record_sentence_substring_considered() {
    add(&metrics().sentence_substrings_considered, 1);
}

pub fn m37_record_sentence_exact_lookup(duration: Duration, candidates: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .sentence_exact_lookup_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.sentence_exact_lookup_ns, duration);
        add(&metrics.sentence_exact_lookup_candidates, candidates as u64);
    }
}

pub fn m37_record_sentence_prefix_lookup(duration: Duration, candidates: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .sentence_prefix_lookup_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.sentence_prefix_lookup_ns, duration);
        add(
            &metrics.sentence_prefix_lookup_candidates,
            candidates as u64,
        );
    }
}

pub fn m37_record_sentence_entry_matches_collected(count: usize) {
    add(&metrics().sentence_entry_matches_collected, count as u64);
}

pub fn m37_record_sentence_path_clones(count: usize) {
    add(&metrics().sentence_path_clones, count as u64);
}

pub fn m37_record_sentence_path_replacements(count: usize) {
    add(&metrics().sentence_path_replacements, count as u64);
}

pub fn m37_record_sentence_paths_pruned(count: usize) {
    add(&metrics().sentence_paths_pruned, count as u64);
}

pub fn m37_record_sentence_max_live_paths(count: usize) {
    max(&metrics().sentence_max_live_paths, count as u64);
}

pub fn m37_record_upstream_sentence_model(duration: Duration, candidates: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .upstream_sentence_model_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.upstream_sentence_model_ns, duration);
        add(
            &metrics.upstream_sentence_model_candidates,
            candidates as u64,
        );
    }
}

pub fn m37_record_upstream_sentence_model_scan(
    code_prefix_checks: usize,
    table_entries_considered: usize,
    vocabulary_entries_considered: usize,
    graph_edges: usize,
) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        add(
            &metrics.upstream_sentence_model_code_prefix_checks,
            code_prefix_checks as u64,
        );
        add(
            &metrics.upstream_sentence_model_table_entries_considered,
            table_entries_considered as u64,
        );
        add(
            &metrics.upstream_sentence_model_vocabulary_entries_considered,
            vocabulary_entries_considered as u64,
        );
        add(
            &metrics.upstream_sentence_model_graph_edges,
            graph_edges as u64,
        );
    }
}

pub fn m37_record_upstream_sentence_model_index_build(duration: Duration) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .upstream_sentence_model_index_build_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.upstream_sentence_model_index_build_ns, duration);
    }
}

pub fn m37_record_upstream_sentence_model_lookup_index(record: M40SentenceLookupMetrics) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        add(
            &metrics.upstream_sentence_model_exact_range_index_hits,
            record.exact_range_index_hits as u64,
        );
        add(
            &metrics.upstream_sentence_model_exact_range_index_misses,
            record.exact_range_index_misses as u64,
        );
        add(
            &metrics.upstream_sentence_model_prefix_filter_hits,
            record.prefix_filter_hits as u64,
        );
        add(
            &metrics.upstream_sentence_model_prefix_filter_misses,
            record.prefix_filter_misses as u64,
        );
        add(
            &metrics.upstream_sentence_model_prefix_filter_early_breaks,
            record.prefix_filter_early_breaks as u64,
        );
        add(
            &metrics.upstream_sentence_model_reachable_starts_visited,
            record.reachable_starts_visited as u64,
        );
        add(
            &metrics.upstream_sentence_model_unreachable_starts_skipped,
            record.unreachable_starts_skipped as u64,
        );
        add(
            &metrics.upstream_sentence_model_phrase_index_walk_calls,
            record.phrase_index_walk_calls as u64,
        );
        add(
            &metrics.upstream_sentence_model_phrase_index_nodes_visited,
            record.phrase_index_nodes_visited as u64,
        );
        add(
            &metrics.upstream_sentence_model_phrase_index_entry_ranges_emitted,
            record.phrase_index_entry_ranges_emitted as u64,
        );
        add(
            &metrics.upstream_sentence_model_partition_point_fallback_calls,
            record.partition_point_fallback_calls as u64,
        );
        metrics
            .upstream_sentence_model_graph_rebuild_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(
            &metrics.upstream_sentence_model_graph_rebuild_ns,
            record.graph_rebuild_duration,
        );
        add(
            &metrics.upstream_sentence_model_incremental_reuse_hits,
            record.incremental_reuse_hits as u64,
        );
        add_duration(
            &metrics.upstream_sentence_model_incremental_extend_ns,
            record.incremental_extend_duration,
        );
        add(
            &metrics.upstream_sentence_model_incremental_discarded_rebuild_chars,
            record.incremental_discarded_rebuild_chars as u64,
        );
    }
}

pub fn m37_record_prefix_fallback(duration: Duration, views_visited: usize, candidates: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .prefix_fallback_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.prefix_fallback_ns, duration);
        add(&metrics.prefix_fallback_views_visited, views_visited as u64);
        add(&metrics.prefix_fallback_candidates, candidates as u64);
    }
}

pub fn m37_record_dynamic_correction(
    duration: Duration,
    codes_considered: usize,
    candidates: usize,
) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .dynamic_correction_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.dynamic_correction_ns, duration);
        add(
            &metrics.dynamic_correction_codes_considered,
            codes_considered as u64,
        );
        add(&metrics.dynamic_correction_candidates, candidates as u64);
    }
}

pub fn m37_record_abbreviation_span_discovery(
    duration: Duration,
    candidates_considered: usize,
    codes_emitted: usize,
) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .abbreviation_span_discovery_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.abbreviation_span_discovery_ns, duration);
        add(
            &metrics.abbreviation_span_candidates_considered,
            candidates_considered as u64,
        );
        add(
            &metrics.abbreviation_span_codes_emitted,
            codes_emitted as u64,
        );
    }
}

pub fn m37_record_abbreviation_model_has_code(duration: Duration) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .abbreviation_model_has_code_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.abbreviation_model_has_code_ns, duration);
    }
}

pub fn m37_record_abbreviation_code_span_graph_build(duration: Duration) {
    add_duration(&metrics().abbreviation_code_span_graph_build_ns, duration);
}

pub fn m37_record_abbreviation_sentence_ranking(duration: Duration) {
    add_duration(&metrics().abbreviation_sentence_ranking_ns, duration);
}

pub fn m37_record_abbreviation_preedit_format(duration: Duration) {
    add_duration(&metrics().abbreviation_preedit_format_ns, duration);
}

pub fn m37_record_abbreviation_candidate_format(duration: Duration) {
    add_duration(&metrics().abbreviation_candidate_format_ns, duration);
}

pub fn m37_record_short_key_candidate_rows_scanned(count: usize) {
    add(&metrics().short_key_candidate_rows_scanned, count as u64);
}

pub fn m37_record_short_key_candidate_materialized() {
    add(&metrics().short_key_candidates_materialized, 1);
}

pub fn m37_record_short_key_candidates_cloned(count: usize) {
    add(&metrics().short_key_candidates_cloned, count as u64);
}

pub fn m37_record_short_key_filter(duration: Duration) {
    add_duration(&metrics().short_key_filter_ns, duration);
}

pub fn m37_record_short_key_sort_rank(duration: Duration) {
    add_duration(&metrics().short_key_sort_rank_ns, duration);
}

pub fn m37_record_short_key_comment_quality(duration: Duration) {
    add_duration(&metrics().short_key_comment_quality_ns, duration);
}

pub fn m37_record_short_key_first_page_materialize(duration: Duration) {
    add_duration(&metrics().short_key_first_page_materialize_ns, duration);
}

pub fn m37_record_track_b_spelling_expansion(duration: Duration, expansions_considered: usize) {
    add(
        &metrics().track_b_spelling_expansions_considered,
        expansions_considered as u64,
    );
    add_duration(&metrics().track_b_spelling_expansion_ns, duration);
}

pub fn m37_record_track_b_exact_lookup(duration: Duration) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .track_b_exact_lookup_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.track_b_exact_lookup_ns, duration);
    }
}

pub fn m37_record_track_b_prefix_lookup(duration: Duration) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .track_b_prefix_lookup_calls
            .fetch_add(1, Ordering::Relaxed);
        add_duration(&metrics.track_b_prefix_lookup_ns, duration);
    }
}

pub fn m37_record_track_b_candidate_materialized() {
    add(&metrics().track_b_candidates_materialized, 1);
}

pub fn m37_record_track_b_first_page_materialize(duration: Duration) {
    add_duration(&metrics().track_b_first_page_materialize_ns, duration);
}
