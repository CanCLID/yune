#[path = "frontend_hosts/mod.rs"]
mod frontend_hosts;

#[test]
fn frontend_host_trace_fixture_contract_is_sanitized() {
    frontend_hosts::assert_baseline_fixture_is_sanitized();
}
