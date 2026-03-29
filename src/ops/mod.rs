mod clean;
mod doctor;
mod install;
mod package;
mod reinstall_rebuild;
mod run;
mod selfexe;
mod uninstall;
mod update;

pub(crate) use clean::clean_temp_artifacts;
pub(crate) use doctor::doctor;
pub(crate) use install::install_app;
pub(crate) use package::show_package;
pub(crate) use reinstall_rebuild::{rebuild_app, reinstall_app};
pub(crate) use run::run_app;
pub(crate) use selfexe::self_update;
pub(crate) use uninstall::uninstall_app;
pub(crate) use update::update_all;

#[derive(Clone, Copy)]
pub(crate) struct TrustOptions {
    pub(crate) prompt: bool,
    pub(crate) global: bool,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::package::list_files_under;
    use super::run::path_for_child;

    #[test]
    fn path_for_child_prefixes_parent_to_path() {
        let td = tempdir().unwrap();
        let exe = td.path().join("bin").join("tool");
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&exe, "#!/bin/sh\n").unwrap();
        let p = path_for_child(&exe).unwrap();
        let s = p.to_string_lossy().to_string();
        assert!(s.starts_with(&format!("{}:", exe.parent().unwrap().display())));
    }

    #[test]
    fn list_files_under_returns_sorted_relative_paths() {
        let td = tempdir().unwrap();
        fs::create_dir_all(td.path().join("a")).unwrap();
        fs::write(td.path().join("z.txt"), "z").unwrap();
        fs::write(td.path().join("a").join("b.txt"), "b").unwrap();
        let files = list_files_under(td.path()).unwrap();
        let names: Vec<String> = files.iter().map(|p| p.display().to_string()).collect();
        assert_eq!(names, vec!["a/b.txt".to_string(), "z.txt".to_string()]);
    }
}
