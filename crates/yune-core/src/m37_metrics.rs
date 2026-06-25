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
