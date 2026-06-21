use super::{build_prism_bin, build_reverse_bin, build_table_bin, RimeDictRebuildExecutionReport};
use super::{RimeDictRebuildPlan, TableDictionary};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub struct RimeDictRebuildSources<'a> {
    pub artifact_stem: &'a str,
    pub table_dictionary: &'a TableDictionary,
    pub reverse_dictionary: &'a TableDictionary,
    pub syllabary: &'a [String],
    pub algebra_formulas: &'a [String],
    pub schema_file_checksum: u32,
}

#[derive(Debug)]
pub enum RimeDictRebuildExecuteError {
    InvalidArtifactStem,
    Io(io::Error),
}

impl From<io::Error> for RimeDictRebuildExecuteError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

pub fn execute_rebuild_plan(
    plan: &RimeDictRebuildPlan,
    sources: &RimeDictRebuildSources<'_>,
    out_dir: impl AsRef<Path>,
) -> Result<RimeDictRebuildExecutionReport, RimeDictRebuildExecuteError> {
    if sources.artifact_stem.is_empty()
        || sources.artifact_stem.contains('/')
        || sources.artifact_stem.contains('\\')
        || sources.artifact_stem.contains(':')
    {
        return Err(RimeDictRebuildExecuteError::InvalidArtifactStem);
    }

    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)?;
    if plan.rebuild_table {
        fs::write(
            artifact_path(out_dir, sources.artifact_stem, "table.bin"),
            build_table_bin(sources.table_dictionary, plan.dict_file_checksum),
        )?;
    }
    if plan.rebuild_prism {
        fs::write(
            artifact_path(out_dir, sources.artifact_stem, "prism.bin"),
            build_prism_bin(
                sources.syllabary,
                sources.algebra_formulas,
                plan.dict_file_checksum,
                sources.schema_file_checksum,
            ),
        )?;
    }
    if plan.rebuild_reverse {
        fs::write(
            artifact_path(out_dir, sources.artifact_stem, "reverse.bin"),
            build_reverse_bin(sources.reverse_dictionary, plan.dict_file_checksum),
        )?;
    }
    Ok(plan.report)
}

fn artifact_path(out_dir: &Path, stem: &str, suffix: &str) -> PathBuf {
    out_dir.join(format!("{stem}.{suffix}"))
}
