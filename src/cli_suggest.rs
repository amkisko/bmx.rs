/// Top-level subcommand names used for typo suggestions before default-run.
const SUBCOMMANDS: &[&str] = &[
    "exec",
    "install",
    "rebuild",
    "uninstall",
    "reinstall",
    "self-update",
    "update",
    "show",
    "source",
    "isolation",
    "checkout",
    "trust",
    "shim",
    "doctor",
    "clean",
    "search",
    "history",
    "undo",
    "help",
];

/// If `token` looks like a mistyped subcommand (not a URL/spec), return the closest name.
pub(crate) fn suggest_subcommand(token: &str) -> Option<&'static str> {
    if token.is_empty() || looks_like_app_spec(token) {
        return None;
    }

    let normalized = token.to_ascii_lowercase();
    let prefix_hits: Vec<&'static str> = SUBCOMMANDS
        .iter()
        .copied()
        .filter(|name| name.starts_with(&normalized) && normalized.len() >= 3)
        .collect();
    if prefix_hits.len() == 1 {
        return Some(prefix_hits[0]);
    }

    let mut best: Option<(&'static str, usize)> = None;
    for name in SUBCOMMANDS {
        let distance = edit_distance(&normalized, name);
        if distance == 0 || distance > 2 {
            continue;
        }
        match best {
            Some((_, current)) if distance >= current => {}
            _ => best = Some((name, distance)),
        }
    }
    best.map(|(name, _)| name)
}

fn looks_like_app_spec(token: &str) -> bool {
    token.contains('/')
        || token.contains(':')
        || token.contains('@')
        || token.contains('.')
        || token.contains('\\')
}

fn edit_distance(left: &str, right: &str) -> usize {
    let left_chars: Vec<char> = left.chars().collect();
    let right_chars: Vec<char> = right.chars().collect();
    let rows = left_chars.len() + 1;
    let cols = right_chars.len() + 1;
    let mut table = vec![0usize; rows * cols];

    for row in 0..rows {
        table[row * cols] = row;
    }
    for (col, cell) in table.iter_mut().enumerate().take(cols) {
        *cell = col;
    }

    for row in 1..rows {
        for col in 1..cols {
            let cost = usize::from(left_chars[row - 1] != right_chars[col - 1]);
            let deletion = table[(row - 1) * cols + col] + 1;
            let insertion = table[row * cols + col - 1] + 1;
            let substitution = table[(row - 1) * cols + col - 1] + cost;
            table[row * cols + col] = deletion.min(insertion).min(substitution);
        }
    }

    table[rows * cols - 1]
}

pub(crate) fn concise_usage() -> &'static str {
    "bmx — install, build, and run apps from git sources\n\n\
Examples:\n\
  bmx ripgrep\n\
  bmx install owner/repo\n\
  bmx doctor\n\n\
Run `bmx --help` for the full command list.\n\
Docs: https://github.com/amkisko/bmx.rs"
}

#[cfg(test)]
mod tests {
    use super::{edit_distance, suggest_subcommand};

    #[test]
    fn suggests_near_miss_subcommand_names() {
        assert_eq!(suggest_subcommand("instal"), Some("install"));
        assert_eq!(suggest_subcommand("updat"), Some("update"));
        assert_eq!(suggest_subcommand("docto"), Some("doctor"));
    }

    #[test]
    fn ignores_app_specs_and_unrelated_names() {
        assert_eq!(suggest_subcommand("ripgrep"), None);
        assert_eq!(suggest_subcommand("owner/repo"), None);
        assert_eq!(suggest_subcommand("app@main"), None);
        assert_eq!(suggest_subcommand("github.com/org/repo"), None);
    }

    #[test]
    fn edit_distance_counts_substitutions() {
        assert_eq!(edit_distance("instal", "install"), 1);
        assert_eq!(edit_distance("abc", "abc"), 0);
    }
}
