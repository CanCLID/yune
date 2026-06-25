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
    pub abi_c_string_allocations: u64,
    pub abi_c_string_bytes: u64,
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
    abi_c_string_allocations: AtomicU64,
    abi_c_string_bytes: AtomicU64,
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
    metrics.abi_c_string_allocations.store(0, Ordering::Relaxed);
    metrics.abi_c_string_bytes.store(0, Ordering::Relaxed);
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
        abi_c_string_allocations: metrics.abi_c_string_allocations.load(Ordering::Relaxed),
        abi_c_string_bytes: metrics.abi_c_string_bytes.load(Ordering::Relaxed),
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

pub fn m37_record_abi_c_string_allocation(bytes: usize) {
    if m37_metrics_enabled() {
        let metrics = metrics();
        metrics
            .abi_c_string_allocations
            .fetch_add(1, Ordering::Relaxed);
        add(&metrics.abi_c_string_bytes, bytes as u64);
    }
}
