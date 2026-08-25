use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Result, bail};
use clap::{Parser, ValueEnum};
use crap4ts::{FunctionResult, analyze_paths};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "CRAP score analyzer for TypeScript and TSX powered by Oxc"
)]
struct Cli {
    #[arg(value_name = "PATH", default_value = "src")]
    paths: Vec<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    coverage: PathBuf,

    #[arg(long, value_name = "SCORE")]
    threshold: Option<f64>,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Serialize)]
struct JsonReport<'a> {
    schema_version: u32,
    threshold: Option<f64>,
    violations: usize,
    functions: &'a [FunctionResult],
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool> {
    let cli = Cli::parse();
    if cli.threshold.is_some_and(|threshold| threshold < 0.0) {
        bail!("threshold must be zero or greater");
    }

    let project_root = env::current_dir()?;
    let functions = analyze_paths(&cli.paths, &cli.coverage, &project_root)?;
    let violations = cli.threshold.map_or(0, |threshold| {
        functions
            .iter()
            .filter(|function| function.crap > threshold)
            .count()
    });

    match cli.format {
        OutputFormat::Text => print_text(&functions, cli.threshold, violations),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&JsonReport {
                schema_version: 1,
                threshold: cli.threshold,
                violations,
                functions: &functions,
            })?
        ),
    }

    Ok(violations == 0)
}

fn print_text(functions: &[FunctionResult], threshold: Option<f64>, violations: usize) {
    println!("{:<8} {:<5} {:<9} LOCATION", "CRAP", "COMP", "COVERAGE");
    for function in functions {
        println!(
            "{:<8.2} {:<5} {:>8.1}% {}:{} {}",
            function.crap,
            function.complexity,
            function.coverage * 100.0,
            function.file,
            function.line,
            function.function
        );
    }
    println!("\n{} function(s) analyzed", functions.len());
    if let Some(threshold) = threshold {
        println!("{violations} function(s) above threshold {threshold:.2}");
    }
}
