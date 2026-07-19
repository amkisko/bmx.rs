//! Keep packaging descriptors aligned with the workspace version.

use std::fs;
use std::path::{Path, PathBuf};

use crate::workspace_version;

pub fn sync_packaging(root: &Path) -> Result<Vec<String>, String> {
    let version = workspace_version(root);
    let mut updated = Vec::new();

    replace_in_file(
        root,
        &mut updated,
        "packaging/nix/default.nix",
        &format!(r#"version = "{version}";"#),
        |content| set_nix_version(content, &version),
    )?;
    replace_in_file(
        root,
        &mut updated,
        "packaging/nix/flake.nix",
        &format!(r#"version = "{version}";"#),
        |content| set_nix_version(content, &version),
    )?;
    replace_in_file(
        root,
        &mut updated,
        "packaging/homebrew/bmx.rb",
        &format!("url \"https://github.com/amkisko/bmx.rs/archive/refs/tags/v{version}.tar.gz\""),
        |content| set_homebrew_url(content, &version),
    )?;
    replace_in_file(
        root,
        &mut updated,
        "packaging/flatpak/io.github.amkisko.bmx.yml",
        &format!("tag: v{version}"),
        |content| set_flatpak_tag(content, &version),
    )?;
    replace_in_file(
        root,
        &mut updated,
        "packaging/aur/PKGBUILD",
        &format!("pkgver={version}"),
        |content| set_pkgver(content, &version),
    )?;
    replace_in_file(
        root,
        &mut updated,
        "packaging/freebsd/Makefile",
        &format!("DISTVERSION=\t{version}"),
        |content| set_distversion(content, &version),
    )?;

    rename_gentoo_ebuild(root, &version, &mut updated)?;
    replace_in_file(
        root,
        &mut updated,
        "packaging/gentoo/app-misc/bmx/README.md",
        &format!("bmx-{version}.ebuild"),
        |content| set_gentoo_readme_ebuild(content, &version),
    )?;

    Ok(updated)
}

pub fn check_packaging(root: &Path) -> Result<(), Vec<String>> {
    let version = workspace_version(root);
    let mut mismatches = Vec::new();

    expect_contains(
        &mut mismatches,
        root.join("packaging/nix/default.nix"),
        &format!(r#"version = "{version}";"#),
    );
    expect_contains(
        &mut mismatches,
        root.join("packaging/nix/flake.nix"),
        &format!(r#"version = "{version}";"#),
    );
    expect_contains(
        &mut mismatches,
        root.join("packaging/homebrew/bmx.rb"),
        &format!("v{version}.tar.gz"),
    );
    expect_contains(
        &mut mismatches,
        root.join("packaging/flatpak/io.github.amkisko.bmx.yml"),
        &format!("tag: v{version}"),
    );
    expect_contains(
        &mut mismatches,
        root.join("packaging/aur/PKGBUILD"),
        &format!("pkgver={version}"),
    );
    expect_contains(
        &mut mismatches,
        root.join("packaging/freebsd/Makefile"),
        &format!("DISTVERSION=\t{version}"),
    );

    let ebuild = root.join(format!(
        "packaging/gentoo/app-misc/bmx/bmx-{version}.ebuild"
    ));
    if !ebuild.is_file() {
        mismatches.push(format!("missing gentoo ebuild: {}", ebuild.display()));
    }

    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(mismatches)
    }
}

fn replace_in_file(
    root: &Path,
    updated: &mut Vec<String>,
    relative_path: &str,
    expected_snippet: &str,
    transform: impl FnOnce(&str) -> String,
) -> Result<(), String> {
    let path = root.join(relative_path);
    let content =
        fs::read_to_string(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let next = transform(&content);
    if next == content {
        if !content.contains(expected_snippet) {
            return Err(format!(
                "could not update {} to include {expected_snippet}",
                path.display()
            ));
        }
        return Ok(());
    }
    fs::write(&path, next).map_err(|error| format!("write {}: {error}", path.display()))?;
    updated.push(relative_path.to_string());
    Ok(())
}

fn expect_contains(mismatches: &mut Vec<String>, path: PathBuf, expected: &str) {
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            mismatches.push(format!("read {}: {error}", path.display()));
            return;
        }
    };
    if !content.contains(expected) {
        mismatches.push(format!(
            "{} is out of sync (expected {expected})",
            path.display()
        ));
    }
}

fn set_nix_version(content: &str, version: &str) -> String {
    replace_line_value(content, "version = ", &format!("\"{version}\";"))
}

fn set_homebrew_url(content: &str, version: &str) -> String {
    let prefix = "  url \"https://github.com/amkisko/bmx.rs/archive/refs/tags/v";
    replace_prefixed_line(content, prefix, &format!("{prefix}{version}.tar.gz\""))
}

fn set_flatpak_tag(content: &str, version: &str) -> String {
    replace_line_value(content, "tag: v", version)
}

fn set_pkgver(content: &str, version: &str) -> String {
    replace_line_value(content, "pkgver=", version)
}

fn set_distversion(content: &str, version: &str) -> String {
    replace_line_value(content, "DISTVERSION=\t", version)
}

fn set_gentoo_readme_ebuild(content: &str, version: &str) -> String {
    let pattern = "bmx-";
    content
        .lines()
        .map(|line| {
            if let Some(index) = line.find(pattern) {
                let suffix_start = index + pattern.len();
                if let Some(end) = line[suffix_start..].find(".ebuild") {
                    let mut next = line.to_string();
                    next.replace_range(suffix_start..suffix_start + end, version);
                    return next;
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if content.ends_with('\n') { "\n" } else { "" }
}

fn replace_line_value(content: &str, prefix: &str, value: &str) -> String {
    content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(prefix) {
                let indent = line.len() - line.trim_start().len();
                format!("{}{prefix}{value}", &line[..indent])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if content.ends_with('\n') { "\n" } else { "" }
}

fn replace_prefixed_line(content: &str, prefix: &str, replacement: &str) -> String {
    content
        .lines()
        .map(|line| {
            if line.starts_with(prefix) || line.contains(prefix) {
                replacement.to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if content.ends_with('\n') { "\n" } else { "" }
}

fn rename_gentoo_ebuild(
    root: &Path,
    version: &str,
    updated: &mut Vec<String>,
) -> Result<(), String> {
    let directory = root.join("packaging/gentoo/app-misc/bmx");
    let target = directory.join(format!("bmx-{version}.ebuild"));
    if target.is_file() {
        return Ok(());
    }

    let entries = fs::read_dir(&directory)
        .map_err(|error| format!("read {}: {error}", directory.display()))?;
    let mut source = None;
    for entry in entries {
        let entry = entry.map_err(|error| format!("read gentoo ebuild dir: {error}"))?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.starts_with("bmx-") && name.ends_with(".ebuild") {
            source = Some(entry.path());
            break;
        }
    }

    let source = source.ok_or_else(|| format!("no bmx ebuild found in {}", directory.display()))?;
    if source == target {
        return Ok(());
    }
    fs::rename(&source, &target).map_err(|error| {
        format!(
            "rename {} -> {}: {error}",
            source.display(),
            target.display()
        )
    })?;
    updated.push(target.strip_prefix(root).unwrap().display().to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_nix_version_replaces_existing_value() {
        let input = "  version = \"0.1.0\";\n";
        assert_eq!(set_nix_version(input, "0.2.0"), "  version = \"0.2.0\";\n");
    }

    #[test]
    fn set_homebrew_url_replaces_tagged_archive_url() {
        let input = "  url \"https://github.com/amkisko/bmx.rs/archive/refs/tags/v0.1.0.tar.gz\"\n";
        let expected =
            "  url \"https://github.com/amkisko/bmx.rs/archive/refs/tags/v0.2.0.tar.gz\"";
        assert_eq!(set_homebrew_url(input, "0.2.0"), format!("{expected}\n"));
    }
}
