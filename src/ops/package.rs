use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::layout::resolve_installed_layout;

pub(crate) fn show_package(home: &Path, app: &str, would_remove: bool) -> Result<()> {
    let layout = resolve_installed_layout(home, app)?;
    let app_root = layout
        .repo_dir
        .parent()
        .ok_or_else(|| anyhow!("invalid app layout for {}", layout.id))?;
    let root = if would_remove {
        app_root
    } else {
        &layout.repo_dir
    };
    let files = list_files_under(root)?;
    for rel in files {
        println!("{}", rel.display());
    }
    Ok(())
}

pub(crate) fn list_files_under(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    fn walk(dir: &Path, base: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
        for e in fs::read_dir(dir)? {
            let p = e?.path();
            if p.is_dir() {
                walk(&p, base, out)?;
            } else {
                let rel = p.strip_prefix(base).unwrap_or(&p).to_path_buf();
                out.push(rel);
            }
        }
        Ok(())
    }
    if root.is_dir() {
        walk(root, root, &mut out)?;
    }
    out.sort();
    Ok(out)
}
