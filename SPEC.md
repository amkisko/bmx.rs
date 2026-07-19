# BMX Specification

## Overview

bmx is a language-agnostic CLI that installs, builds, and runs software from source repositories. Invoking `bmx <app-or-repo>` installs if needed and runs the app; all managed state lives in a local cache under the user’s home directory.

Commands: `bmx <app> [-- …]` (optional `--rm`, `--pin`, `--trust`, `--global`, `--no-input`, `-v` / `--verbose`, `--isolate-run`, `--no-isolate-run`); `bmx exec <app> [-- …]` or `bmx exec --pin` when `.bmx/pins.toml` (or legacy `.bmx/pin`) defines the app; `bmx install [--as NAME]|rebuild [--install] [APP]|uninstall [--force]|reinstall|self-update|update`; `bmx show <app> [--would-remove]`; `bmx search [WORDS …] [--backend auto|github|gitlab|aur|homebrew] [-n N] [--forks] [--url] [--no-probe] [--json]`; `bmx history [-n N] [--json]`; `bmx undo [ID] [--only APP]`; `bmx source set-default|show`; `bmx isolation set-default|show|set-default-run|show-run`; `bmx checkout set-default|show`; `bmx trust list [--json]|show|add-key|remove-key|set-signed|set-allow|import-repo|check` (`remove-key` has alias `revoke`); `bmx shim init|path|add`; `bmx doctor [--json]`; `bmx clean [--dry-run]` (delete leftover `bmx-*` dirs under the system temp folder).

Search: discovers installable sources without cloning. `--backend auto` uses the GitHub or GitLab host from `default_source` (or `BMX_GITHUB_API_BASE`). GitHub/GitLab hits are optionally filtered with `--no-probe` off (default): one REST call per candidate lists the repository root; results must contain a root file bmx already recognizes (`Cargo.toml`, `CMakeLists.txt`, `PKGBUILD`, `Brewfile`, `Makefile`/`makefile`, `bmx.toml`, or a `.rb` formula stub). `--backend aur` queries the AUR RPC (clone URL `https://aur.archlinux.org/<PackageBase>.git`, always PKGBUILD-based). `--backend homebrew` reads `formulae.brew.sh/api/formula.json` and keeps formulas whose homepage (or stable tarball URL) maps to a GitHub/GitLab clone URL bmx can use.

bmx is not a language-specific package manager: no `package.json`, npm client, or bundled JavaScript runtime. Optional `[registries]` entries in `config.toml` are git URL aliases only, not npm/gem/cargo indices.

Not in this version: multi-version side-by-side management, a shared compile cache across apps, strong security sandboxing, parallel mass updates, or pluggable registry protocols beyond static URL prefixes in config.

## State layout and configuration file

Everything is under `~/.bmx`: `config.toml`, `apps/<app-id>/repo/`, and `apps/<app-id>/install.toml`. Optional: `audit.log.jsonl` (append-only action log), `undo-stack.json` (last undoable operations, capped), and `snapshots/` (files captured before mutating installs).

```toml
default_source = "https://github.com"
self_update_source = "amkisko/bmx.rs" # source used by `bmx self-update` (env override: BMX_SELF_UPDATE_SOURCE)
build_isolation = "off"           # off | auto | docker | podman | nerdctl
run_isolation = "off"             # off | auto | docker | podman | nerdctl
checkout_backend = "git"       # git | gh | custom (git2 still accepted as legacy alias)
checkout_profiles = []
integrity_check = true            # run/exec compare live HEAD to install.toml (fail closed)
hooks_enabled = false             # when true, run bmx.toml pre_run / post_install shell hooks

[registries]
# corp = "https://git.corp.example"   # bmx install corp:widgets -> that URL.git
```

```toml
app = "ripgrep"
source_url = "https://github.com/BurntSushi/ripgrep.git"
executable_rel = "target/release/rg"
strategy = "rust-cargo"
requested_ref = "^1.0"
resolved_commit = "7f3d5c1..."
```

Optional trust policy (`~/.bmx/trust.toml`) can allow/deny source prefixes and require signed commits:

```toml
[default]
allow = true
require_signed_commit = false
allowed_signing_keys = []      # key ids or fingerprints (optional)

[[rules]]
match_prefix = "https://github.com/acme/"
allow = true
require_signed_commit = true
allowed_signing_keys = ["ABCD1234EF567890"]

[[rules]]
match_prefix = "https://github.com/untrusted/"
allow = false
```

