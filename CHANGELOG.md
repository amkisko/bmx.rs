# Changelog

## Unreleased

- Contain `bmx.run` / executable paths under the install tree (reject absolute paths and `..`).
- Sync OpenSSH public keys from trust policy into per-source `allowedSignersFile`; document fingerprint-only limitation.
- Typed process exit codes (usage/trust/network/io) with `cli_error` hints; `bmx completions` and `bmx man`.
- HTTP fetches use a shared ureq agent with timeout, redirect cap, https-only, and body size limit.
- Defaults: `integrity_check = true`, `hooks_enabled = false`; consent no longer skips on local good signature alone; optional `BMX_TRUST_REQUIRED`.
- Release tooling: `usr/bin/release` (`sync-packaging`, `check-loc`), Makefile, CI gates; packaging installs completions/man.
- Global `--no-input` for non-interactive runs; trust auto-consent still needs `BMX_TRUST_ASSUME_YES=1`.
- `bmx uninstall` confirms on a TTY; use `--force` in scripts or with `--no-input`.
- `--json` on `history`, `search`, `doctor`, and `trust list`.
- Bare `bmx` prints concise help; `--version`; typo suggestions before treating unknown names as apps.

## 0.1.3 (2026-03-30)

- Added runtime isolation in addition to build isolation: apps can now run through `docker` / `podman` / `nerdctl` backends.
- New persistent config key `run_isolation` with CLI support: `bmx isolation set-default-run MODE` and `bmx isolation show-run`.
- New per-invocation runtime overrides: `--isolate-run` (force isolated run with auto backend resolution) and `--no-isolate-run` (force host run).
- `bmx doctor` now reports both build and run isolation defaults.

## 0.1.2 (2026-03-29)

- `bmx trust remove-key` (alias `bmx trust revoke`) removes a signer from the same default or `--match-prefix` scope as `add-key`.
- New `bmx clean [--dry-run]` command removes leftover bmx temp directories (`bmx-ephemeral-*`, `bmx-trust-import-*`, `bmx-trust-check-*`) under the system temp folder.
- New `bmx rebuild [--install] [APP]` command: rebuilds with explicit ref support (`APP@ref`), uses pinned ref when present, and works with `--pin`.
- New trust policy support in `~/.bmx/trust.toml`: source allow/deny rules, optional signed-commit enforcement, and signer allowlists.
- New trust management commands: `bmx trust show|add-key|set-signed|set-allow|import-repo`.
- New global `--trust` consent flow for install/update/rebuild/reinstall/self-update: shows concise HEAD signer details and asks before importing signer keys.
- `--rm` now correctly applies to `bmx shim` commands (no persistent `~/.bmx` writes in ephemeral mode).
- `--rm` seeds ephemeral `BMX_HOME` from the real `~/.bmx/trust.toml` and `~/.bmx/trust/` so trust policy (allowlists, deny rules, SSH allowed signers) still applies; `config.toml` remains isolated.
- Ref pin persistence tightened: refs are persisted only when `--pin` is explicitly used.

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
