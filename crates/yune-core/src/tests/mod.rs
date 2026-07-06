use std::sync::{Mutex, MutexGuard, OnceLock};

mod dictionary;
mod engine;
mod filter;
mod poet;
mod translator;

fn m37_metrics_test_guard() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn m37_metrics_test_enable_is_thread_local() {
    let _guard = m37_metrics_test_guard();
    crate::m37_metrics_enable(true);
    crate::m37_metrics_reset();

    let worker = std::thread::spawn(|| {
        assert!(!crate::m37_metrics_enabled());
        crate::m37_record_lookup_view();
    });
    worker.join().expect("test metrics worker should not panic");

    crate::m37_record_lookup_view();
    let metrics = crate::m37_metrics_snapshot();
    crate::m37_metrics_enable(false);

    assert_eq!(metrics.lookup_views_visited, 1);
}
