# BMX Specification

## Overview

bmx is a language-agnostic CLI that installs, builds, and runs software from source repositories. Invoking `bmx <app-or-repo>` installs if needed and runs the app; all managed state lives in a local cache under the user’s home directory.

Commands: `bmx <app> [-- …]` (optional `--rm`, `--pin`, `-v` / `--verbose`); `bmx exec <app> [-- …]` or `bmx exec --pin` when `.bmx/pin` defines the app; `bmx install|uninstall|reinstall|self-update|update`; `bmx source set-default|show`; `bmx isolation set-default|show`; `bmx checkout set-default|show`; `bmx shim init|path|add`; `bmx doctor`.

bmx is not a language-specific package manager: no `package.json`, npm client, or bundled JavaScript runtime. Optional `[registries]` entries in `config.toml` are git URL aliases only, not npm/gem/cargo indices.

Not in this version: multi-version side-by-side management, a shared compile cache across apps, strong security sandboxing, parallel mass updates, or pluggable registry protocols beyond static URL prefixes in config.

## State layout and configuration file

Everything is under `~/.bmx`: `config.toml`, `apps/<app-id>/repo/`, and `apps/<app-id>/install.toml`.

```toml
default_source = "https://github.com"
build_isolation = "off"           # off | auto | docker | podman | nerdctl
checkout_backend = "git"       # git | gh | custom (git2 still accepted as legacy alias)
checkout_profiles = []
integrity_check = false           # if true, run/exec compare live HEAD to install.toml

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

## Source resolution and checkout

An app argument may be a repo URL, a repository path form, a bare name (combined with `default_source` as `<default>/<name>.git`), or `name@ref` where ref is a tag, semver range, or SHA. For **URL-like** sources (`…://…`, `git@…`), or short **`owner/repo`** paths (with at least one `/`), you may append `:cargo_package` before an optional `@ref` (examples: `https://github.com/org/proj.rs:my-bin@main`, `org/proj.rs:my-bin` with `default_source = "https://github.com"`) to select a Cargo workspace member: the clone URL omits that suffix, installs use a distinct app id `{repo_id}__{package}`, Rust builds run `cargo build --release -p <package>`, and the release binary for that package is preferred. This does not apply to bare single-segment names or `registry_key:repo` forms that contain no `/` before the package suffix (so `corp:widgets` stays a registry key, not `corp` + package `widgets`).

Checkout and sync use `checkout_backend` in config. Default is `git` (system `git` for clone, fetch, checkout). Older configs may still say `git2`; it means the same thing. `[[checkout_profiles]]` can match `match_prefix` on the resolved URL and override backend, SSH, env, proxy, or custom clone/sync commands.

## Install, run, and updates

Install resolves the source, clones or syncs the cache, checks out the requested revision when present, detects build strategy, runs the build, discovers the executable, and writes `install.toml`. Run installs when needed, reinstalls when the requested ref no longer matches metadata, runs the cached binary, and forwards its exit code. Uninstall removes `~/.bmx/apps/<app-id>/`. `bmx update [app]` updates one app or all installed apps. Self-update builds from a given source and replaces the running `bmx` binary (see platform notes below).

With `--rm`, bmx uses a throwaway temp directory as `BMX_HOME` (default-shaped config only; the real `~/.bmx` is not read) and deletes it after the command—useful for one-off runs without touching the persistent cache.

With global `--pin`, the app spec is read from the first `<project>/.bmx/pin` found by walking parents from the working directory; the file is one non-empty line with the same spec you would pass to `bmx install`.

The child process gets `PATH` with the managed executable’s directory prepended so the binary bmx selected wins over a same-named program elsewhere.

With `-v` / `--verbose`, bmx logs the canonical executable path, app id, and `resolved_commit` to stderr before launch.

## Builds, workdir, executable, hooks, and shims

Strategy detection order: `Cargo.toml` (Rust), `CMakeLists.txt`, `PKGBUILD`, `Brewfile` or root `.rb` formula, then `Makefile` / `makefile`. Rust uses `cargo build --release`, or `cargo build --release -p <name>` when the app spec includes `:name`; CMake uses configure + build in `build/` with Release; Make uses `make -j`; Homebrew uses `brew bundle` or `--build-from-source`; AUR prefers yay/paru else `makepkg`.

Repository root may contain `bmx.toml` with `[bmx] workdir` (relative, no `..`, must exist under the repo); detection and build run from that subdirectory. Optional `run` sets the executable path explicitly; otherwise Rust tries `target/release/<cargo_package_or_app_id_with_underscores>`, then scan `build/`, `bin/`, `target/release/`. Paths in metadata are relative to repo root even when `workdir` is set.

Optional `[bmx.hooks]`: `pre_run` runs from repo root before launching the installed binary; `post_install` runs after a successful install/reinstall.

`bmx shim init` creates `~/.bmx/shims`; `bmx shim path` prints a POSIX `PATH` export line; `bmx shim add <app>` adds a small stub that calls `bmx exec` with an absolute path to the current `bmx` binary. Shim generation is Unix-only; on Windows use `bmx` / `bmx.exe` directly.

```toml
[bmx]
workdir = "path/to/subdir"
run = "path/to/executable"

[bmx.hooks]
pre_run = "echo warmup"
post_install = "./tooling/post-install.sh"
```

## Build isolation

`config.toml` sets persistent isolation: `off` (host build), `auto` (try docker, then podman, then nerdctl), or a specific backend. When enabled, installs and updates run in the container until the setting changes. Image comes from `BMX_ISOLATION_IMAGE` or defaults to `ghcr.io/catthehacker/ubuntu:full-latest`. That default is Linux-oriented; artifacts may not run on macOS/Windows hosts without a matching target strategy.

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
