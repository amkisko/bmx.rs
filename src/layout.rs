use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::app_spec::{AppSpec, layout_id, parse_app_spec};
use crate::source::app_id;

#[derive(Debug, Clone)]
pub(crate) struct AppLayout {
    pub(crate) id: String,
    pub(crate) repo_dir: PathBuf,
    pub(crate) meta_file: PathBuf,
}

pub(crate) fn app_layout(home: &Path, app: &str) -> AppLayout {
    let id = app_id(app);
    app_layout_for_id(home, id)
}

pub(crate) fn app_layout_from_spec(home: &Path, spec: &AppSpec) -> AppLayout {
    let id = layout_id(spec);
    app_layout_for_id(home, id)
}

pub(crate) fn app_layout_for_id(home: &Path, id: impl Into<String>) -> AppLayout {
    let id = id.into();
    let app_root = home.join("apps").join(&id);
    AppLayout {
        id,
        repo_dir: app_root.join("repo"),
        meta_file: app_root.join("install.toml"),
    }
}

/// Resolve layout for run/exec/install when the user may refer to an install by `install.toml` `app` id (e.g. `bmx install … --as`).
pub(crate) fn resolve_layout_for_run(home: &Path, input: &str) -> AppLayout {
    let spec = parse_app_spec(input);
    let from_spec = app_layout_from_spec(home, &spec);
    if from_spec.meta_file.exists() {
        return from_spec;
    }
    let direct = app_layout_for_id(home, spec.source.clone());
    if direct.meta_file.exists() {
        return direct;
    }
    let sanitized = app_layout(home, &spec.source);
    if sanitized.meta_file.exists() {
        return sanitized;
    }
    from_spec
}

pub(crate) fn resolve_installed_layout(home: &Path, input: &str) -> Result<AppLayout> {
    let layout = resolve_layout_for_run(home, input);
    if layout.meta_file.exists() {
        return Ok(layout);
    }
    Err(anyhow!(
        "not installed (no {}); run `bmx install` first",
        layout.meta_file.display()
    ))
}

pub(crate) fn sanitize_install_as(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("`--as` name must be non-empty");
    }
    let out: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect();
    if out.is_empty() {
        anyhow::bail!("`--as` name must contain at least one letter or digit");
    }
    Ok(out)
}
