use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::analyzer::FunctionMetric;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IstanbulFile {
    statement_map: HashMap<String, Location>,
    #[serde(default)]
    fn_map: HashMap<String, IstanbulFunction>,
    s: HashMap<String, u64>,
    #[serde(default)]
    f: HashMap<String, u64>,
}

#[derive(Debug, Deserialize)]
struct IstanbulFunction {
    loc: Location,
}

#[derive(Debug, Deserialize)]
struct Location {
    start: Position,
    end: Position,
}

#[derive(Debug, Deserialize)]
struct Position {
    line: u32,
    column: u32,
}

pub(crate) struct CoverageIndex {
    files: HashMap<PathBuf, IstanbulFile>,
}

impl CoverageIndex {
    pub fn load(path: &Path, project_root: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read coverage file {}", path.display()))?;
        let raw: HashMap<String, IstanbulFile> = serde_json::from_str(&content)
            .with_context(|| format!("invalid Istanbul coverage JSON in {}", path.display()))?;
        let mut files = HashMap::new();

        for (file, coverage) in raw {
            let path = PathBuf::from(file);
            let path = if path.is_absolute() {
                path
            } else {
                project_root.join(path)
            };
            let normalized = fs::canonicalize(&path).unwrap_or(path);
            files.insert(normalized, coverage);
        }
        Ok(Self { files })
    }

    pub fn coverage_for(
        &self,
        path: &Path,
        source: &str,
        functions: &[FunctionMetric],
    ) -> Result<Vec<f64>> {
        let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
        let Some(file) = self.files.get(&path) else {
            bail!("coverage data is missing for {}", path.display());
        };

        let mut totals = vec![0_u32; functions.len()];
        let mut covered = vec![0_u32; functions.len()];

        for (id, location) in &file.statement_map {
            let Some(offset) = source_offset(source, &location.start) else {
                continue;
            };
            if let Some(index) = innermost_function(functions, offset) {
                totals[index] += 1;
                if file.s.get(id).copied().unwrap_or_default() > 0 {
                    covered[index] += 1;
                }
            }
        }

        Ok(functions
            .iter()
            .enumerate()
            .map(|(index, function)| {
                if totals[index] > 0 {
                    f64::from(covered[index]) / f64::from(totals[index])
                } else {
                    function_hit_coverage(file, source, function).unwrap_or(0.0)
                }
            })
            .collect())
    }
}

fn innermost_function(functions: &[FunctionMetric], offset: u32) -> Option<usize> {
    functions
        .iter()
        .enumerate()
        .filter(|(_, function)| {
            function.body_span.start <= offset && offset < function.body_span.end
        })
        .min_by_key(|(_, function)| function.body_span.size())
        .map(|(index, _)| index)
}

fn function_hit_coverage(
    file: &IstanbulFile,
    source: &str,
    function: &FunctionMetric,
) -> Option<f64> {
    file.fn_map
        .iter()
        .filter_map(|(id, entry)| {
            let start = source_offset(source, &entry.loc.start)?;
            let end = source_offset(source, &entry.loc.end)?;
            let overlaps = start < function.span.end && end > function.span.start;
            overlaps.then_some((id, start.abs_diff(function.span.start)))
        })
        .min_by_key(|(_, distance)| *distance)
        .map(|(id, _)| {
            if file.f.get(id).copied().unwrap_or_default() > 0 {
                1.0
            } else {
                0.0
            }
        })
}

fn source_offset(source: &str, position: &Position) -> Option<u32> {
    let line_start = if position.line == 1 {
        0
    } else {
        source
            .match_indices('\n')
            .nth(position.line.saturating_sub(2) as usize)?
            .0
            + 1
    };
    let line = source[line_start..]
        .split_once('\n')
        .map_or(&source[line_start..], |(line, _)| line);
    let mut utf16_column = 0_u32;
    let mut byte_column = 0_usize;
    for character in line.chars() {
        if utf16_column >= position.column {
            break;
        }
        utf16_column += character.len_utf16() as u32;
        byte_column += character.len_utf8();
    }
    (utf16_column == position.column).then_some((line_start + byte_column) as u32)
}

#[cfg(test)]
mod tests {
    use super::{Position, source_offset};

    #[test]
    fn converts_utf16_coverage_columns_to_utf8_offsets() {
        let source = "const emoji = '😀';\nreturn emoji;";
        let offset = source_offset(
            source,
            &Position {
                line: 1,
                column: 18,
            },
        )
        .unwrap();
        assert_eq!(&source[offset as usize..], ";\nreturn emoji;");
    }
}