CLI helpers mutate this file:

- `bmx trust list` prints human-readable trust state (global + local by default).
  - add top-level `--global` to show only global/default scope
  - add `--local` to show only local/prefix rules
  - pass an app (`bmx trust list APP`) to filter by that installed app's source URL
- `bmx trust show` prints effective policy TOML (defaults if file is absent).
- `bmx trust add-key KEY [--match-prefix PREFIX]` appends signer key/fingerprint to `[default]` or a prefix rule.
- `bmx trust set-signed --match-prefix PREFIX [--enabled true|false]` toggles `require_signed_commit`.
- `bmx trust set-allow --match-prefix PREFIX [--allow true|false]` toggles allow/deny.
- `bmx trust import-repo APP [--match-prefix PREFIX]` imports signer key/fingerprint from HEAD of an installed app repo.
- `bmx trust check [SOURCE]` compares all trusted keys against a compromised-key feed; if any trusted key is found in the feed, command exits non-zero.
  - `SOURCE` supports git URL or direct `.toml`/`.txt` URL (and local/file paths for testing).
  - for git sources, bmx looks for `compromised-keys.toml` / `compromised_keys.toml` (or same names under `trust/`) and `.txt` variants.
  - TOML feed format:

```toml
[[keys]]
value = "SHA256:EXAMPLEKEY..."
reason = "private key exposure"
reference = "https://example.com/advisory/123"
reported_at = "2026-03-29"
```

## Source resolution and checkout

An app argument may be a repo URL, a repository path form, a host shortcut (`host.tld/owner/repo`), a bare name (combined with `default_source` as `<default>/<name>.git`), or `name@ref` where ref is a tag, semver range, or SHA. For URL-like sources (`…://…`, `git@…`, `host.tld/...`), or short `owner/repo` paths (with at least one `/`), you may append `:cargo_package` before an optional `@ref` (examples: `https://github.com/org/proj.rs:my-bin@main`, `org/proj.rs:my-bin` with `default_source = "https://github.com"`) to select a Cargo workspace member: the clone URL omits that suffix, installs use a distinct app id `{repo_id}__{package}`, Rust builds run `cargo build --release -p <package>`, and the release binary for that package is preferred. This does not apply to bare single-segment names or `registry_key:repo` forms that contain no `/` before the package suffix (so `corp:widgets` stays a registry key, not `corp` + package `widgets`).

Checkout and sync use `checkout_backend` in config. Default is `git` (system `git` for clone, fetch, checkout). Older configs may still say `git2`; it means the same thing. `[[checkout_profiles]]` can match `match_prefix` on the resolved URL and override backend, SSH, env, proxy, or custom clone/sync commands.

## Install, run, and updates

Install resolves the source, clones or syncs the cache, checks out the requested revision when present, detects build strategy, runs the build, discovers the executable, and writes `install.toml`. Run installs when needed, reinstalls when the requested ref no longer matches metadata, then executes the cached binary on host or in runtime isolation (`run_isolation`, or per-run `--isolate-run` / `--no-isolate-run`) and forwards its exit code. Uninstall removes `~/.bmx/apps/<app-id>/`. `bmx update [app]` with no `app` argument runs `sync + checkout + build` for every `install.toml` under `apps/`; with an app argument it does the same for that install only. Self-update builds from a given source and replaces the running `bmx` binary (see platform notes below).

When `~/.bmx/trust.toml` exists, installs/reinstalls/runs enforce the matching trust rule (longest `match_prefix` wins, else `[default]`): `allow = false` blocks the source; `require_signed_commit = true` requires `git verify-commit HEAD` to pass; if `allowed_signing_keys` is non-empty, the commit signer key/fingerprint must match an entry.

`--trust` on install/update/reinstall/self-update does **not** auto-trust silently: bmx displays concise human-review details from HEAD (source URL, commit, author, date, subject, signature status, signer identity, key id/fingerprint, and candidate key values) and asks for explicit consent before writing any keys to `trust.toml`. By default those imports are source-scoped (`match_prefix = <source_url>`); adding `--global --trust` imports into `[default]` instead. For install/update/rebuild/reinstall/self-update, `--global` requires `--trust`.

Independent of `--trust`, installs/updates/rebuilds/reinstalls/self-updates prompt for consent before proceeding with untrusted sources (no verified-good signature and signer not already trusted by policy).

