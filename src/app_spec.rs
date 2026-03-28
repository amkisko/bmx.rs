//! App install strings: optional `@ref`, optional trailing `:cargo_package` on URLs,
//! `git@…` hosts, and `owner/repo` short paths (must contain `/`).

use crate::source::{app_id, looks_like_url};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppSpec {
    /// URL or name used for `resolve_source` (no `:package` suffix).
    pub(crate) source: String,
    pub(crate) requested_ref: Option<String>,
    /// When set, Rust workspaces build with `cargo build --release -p <name>` and pick that binary.
    pub(crate) cargo_package: Option<String>,
}

pub(crate) fn parse_app_spec(input: &str) -> AppSpec {
    let (without_ref, requested_ref) = split_at_ref(input);
    let (source, cargo_package) = split_cargo_package(without_ref);
    AppSpec {
        source,
        requested_ref,
        cargo_package,
    }
}

/// Directory id under `apps/` (distinct per repo and optional Cargo package).
pub(crate) fn layout_id(spec: &AppSpec) -> String {
    let base = app_id(&spec.source);
    match &spec.cargo_package {
        None => base,
        Some(pkg) => format!("{}__{}", base, pkg_segment_id(pkg)),
    }
}

fn pkg_segment_id(pkg: &str) -> String {
    pkg.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn split_at_ref(input: &str) -> (&str, Option<String>) {
    let Some(at_index) = input.rfind('@') else {
        return (input, None);
    };

    let right = &input[at_index + 1..];
    if right.is_empty() || is_ssh_user_separator(input) || is_url_credentials(input, right) {
        return (input, None);
    }

    (&input[..at_index], Some(right.to_string()))
}

fn is_ssh_user_separator(input: &str) -> bool {
    input.starts_with("git@") && input.matches('@').count() == 1
}

fn is_url_credentials(input: &str, right_of_at: &str) -> bool {
    input.contains("://") && right_of_at.contains('/')
}

/// `https://host/repo:pkg`, `git@host:path:pkg`, or `owner/repo:pkg` (with `default_source`).
/// Avoids `registry_key:tail` (no `/` before the last `:pkg`).
fn split_cargo_package(without_ref: &str) -> (String, Option<String>) {
    if !cargo_package_suffix_eligible(without_ref) {
        return (without_ref.to_string(), None);
    }

    let Some((left, right)) = without_ref.rsplit_once(':') else {
        return (without_ref.to_string(), None);
    };

    if right.is_empty() {
        return (without_ref.to_string(), None);
    }

    // Avoid `https://host:443/...` style false positives (port is digits-only).
    if right.chars().all(|c| c.is_ascii_digit()) {
        return (without_ref.to_string(), None);
    }

    if !is_valid_cargo_package_name(right) {
        return (without_ref.to_string(), None);
    }

    (left.to_string(), Some(right.to_string()))
}

fn cargo_package_suffix_eligible(s: &str) -> bool {
    looks_like_url(s) || s.starts_with("git@") || s.contains('/')
}

fn is_valid_cargo_package_name(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub(crate) fn app_spec_from_install_meta(
    source_url: &str,
    requested_ref: Option<String>,
    rust_package: Option<String>,
) -> AppSpec {
    AppSpec {
        source: source_url.to_string(),
        requested_ref,
        cargo_package: rust_package,
    }
}
