# Changelog

## 0.1.1 (2026-03-29)

- `bmx install --as` and `bmx show` (list files under an install; `--would-remove` previews what uninstall would delete); build and install layout handling improvements.
- `bmx history` and `bmx undo` for install/update/reinstall flows, with audit log entries and snapshot-based rollback.
- README: remote bootstrap for `setup.sh` using `curl` and a direct GitHub raw URL.
- Trunk and linter configuration (`.trunk/`); GitHub Actions workflow runs trunk checks on push and pull requests.
- `bmx update <app>` requires an existing install and fails otherwise; use `bmx install` for a first-time install.

## 0.1.0 (2026-03-28)

Initial release.

- CLI to install, build, run, update, reinstall, and uninstall apps from git sources; default command runs after install-if-needed.
- Config and cache under `~/.bmx` (`config.toml`, per-app repo cache and `install.toml`).
- Source resolution: URLs, repo refs, bare names against a configurable default base, and ref-qualified inputs (`app@ref`).
- Build strategy detection (Cargo, CMake, Make, Homebrew-style, AUR) with optional `bmx.toml` workdir and explicit run path.
- Configurable build isolation: off, auto, or a specific OCI backend (`docker`, `podman`, `nerdctl`).
- Subcommands: `exec`, `install`, `uninstall`, `reinstall`, `self-update`, `update`, `source`, `isolation`, `checkout`, `shim`, `doctor`; `source`, `isolation`, and `checkout` each include `show` and `set-default` for defaults.
- `setup.sh` for building from source and installing the `bmx` binary.
- Specification and contributor docs in the repository root.
