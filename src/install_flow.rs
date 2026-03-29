use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};

use crate::app_spec::app_spec_from_install_meta;
use crate::build::{build_with_strategy, detect_strategy};
use crate::config::load_config;
use crate::executable::detect_executable_rel;
use crate::io::{read_toml, write_toml};
use crate::layout::app_layout;
use crate::repo::sync_repo;
use crate::revision::checkout_requested_ref;
use crate::types::{BuildStrategy, InstallMetadata};

/// Rebuild the cached worktree using saved metadata without fetching (for undo / local recovery).
pub(crate) fn rebuild_worktree_from_saved_metadata(
    home: &Path,
    install: &InstallMetadata,
    cli_verbose: bool,
) -> Result<()> {
    let cfg = load_config(home)?;
    let layout = crate::layout::app_layout(home, &install.app);
    let show_output = crate::runtime::subprocess_output_visible(cli_verbose);
    let strategy =
        BuildStrategy::parse(&install.strategy).or_else(|| detect_strategy(&layout.repo_dir));
    let strategy = strategy.ok_or_else(|| {
        anyhow!(
            "unable to determine build strategy for {}",
            layout.repo_dir.display()
        )
    })?;
    let spec = app_spec_from_install_meta(
        &install.source_url,
        install.requested_ref.clone(),
        install.rust_package.clone(),
    );
    build_with_strategy(
        &strategy,
        &layout.repo_dir,
        cfg.build_isolation,
        show_output,
        spec.cargo_package.as_deref(),
    )?;
    let executable_rel = detect_executable_rel(&layout.repo_dir, &spec)?;
    let resolved_commit =
        crate::git_cmd::git_output(&layout.repo_dir, &["rev-parse", "HEAD"], &[]).ok();
    write_toml(
        &layout.meta_file,
        &InstallMetadata {
            app: install.app.clone(),
            source_url: install.source_url.clone(),
            executable_rel,
            strategy: strategy.as_str().to_string(),
            requested_ref: install.requested_ref.clone(),
            resolved_commit: resolved_commit.filter(|s| !s.is_empty()),
            rust_package: install.rust_package.clone(),
        },
    )?;
    crate::hooks::run_post_install_hook(&layout.repo_dir)?;
    Ok(())
}

pub(crate) fn reinstall_from_meta(
    home: &Path,
    install: InstallMetadata,
    cli_verbose: bool,
) -> Result<()> {
    let cfg = load_config(home)?;
    let layout = app_layout(home, &install.app);
    fs::create_dir_all(&layout.repo_dir)?;
    let show_output = crate::runtime::subprocess_output_visible(cli_verbose);
    sync_repo(&install.source_url, &layout.repo_dir, &cfg, cli_verbose)?;
    let resolved_commit = checkout_requested_ref(
        &layout.repo_dir,
        install.requested_ref.as_deref(),
        cli_verbose,
    )?;

    let strategy = detect_strategy(&layout.repo_dir).ok_or_else(|| {
        anyhow!(
            "unable to detect build strategy for {}",
            layout.repo_dir.display()
        )
    })?;
    let spec = app_spec_from_install_meta(
        &install.source_url,
        install.requested_ref.clone(),
        install.rust_package.clone(),
    );
    build_with_strategy(
        &strategy,
        &layout.repo_dir,
        cfg.build_isolation,
        show_output,
        spec.cargo_package.as_deref(),
    )?;
    let executable_rel = detect_executable_rel(&layout.repo_dir, &spec)?;

    write_toml(
        &layout.meta_file,
        &InstallMetadata {
            app: install.app,
            source_url: install.source_url,
            executable_rel,
            strategy: strategy.as_str().to_string(),
            requested_ref: install.requested_ref,
            resolved_commit,
            rust_package: install.rust_package,
        },
    )?;
    crate::hooks::run_post_install_hook(&layout.repo_dir)?;
    Ok(())
}

pub(crate) fn must_reinstall(meta_file: &Path, requested_ref: &Option<String>) -> Result<bool> {
    if !meta_file.exists() {
        return Ok(true);
    }
    let metadata: InstallMetadata = read_toml(meta_file)?;
    Ok(metadata.requested_ref.as_deref() != requested_ref.as_deref())
}