`--no-input` disables interactive prompts. Trust consent still requires `BMX_TRUST_ASSUME_YES=1` (it is not implied by `--no-input`). `bmx uninstall` confirms on a TTY; without a TTY or with `--no-input`, pass `--force`. `history`, `search`, `doctor`, and `trust list` accept `--json` for machine-readable stdout.

For SSH-signed commits, bmx uses per-source `allowedSignersFile` values under `$BMX_HOME/trust/allowed_signers/*.signers` for its own git subprocesses only; it does not modify user/global git configuration. Entries that look like OpenSSH public keys in `allowed_signing_keys` are written into that file; fingerprints/key ids alone are commented and used only for post-verify allowlist matching. Set `BMX_TRUST_REQUIRED=1` to refuse install/run when `trust.toml` is missing.

`bmx install APP --as NAME` stores the install under `apps/<NAME>/` and records `app = "<NAME>"` in `install.toml`. Use this when two different sources would map to the same default id, or when you want a second checkout of the same repo. Installing into an existing directory with a different resolved `source_url` fails with a hint to pick another `--as` or uninstall first.

`bmx reinstall APP` re-fetches, re-checks out per `install.toml`, and rebuilds without deleting the cache directory (developer workflow after local edits in the clone).

`bmx show APP` lists files under the cached repository (`apps/<id>/repo/`). `bmx show APP --would-remove` lists all paths that `bmx uninstall APP` would delete (including `install.toml`). The global `--rm` flag still means ephemeral `BMX_HOME` only; prefer `--would-remove` for uninstall previews.

## History and undo

