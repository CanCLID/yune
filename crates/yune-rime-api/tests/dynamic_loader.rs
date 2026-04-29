use std::path::PathBuf;

#[test]
fn dynamic_loader_harness_loads_cargo_cdylib_and_api_table() {
    let artifact = discover_dynamic_artifact().expect("dynamic artifact discovery should be implemented");
    assert!(artifact.is_file(), "dynamic artifact should exist at {}", artifact.display());
}

fn discover_dynamic_artifact() -> Result<PathBuf, String> {
    Err("dynamic loader harness not implemented yet".to_owned())
}
