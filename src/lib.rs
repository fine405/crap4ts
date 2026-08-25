mod analyzer;
mod coverage;
mod crap;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use walkdir::{DirEntry, WalkDir};

use analyzer::analyze_source;
use coverage::CoverageIndex;

pub use crap::crap_score;

#[derive(Debug, Clone, Serialize)]
pub struct FunctionResult {
    pub file: String,
    pub function: String,
    pub line: u32,
    pub complexity: u32,
    pub coverage: f64,
    pub crap: f64,
}

pub fn analyze_paths(
    paths: &[PathBuf],
    coverage_path: &Path,
    project_root: &Path,
) -> Result<Vec<FunctionResult>> {
    let files = collect_source_files(paths)?;
    if files.is_empty() {
        bail!("no .ts or .tsx source files found");
    }

    let coverage = CoverageIndex::load(coverage_path, project_root)?;
    let mut results = Vec::new();

    for path in files {
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let functions = analyze_source(&path, &source)?;
        let percentages = coverage.coverage_for(&path, &source, &functions)?;
        let display_path = path
            .strip_prefix(project_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");

        for (function, coverage) in functions.into_iter().zip(percentages) {
            results.push(FunctionResult {
                file: display_path.clone(),
                function: function.name,
                line: function.line,
                complexity: function.complexity,
                coverage,
                crap: crap_score(function.complexity, coverage),
            });
        }
    }

    results.sort_by(|left, right| {
        right
            .crap
            .total_cmp(&left.crap)
            .then_with(|| left.file.cmp(&right.file))
            .then_with(|| left.line.cmp(&right.line))
    });
    Ok(results)
}

fn collect_source_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = BTreeSet::new();

    for path in paths {
        if path.is_file() {
            if is_source_file(path) {
                files.insert(canonical(path)?);
            }
            continue;
        }
        if !path.exists() {
            bail!("source path does not exist: {}", path.display());
        }

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|entry| !is_ignored_directory(entry))
        {
            let entry = entry.with_context(|| format!("failed to walk {}", path.display()))?;
            if entry.file_type().is_file() && is_source_file(entry.path()) {
                files.insert(canonical(entry.path())?);
            }
        }
    }

    Ok(files.into_iter().collect())
}

fn canonical(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).with_context(|| format!("failed to resolve {}", path.display()))
}

fn is_source_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.ends_with(".d.ts") {
        return false;
    }
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx")
    )
}

fn is_ignored_directory(entry: &DirEntry) -> bool {
    entry.file_type().is_dir()
        && matches!(
            entry.file_name().to_str(),
            Some(".git" | "node_modules" | "dist" | "build" | "coverage")
        )
}
