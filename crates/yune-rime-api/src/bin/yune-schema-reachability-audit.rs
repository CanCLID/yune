use std::{env, path::PathBuf, process};

use yune_rime_api::schema_reachability_audit_json;

fn main() {
    match parse_args().and_then(|args| {
        schema_reachability_audit_json(
            &args.repo_root,
            &args.shared_data_dir,
            &args.user_data_dir,
            &args.schema_assets,
        )
    }) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("yune-schema-reachability-audit: {error}");
            process::exit(2);
        }
    }
}

#[derive(Debug)]
struct Args {
    repo_root: PathBuf,
    shared_data_dir: PathBuf,
    user_data_dir: PathBuf,
    schema_assets: Vec<PathBuf>,
}

fn parse_args() -> Result<Args, String> {
    let mut repo_root = None;
    let mut shared_data_dir = None;
    let mut user_data_dir = None;
    let mut schema_assets = Vec::new();
    let mut args = env::args_os().skip(1);
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("{} requires a value", flag.to_string_lossy()))?;
        match flag.to_str() {
            Some("--repo-root") if repo_root.is_none() => repo_root = Some(PathBuf::from(value)),
            Some("--shared-data-dir") if shared_data_dir.is_none() => {
                shared_data_dir = Some(PathBuf::from(value));
            }
            Some("--user-data-dir") if user_data_dir.is_none() => {
                user_data_dir = Some(PathBuf::from(value));
            }
            Some("--schema-asset") => schema_assets.push(PathBuf::from(value)),
            Some(flag) => return Err(format!("unknown or duplicate argument {flag}")),
            None => return Err("argument names must be valid UTF-8".to_owned()),
        }
    }
    Ok(Args {
        repo_root: repo_root.ok_or_else(|| "--repo-root is required".to_owned())?,
        shared_data_dir: shared_data_dir
            .ok_or_else(|| "--shared-data-dir is required".to_owned())?,
        user_data_dir: user_data_dir
            .ok_or_else(|| "--user-data-dir is required".to_owned())?,
        schema_assets,
    })
}
