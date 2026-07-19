//! Sync packaging manifests to the workspace version in Cargo.toml.
//! Run from repository root: cargo run -p release --bin sync-packaging

use std::env;
use std::process::ExitCode;

const GREEN: &str = "\x1b[0;32m";
const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const NC: &str = "\x1b[0m";

fn main() -> ExitCode {
    let root = env::current_dir().expect("current dir");
    let check_only = env::args().any(|arg| arg == "--check");

    if check_only {
        match release::check_packaging(&root) {
            Ok(()) => {
                println!(
                    "{}packaging matches workspace version {}{}",
                    GREEN,
                    release::workspace_version(&root),
                    NC
                );
                ExitCode::SUCCESS
            }
            Err(mismatches) => {
                eprintln!("{}packaging is out of sync:{}", RED, NC);
                for mismatch in mismatches {
                    eprintln!("  - {mismatch}");
                }
                eprintln!(
                    "{}Run `cargo run -p release --bin sync-packaging` to update manifests.{}",
                    YELLOW, NC
                );
                ExitCode::FAILURE
            }
        }
    } else {
        let version = release::workspace_version(&root);
        match release::sync_packaging(&root) {
            Ok(updated) => {
                if updated.is_empty() {
                    println!(
                        "{}packaging already matches workspace version {}{}",
                        GREEN, version, NC
                    );
                } else {
                    println!("{}Synced packaging to version {}{}", GREEN, version, NC);
                    for path in updated {
                        println!("  updated {path}");
                    }
                    println!(
                        "{}Remember to fill checksums that need a published tag (Homebrew sha256, AUR updpkgsums).{}",
                        YELLOW, NC
                    );
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{}sync-packaging failed: {error}{}", RED, NC);
                ExitCode::FAILURE
            }
        }
    }
}
