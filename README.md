# bmx

[![Test Status](https://github.com/amkisko/bmx.rs/actions/workflows/test.yml/badge.svg)](https://github.com/amkisko/bmx.rs/actions/workflows/test.yml)

Command-line tool that installs, builds, and runs software from source repositories. The usual shape is: you name an app or repo, bmx makes sure it is built and cached locally, then runs it. State lives under `~/.bmx` so everything stays on your machine.

For behaviour, commands, storage layout, and limits, see [SPEC.md](SPEC.md).

## Build

From a checkout:

```bash
cargo build
```

The package library crate is `bmx_rs`; the installed binary is `bmx` (see `target/release/bmx` after `cargo build --release`). Use `cargo run -- …` during development (`default-run` is `bmx`).

## Install with setup.sh

Local checkout:

```bash
./setup.sh
```

Remote bootstrap (replace the URL with this repository when you publish it):

```bash
curl -fsSL https://raw.githubusercontent.com/amkisko/bmx.rs/refs/heads/main/setup.sh | bash -s -- --repo https://github.com/amkisko/bmx.rs.git
```

User-local install without touching `/usr/local`:

```bash
./setup.sh --user
```

## Usage

### Build

Each install lives in `~/.bmx/apps/<id>/` (`repo/` is the clone, `install.toml` is metadata). On install or first run, bmx resolves the spec (bare name, `owner/repo`, URL, `registry:repo`, `name@ref` / semver), syncs git, checks out the ref, picks a stack (Rust → CMake → Make → Homebrew → AUR), builds, then stores the executable. Per-repo `bmx.toml` can set `strategy`, `workdir`, `run`, and `[bmx.hooks]` (`pre_run`, `post_install`). Workspace members: `owner/repo.rs:crate@ref`. Host vs Docker/Podman/nerdctl: `bmx isolation`; optional `integrity_check` in `config.toml` compares live HEAD to metadata on run.

Where is the code, and how to rebuild:

```bash
bmx show my-app                    # files under the cached checkout
bmx show my-app --would-remove    # dry-run what uninstall deletes
bmx rebuild my-app                # sync + checkout(ref/pin/default) + build (no metadata rewrite)
bmx rebuild --install my-app      # rebuild and persist install metadata/output selection
bmx reinstall my-app              # sync + rebuild from install metadata
bmx doctor                        # home, backends, isolation, broken installs
```

Rebuild ref behavior:
- `bmx rebuild APP@version` builds that exact ref.
- `bmx rebuild APP` uses pinned `requested_ref` when present; otherwise default/latest.
- `--pin` is respected: with `--pin APP` it writes/updates project pins, and with `--pin` (no APP) it resolves from pins.

### Manage

```bash
bmx install APP
bmx install APP --as other        # force app id / second checkout
bmx rebuild APP                   # sync + checkout(ref/pin/default) + build, no metadata rewrite
bmx rebuild --install APP         # rebuild and persist install metadata/output selection
bmx uninstall APP
bmx reinstall APP
bmx update APP                    # fetch + checkout + build one
bmx update                        # all installs
bmx self-update SOURCE            # replace the bmx binary
```

```bash
bmx source set-default URL && bmx source show
bmx checkout set-default BACKEND && bmx checkout show   # git | gh | custom
bmx isolation set-default MODE && bmx isolation show    # off | auto | docker | …
```

Optional `[registries]` in `~/.bmx/config.toml` (URL aliases). Undo: `bmx history`, then `bmx undo`, `bmx undo ID`, or `bmx undo --only APP` after a mass update. Shims (Unix): `bmx shim init`, `bmx shim path`, `bmx shim add APP`. No persistent state: add `--rm` to any invocation (temp `BMX_HOME`).

Optional `~/.bmx/trust.toml` can enforce source allow/deny prefixes and signed-commit verification (with optional signer key allowlists). See [SPEC.md](SPEC.md).

Trust helpers:

```bash
bmx trust show
bmx trust add-key KEYID_OR_FINGERPRINT [--match-prefix PREFIX]
bmx trust set-signed --match-prefix PREFIX [--enabled true|false]
bmx trust set-allow --match-prefix PREFIX [--allow true|false]
bmx trust import-repo APP [--match-prefix PREFIX]
```

### Execute

```bash
bmx APP -- ARGS                    # install-if-needed, then run (forward args after --)
bmx exec APP -- ARGS
bmx exec --pin -- ARGS             # app line from .bmx/pin (walks parents)
bmx APP --pin -- ARGS              # same, default command
```

`-v` / `--verbose` prints app id, commit, and binary path; the child prepends that binary’s directory to `PATH`. With `integrity_check = true`, run/exec can verify the checkout matches `install.toml`. Details: `bmx --help` and [SPEC.md](SPEC.md).

`--trust` (global) affects install/update/rebuild/reinstall/self-update: it shows concise signer details for the checked-out `HEAD` commit and asks for explicit consent before importing signer keys into trust policy.

Even without `--trust`, install/update/rebuild/reinstall/self-update require explicit consent for untrusted sources (when HEAD is not verified-good and signer is not already trusted by policy).

For CI/non-interactive pipelines, set `BMX_TRUST_ASSUME_YES=1` to auto-consent these trust prompts. Use this only in controlled automation contexts.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Contributing

Bug reports and pull requests are welcome on GitHub at https://github.com/amkisko/bmx.rs.

For questions, expectations, and how to propose changes, see [CONTRIBUTING.md](CONTRIBUTING.md). Community standards are in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Release history is in [CHANGELOG.md](CHANGELOG.md); maintainer roles and decisions are described in [GOVERNANCE.md](GOVERNANCE.md).

## Security

If you discover a security vulnerability, please report it responsibly. Do not open a public issue. See [SECURITY.md](SECURITY.md) for how to report.

## License

MIT. See [LICENSE.md](LICENSE.md).
