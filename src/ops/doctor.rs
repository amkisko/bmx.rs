use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use serde::Serialize;

use crate::config::load_config;
use crate::io::read_toml;
use crate::process::has_tool;
use crate::types::InstallMetadata;

#[derive(Debug, Serialize)]
struct DoctorReport {
    home: String,
    checkout_sync: &'static str,
    default_source: String,
    build_isolation: String,
    run_isolation: String,
    checkout_backend: String,
    integrity_check: bool,
    registries: usize,
    git_version: Option<String>,
    approx_apps_bytes: u64,
    broken_installs: Vec<String>,
    tools: BTreeMap<&'static str, &'static str>,
}

pub(crate) fn doctor(home: &Path, json: bool) -> Result<()> {
    let report = collect_doctor_report(home)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    print_doctor_report(&report);
    Ok(())
}

fn collect_doctor_report(home: &Path) -> Result<DoctorReport> {
    let cfg = load_config(home)?;
    let git_version = Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string());

    let apps = home.join("apps");
    let mut approx_bytes: u64 = 0;
    let mut broken: Vec<String> = Vec::new();
    if apps.is_dir() {
        for entry in fs::read_dir(&apps)? {
            let path = entry?.path();
            if !path.is_dir() {
                continue;
            }
            approx_bytes += du_dir_bytes(&path)?;
            let meta = path.join("install.toml");
            if !meta.exists() {
                continue;
            }
            let ok = match read_toml::<InstallMetadata>(&meta) {
                Ok(m) => path.join("repo").join(&m.executable_rel).exists(),
                Err(_) => false,
            };
            if !ok && let Some(id) = path.file_name().and_then(|n| n.to_str()) {
                broken.push(id.to_string());
            }
        }
    }

    let mut tools = BTreeMap::new();
    for tool in [
        "cargo", "cmake", "make", "brew", "yay", "paru", "makepkg", "docker", "podman", "nerdctl",
        "git", "gh",
    ] {
        tools.insert(tool, if has_tool(tool) { "ok" } else { "missing" });
    }

    Ok(DoctorReport {
        home: home.display().to_string(),
        checkout_sync: "system `git` CLI (unless gh/custom profile)",
        default_source: cfg
            .default_source
            .clone()
            .unwrap_or_else(|| "(not set)".to_string()),
        build_isolation: cfg.build_isolation.as_str().to_string(),
        run_isolation: cfg.run_isolation.as_str().to_string(),
        checkout_backend: cfg.checkout_backend.as_str().to_string(),
        integrity_check: cfg.integrity_check,
        registries: cfg.registries.len(),
        git_version,
        approx_apps_bytes: approx_bytes,
        broken_installs: broken,
        tools,
    })
}

fn print_doctor_report(report: &DoctorReport) {
    println!("bmx home: {}", report.home);
    println!("checkout/sync: {}", report.checkout_sync);
    println!("default source: {}", report.default_source);
    println!("build isolation: {}", report.build_isolation);
    println!("run isolation: {}", report.run_isolation);
    println!("checkout backend: {}", report.checkout_backend);
    println!("integrity_check (config): {}", report.integrity_check);
    println!("registries defined: {}", report.registries);
    if let Some(version) = &report.git_version {
        println!("{version}");
    }
    println!(
        "approx cache under {}/apps: {} bytes",
        report.home, report.approx_apps_bytes
    );
    if !report.broken_installs.is_empty() {
        println!(
            "possible broken installs (bad install.toml or missing binary): {}",
            report.broken_installs.join(", ")
        );
    }
    for (tool, status) in &report.tools {
        println!("{tool}: {status}");
    }
}

fn du_dir_bytes(path: &Path) -> Result<u64> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() {
        return Ok(meta.len());
    }
    let mut n = 0u64;
    for e in fs::read_dir(path)? {
        n += du_dir_bytes(&e?.path())?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::du_dir_bytes;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn du_dir_bytes_sums_file_sizes() {
        let td = tempdir().unwrap();
        fs::create_dir_all(td.path().join("d")).unwrap();
        fs::write(td.path().join("a"), "1234").unwrap();
        fs::write(td.path().join("d").join("b"), "12").unwrap();
        let n = du_dir_bytes(td.path()).unwrap();
        assert!(n >= 6);
    }
}
