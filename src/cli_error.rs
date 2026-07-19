//! Human-oriented CLI error rendering.

use crate::exit::{AppExit, exit_for_message};
use std::io::{self, Write};

const ISSUES_URL: &str = "https://github.com/amkisko/bmx.rs/issues/new";

pub struct ErrorContext {
    pub verbose: bool,
}

pub fn print_error(error: &anyhow::Error, context: &ErrorContext) -> AppExit {
    let _ = writeln!(io::stderr(), "bmx error: {error:#}");
    let message = error.to_string();
    if !context.verbose {
        print_hints(&message);
    }
    if should_suggest_bug_report(&message) {
        let _ = writeln!(
            io::stderr(),
            "\nReport a bug: {ISSUES_URL} (bmx {})",
            env!("CARGO_PKG_VERSION")
        );
    }
    exit_for_message(&message)
}

fn print_hints(message: &str) {
    let lower = message.to_lowercase();
    if lower.contains("untrusted") || lower.contains("trust") {
        let _ = writeln!(
            io::stderr(),
            "Hint: review with `bmx trust show`, or set BMX_TRUST_ASSUME_YES=1 for non-interactive consent."
        );
    } else if lower.contains("bmx.run") || lower.contains("executable path") {
        let _ = writeln!(
            io::stderr(),
            "Hint: bmx.run must be a relative path under the repo (no absolute paths or '..')."
        );
    } else if lower.contains("http") || lower.contains("search api") {
        let _ = writeln!(
            io::stderr(),
            "Hint: check network access and optional GITHUB_TOKEN / GITLAB_TOKEN."
        );
    }
}

fn should_suggest_bug_report(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("internal error") || lower.contains("bug:")
}
