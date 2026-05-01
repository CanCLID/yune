use std::sync::{Mutex, MutexGuard, OnceLock};

#[path = "frontend_hosts/mod.rs"]
mod frontend_hosts;

fn frontend_hosts_test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("frontend host integration test lock should not be poisoned")
}

#[test]
fn frontend_host_trace_fixture_contract_is_sanitized() {
    let _guard = frontend_hosts_test_guard();
    frontend_hosts::assert_baseline_fixture_is_sanitized();
}

#[test]
fn typeduck_web_wrapper_lifecycle_is_validated_through_yune_abi() {
    let _guard = frontend_hosts_test_guard();
    frontend_hosts::typeduck_web::typeduck_web_wrapper_lifecycle_is_validated_through_yune_abi();
}

#[test]
fn typeduck_web_basic_fixture_is_sanitized_and_matches_trace_contract() {
    let _guard = frontend_hosts_test_guard();
    let fixture = include_str!("../../../fixtures/frontend-traces/typeduck-web-basic.json");
    frontend_hosts::typeduck_web::assert_typeduck_web_fixture_contract(fixture);
}

#[test]
fn native_frontends_squirrel_lifecycle_is_source_modeled_through_yune_abi() {
    let _guard = frontend_hosts_test_guard();
    frontend_hosts::native_frontends::squirrel_lifecycle_is_source_modeled_through_yune_abi();
}

#[test]
fn native_frontends_squirrel_fixture_is_sanitized_and_matches_trace_contract() {
    let _guard = frontend_hosts_test_guard();
    let fixture = include_str!("../../../fixtures/frontend-traces/squirrel-lifecycle.json");
    frontend_hosts::native_frontends::assert_squirrel_fixture_contract(fixture);
}
