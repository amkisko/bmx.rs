# Changelog

## 0.1.0 (2026-03-28)

Initial release.

- CLI to install, build, run, update, reinstall, and uninstall apps from git sources; default command runs after install-if-needed.
- Config and cache under `~/.bmx` (`config.toml`, per-app repo cache and `install.toml`).
- Source resolution: URLs, repo refs, bare names against a configurable default base, and ref-qualified inputs (`app@ref`).
- Build strategy detection (Cargo, CMake, Make, Homebrew-style, AUR) with optional `bmx.toml` workdir and explicit run path.
- Configurable build isolation: off, auto, or a specific OCI backend (`docker`, `podman`, `nerdctl`).
- Subcommands: `exec`, `install`, `uninstall`, `reinstall`, `update`, `source`, `isolation`, `doctor`.
- `setup.sh` for building from source and installing the `bmx` binary.
- Specification and contributor docs in the repository root.