`bmx history` prints recent entries from `audit.log.jsonl` (id, unix time, kind, undoable flag, summary, and `source_url` when recorded (the resolved git remote from `install.toml`). Older log lines without `source_url` deserialize with that column empty. `--rm` runs do not write history or undo data (ephemeral home only).

Undoable steps record a marker in `undo-stack.json` and (for mutations of existing installs) a directory under `snapshots/` holding the previous `install.toml` and `git_head.txt` (HEAD before the operation):

- Fresh `install`: undo removes `apps/<id>/` entirely.
- `update`, `reinstall`, `install` that refreshes an existing app, or `update` with no app (`update all`): undo restores saved metadata, `git checkout --force` to the saved commit when available, and rebuilds without fetching.

`bmx undo` reverses the top frame on the stack. `bmx undo <id>` reverses the frame with that id only if it is the top frame (undo newer steps first). `bmx undo --only <app>` (same app forms as `bmx install`) rolls back one install from the top frame only—useful after `bmx update` touched every app: other apps from that batch stay updated until you `bmx undo --only` them too or run `bmx undo` to revert the rest at once. Partial undo does not apply to a lone fresh `install` frame (use full `bmx undo`). Uninstall and self-update are logged with `undoable: false` (no automatic rollback). The stack is capped (oldest frames and their snapshot dirs are dropped).

With `--rm`, bmx uses a throwaway temp directory as `BMX_HOME` and deletes it after the command—useful for one-off runs without touching the persistent app cache. Ephemeral mode starts from default `config.toml` only, but copies **`~/.bmx/trust.toml`** and **`~/.bmx/trust/`** into the temp home at startup so trust policy (allowlists, deny rules, per-source SSH allowed signers files) still applies.

Project pins live under `.bmx/`. Preferred: `.bmx/pins.toml` — TOML with optional `default = "<layout-id>"` and an `[apps]` table mapping layout ids (same as `~/.bmx/apps/<id>/`, from `layout_id` / `bmx install` resolution) to a full install spec string (`owner/repo@ref`, URL, etc.). Legacy: a single-line `.bmx/pin` file (one spec) is still read and migrated to `pins.toml` on the next `--pin` write.

Walking parents from the working directory, bmx uses the first project that has `pins.toml` or legacy `pin`. With `--pin` and no app argument: if `[apps]` has one entry, that spec is used; if several, `default` must name a layout id present in `[apps]`, or bmx errors with the list of keys. With `--pin` and an explicit app spec, bmx merges into `./.bmx/pins.toml` in the current working directory only: it sets `apps[<layout-id>] = <spec>` (replacing any prior spec for the same app), sets `default` to that id when it was the first and only app, and removes legacy `.bmx/pin` after writing. Pinning a second app does not remove the first; change `default` in the file to pick which app `bmx --pin` runs when multiple are listed.

The child process gets `PATH` with the managed executable’s directory prepended so the binary bmx selected wins over a same-named program elsewhere.

With `-v` / `--verbose`, bmx logs the canonical executable path, app id, and `resolved_commit` to stderr before launch.

## Builds, workdir, executable, hooks, and shims

Build stacks are implemented as an internal plugin registry (Rust, CMake, AUR, Homebrew, Make) tried in that detection order. Repository root may include `bmx.toml` with `[bmx] strategy = "<id>"` where `<id>` is `rust-cargo`, `cmake`, `make`, `homebrew`, or `aur`, to force a plugin instead of auto-detect (still uses the same `workdir` rules below).

Rust uses `cargo build --release`, or `cargo build --release -p <name>` when the app spec includes `:name`; CMake uses configure + build in `build/` with Release; Make uses `make -j`; Homebrew uses `brew bundle` or `--build-from-source`; AUR prefers yay/paru else `makepkg`.

Repository root may contain `bmx.toml` with `[bmx] workdir` (relative, no `..`, must exist under the repo); detection and build run from that subdirectory. Optional `run` sets the executable path explicitly; otherwise Rust tries `target/release/<cargo_package_or_app_id_with_underscores>`, then scan `build/`, `bin/`, `target/release/`. Paths in metadata are relative to repo root even when `workdir` is set.

`bmx.run` and `bmx.workdir` must be relative paths under the repo (no absolute paths or `..`). Optional `[bmx.hooks]`: `pre_run` / `post_install` run only when `hooks_enabled = true` in config.

`bmx shim init` creates `~/.bmx/shims`; `bmx shim path` prints a POSIX `PATH` export line; `bmx shim add <app>` adds a small stub that calls `bmx exec` with an absolute path to the current `bmx` binary. Shim generation is Unix-only; on Windows use `bmx` / `bmx.exe` directly.

```toml
[bmx]
workdir = "path/to/subdir"
run = "path/to/executable"
# strategy = "make"

[bmx.hooks]
pre_run = "echo warmup"
post_install = "./tooling/post-install.sh"
```

## Build isolation

`config.toml` sets persistent isolation: `off` (host build), `auto` (try docker, then podman, then nerdctl), or a specific backend. When enabled, installs and updates run in the container until the setting changes. Image comes from `BMX_ISOLATION_IMAGE` or defaults to `ghcr.io/catthehacker/ubuntu:full-latest`. That default is Linux-oriented; artifacts may not run on macOS/Windows hosts without a matching target strategy.

## Run isolation

`config.toml` also supports `run_isolation` with the same modes (`off`, `auto`, `docker`, `podman`, `nerdctl`). Set it persistently with `bmx isolation set-default-run MODE`; inspect with `bmx isolation show-run`. Per invocation, `--isolate-run` forces `auto` runtime isolation and `--no-isolate-run` forces host execution.

## Checkout backends and profiles

Backends: `git` uses the system git CLI; `gh` uses `gh repo clone` then git; `custom` uses profile-defined commands. Templates may use `{source_url}` and `{repo_dir}`.

```toml
checkout_backend = "git"

[[checkout_profiles]]
name = "github-corp"
match_prefix = "https://github.com/acme/"
backend = "git"
ssh_command = "ssh -i ~/.ssh/acme_id -o IdentitiesOnly=yes"
https_proxy = "http://proxy.local:8443"
no_proxy = "localhost,127.0.0.1,.internal"

[[checkout_profiles.env]]
key = "GH_TOKEN"
value = "ghp_example_token"

[[checkout_profiles]]
name = "legacy-vcs-bridge"
match_prefix = "ssh://legacy.example.com/"
backend = "custom"
custom_clone = "my-checkout clone {source_url} {repo_dir}"
custom_sync = "my-checkout sync {repo_dir}"
```

## Self-update on disk

Unix: stage the new binary and rename it over the current executable. Windows: replacement of the running exe is deferred until after exit, then applied.

## Errors, security, doctor, and future work

Errors should be actionable for resolution, sync, build, executable detection, and missing tools. Preserve cache directories except when uninstall removes an app.

The trust model is explicit: third-party source is checked out and built on the machine. Harder guarantees (signatures, allowlists, sandboxed execution, deterministic isolation) are future work.

`bmx doctor` summarizes home path, checkout backend note, default source, isolation mode, `integrity_check`, registry count, `git --version` when present, approximate size under `apps/`, installs that look broken, and common build-tool availability.

Near-term direction: more integration coverage, clearer reproducibility for last-installed revisions, richer executable resolution from ecosystem metadata. Parallel batch updates and shared cross-app caches remain unimplemented.
