use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;

use crate::config::load_config;
use crate::io::read_toml;
use crate::process::has_tool;
use crate::types::InstallMetadata;

#[allow(clippy::collapsible_if)]
pub(crate) fn doctor(home: &Path) -> Result<()> {
    let cfg = load_config(home)?;
    println!("bmx home: {}", home.display());
    println!("checkout/sync: system `git` CLI (unless gh/custom profile)");
    println!(
        "default source: {}",
        cfg.default_source
            .clone()
            .unwrap_or_else(|| "(not set)".to_string())
    );
    println!("build isolation: {}", cfg.build_isolation.as_str());
    println!("checkout backend: {}", cfg.checkout_backend.as_str());
    println!("integrity_check (config): {}", cfg.integrity_check);
    println!("registries defined: {}", cfg.registries.len());

    if let Ok(out) = Command::new("git").arg("--version").output() {
        if out.status.success() {
            print!("{}", String::from_utf8_lossy(&out.stdout));
        }
    }

    let apps = home.join("apps");
    let mut approx_bytes: u64 = 0;
    let mut broken: Vec<String> = Vec::new();
    if apps.is_dir() {
        for e in fs::read_dir(&apps)? {
            let p = e?.path();
            if !p.is_dir() {
                continue;
            }
            approx_bytes += du_dir_bytes(&p)?;
            let meta = p.join("install.toml");
            if !meta.exists() {
                continue;
            }
            let ok = match read_toml::<InstallMetadata>(&meta) {
                Ok(m) => p.join("repo").join(&m.executable_rel).exists(),
                Err(_) => false,
            };
            if !ok {
                if let Some(id) = p.file_name().and_then(|n| n.to_str()) {
                    broken.push(id.to_string());
                }
            }
        }
    }
    println!(
        "approx cache under {}/apps: {} bytes",
        home.display(),
        approx_bytes
    );
    if !broken.is_empty() {
        println!(
            "possible broken installs (bad install.toml or missing binary): {}",
            broken.join(", ")
        );
    }

    for tool in [
        "cargo", "cmake", "make", "brew", "yay", "paru", "makepkg", "docker", "podman", "nerdctl",
        "git", "gh",
    ] {
        println!("{tool}: {}", if has_tool(tool) { "ok" } else { "missing" });
    }
    Ok(())
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
