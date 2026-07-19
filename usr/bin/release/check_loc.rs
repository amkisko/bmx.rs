//! Check Rust source line counts against workspace limits.
//! Run from repository root: cargo run -p release --bin check-loc

use std::env;
use std::process::ExitCode;

const GREEN: &str = "\x1b[0;32m";
const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const NC: &str = "\x1b[0m";

fn main() -> ExitCode {
    let root = env::current_dir().expect("current dir");
    let update_baseline = env::args().any(|argument| argument == "--update-baseline");

    if update_baseline {
        match release::write_loc_baseline(&root) {
            Ok(recorded) => {
                if recorded.is_empty() {
                    println!(
                        "{}no files exceed the {} line hard limit{}",
                        GREEN,
                        release::HARD_LOC_LIMIT,
                        NC
                    );
                } else {
                    println!(
                        "{}updated loc baseline for {} file(s){}",
                        GREEN,
                        recorded.len(),
                        NC
                    );
                    for path in recorded {
                        println!("  {path}");
                    }
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{}check-loc failed: {error}{}", RED, NC);
                ExitCode::FAILURE
            }
        }
    } else {
        match release::check_loc_limits(&root) {
            Ok(report) => {
                for warning in &report.warnings {
                    eprintln!(
                        "{}{}: {}{}",
                        YELLOW,
                        format_loc_finding(warning),
                        warning.path,
                        NC
                    );
                }

                if report.has_errors() {
                    eprintln!(
                        "{}line count check failed (hard limit {} lines, soft warning {} lines):{}",
                        RED,
                        release::HARD_LOC_LIMIT,
                        release::SOFT_LOC_LIMIT,
                        NC
                    );
                    for error in &report.errors {
                        eprintln!("  {}", format_loc_finding(error));
                        eprintln!("    {}", error.path);
                    }
                    ExitCode::FAILURE
                } else if report.warnings.is_empty() {
                    println!(
                        "{}all Rust sources are within the {} line soft limit{}",
                        GREEN,
                        release::SOFT_LOC_LIMIT,
                        NC
                    );
                    ExitCode::SUCCESS
                } else {
                    eprintln!(
                        "{}line count warnings: {} file(s) exceed the {} line soft limit (hard limit {}){}",
                        YELLOW,
                        report.warnings.len(),
                        release::SOFT_LOC_LIMIT,
                        release::HARD_LOC_LIMIT,
                        NC
                    );
                    ExitCode::SUCCESS
                }
            }
            Err(error) => {
                eprintln!("{}check-loc failed: {error}{}", RED, NC);
                ExitCode::FAILURE
            }
        }
    }
}

fn format_loc_finding(finding: &release::LocFinding) -> String {
    match finding.kind {
        release::LocFindingKind::SoftLimit => {
            format!(
                "warning: {} lines exceeds soft limit {}",
                finding.lines,
                release::SOFT_LOC_LIMIT
            )
        }
        release::LocFindingKind::HardLimit => {
            format!(
                "error: {} lines exceeds hard limit {}",
                finding.lines,
                release::HARD_LOC_LIMIT
            )
        }
        release::LocFindingKind::BaselineExceeded { allowed } => {
            format!(
                "error: {} lines exceeds baseline allowance of {}",
                finding.lines, allowed
            )
        }
        release::LocFindingKind::BaselineShrink { allowed } => {
            format!(
                "warning: {} lines is below baseline {}; run `cargo run -p release --bin check-loc -- --update-baseline`",
                finding.lines, allowed
            )
        }
    }
}
