use std::path::{Path, PathBuf};

use crate::app_spec::{AppSpec, layout_id};
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

fn app_layout_for_id(home: &Path, id: String) -> AppLayout {
    let app_root = home.join("apps").join(&id);
    AppLayout {
        id,
        repo_dir: app_root.join("repo"),
        meta_file: app_root.join("install.toml"),
    }
}
