# bmx

[![Test Status](https://github.com/amkisko/bmx.rs/actions/workflows/test.yml/badge.svg)](https://github.com/amkisko/bmx.rs/actions/workflows/test.yml)

Command-line tool that installs, builds, and runs software from source repositories. The usual shape is: you name an app or repo, bmx makes sure it is built and cached locally, then runs it. State lives under `~/.bmx` so everything stays on your machine.

For behaviour, commands, storage layout, and limits, see [SPEC.md](SPEC.md).

## Build and install

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

Remote bootstrap (default user-local install):

```bash
curl -fsSL https://raw.githubusercontent.com/amkisko/bmx.rs/refs/heads/main/setup.sh | bash -s -- --repo https://github.com/amkisko/bmx.rs.git
```

Global install (explicit), requires superuser permissions:

```bash
curl -fsSL https://raw.githubusercontent.com/amkisko/bmx.rs/refs/heads/main/setup.sh | bash -s -- --repo https://github.com/amkisko/bmx.rs.git --system
```

## Usage

### Build

Each install lives in `~/.bmx/apps/<id>/` (`repo/` is the clone, `install.toml` is metadata). On install or first run, bmx resolves the spec (bare name, `owner/repo`, URL, `registry:repo`, `name@ref` / semver), syncs git, checks out the ref, picks a stack (Rust → CMake → Make → Homebrew → AUR), builds, then stores the executable. Per-repo `bmx.toml` can set `strategy`, `workdir`, `run` (repo-relative only), and `[bmx.hooks]` (`pre_run`, `post_install`; requires `hooks_enabled = true` in config). Workspace members: `owner/repo.rs:crate@ref`. Host vs Docker/Podman/nerdctl: `bmx isolation`. By default `integrity_check = true` compares live HEAD to metadata on run. Shell completions: `bmx completions bash|zsh|fish|…`; man page: `bmx man`.

Public forge host shortcuts are accepted directly (no scheme required), including GitHub, GitLab, Bitbucket, SourceHut (`git.sr.ht`), Codeberg, and other host-style URLs like `gitea.example.com/org/repo`.

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
bmx uninstall APP                 # confirms on a TTY; use --force in scripts
bmx history --json
bmx doctor --json
bmx trust list --json
bmx reinstall APP
bmx update APP                    # fetch + checkout + build one
bmx update                        # all installs
bmx self-update                   # rebuild the bmx binary (from amkisko/bmx.rs by default)
```

```bash
bmx source set-default URL && bmx source show
bmx checkout set-default BACKEND && bmx checkout show   # git | gh | custom
bmx isolation set-default MODE && bmx isolation show        # build: off | auto | docker | …
bmx isolation set-default-run MODE && bmx isolation show-run # run: off | auto | docker | …
```

Optional `[registries]` in `~/.bmx/config.toml` (URL aliases). `bmx self-update` source is configurable via `self_update_source = "owner/repo"` (or env override `BMX_SELF_UPDATE_SOURCE`). Undo: `bmx history`, then `bmx undo`, `bmx undo ID`, or `bmx undo --only APP` after a mass update. Shims (Unix): `bmx shim init`, `bmx shim path`, `bmx shim add APP`. No persistent state: add `--rm` to any invocation (temp `BMX_HOME`).

Optional `~/.bmx/trust.toml` can enforce source allow/deny prefixes and signed-commit verification (with optional signer key allowlists). See [SPEC.md](SPEC.md).

Trust helpers:

```bash
bmx trust list
bmx trust list --global          # only global/default trust scope
bmx trust list --local           # only local/source-scoped rules
bmx trust list my-app            # trust view filtered to installed app source
bmx trust add-key KEYID_OR_FINGERPRINT [--match-prefix PREFIX]
bmx trust remove-key KEYID_OR_FINGERPRINT [--match-prefix PREFIX]   # alias: bmx trust revoke …
bmx trust set-signed --match-prefix PREFIX [--enabled true|false]
bmx trust set-allow --match-prefix PREFIX [--allow true|false]
bmx trust import-repo APP [--match-prefix PREFIX]
bmx trust check [SOURCE]
```

`bmx trust check` audits all trusted signer keys from `~/.bmx/trust.toml` against a compromised-key feed.

- `SOURCE` may be:
- A direct `.toml` or `.txt` URL.
- A git URL (the repository should include one of: `compromised-keys.toml`, `compromised_keys.toml`, `trust/compromised-keys.toml`, `trust/compromised_keys.toml`, or `.txt` variants).
- If `SOURCE` is omitted, bmx uses a default public feed URL.

Recommended maintainer format (`.toml`):

```toml
[[keys]]
value = "SHA256:EXAMPLEKEY..."
reason = "private key exposure"
reference = "https://example.com/advisory/123"
reported_at = "2026-03-29"
```

Plain-text feeds are also supported (`.txt`): one key per line, optional `# reason` comment.

### Execute

```bash
bmx APP -- ARGS                    # install-if-needed, then run (forward args after --)
bmx exec APP -- ARGS
bmx exec --pin -- ARGS             # app line from .bmx/pin (walks parents)
bmx APP --pin -- ARGS              # same, default command
bmx --isolate-run APP -- ARGS      # run this invocation in container isolation (auto backend)
bmx --no-isolate-run APP -- ARGS   # force host run this invocation
```

`-v` / `--verbose` prints app id, commit, and binary path; the child prepends that binary’s directory to `PATH`. Run/exec verify the checkout against `install.toml` unless `integrity_check = false`. Set `BMX_TRUST_REQUIRED=1` to refuse install/run when `trust.toml` is missing. Details: `bmx --help` and [SPEC.md](SPEC.md).

`--trust` (global flag) affects install/update/rebuild/reinstall/self-update: it shows concise signer details for the checked-out `HEAD` commit and asks for explicit consent before importing signer keys into trust policy. Imported keys are source-scoped by default; pass `--global --trust` to add them to default/global trust scope.

Even without `--trust`, install/update/rebuild/reinstall/self-update require explicit consent for untrusted sources (when HEAD is not verified-good and signer is not already trusted by policy).

SSH-signature checks are scoped to `bmx` git subprocesses by using per-source allowed signers files under `$BMX_HOME/trust/allowed_signers/`, so `bmx` does not modify your global git config.

For CI/non-interactive pipelines, pass `--no-input` so prompts never block, and set `BMX_TRUST_ASSUME_YES=1` when trust auto-consent is required. Use assume-yes only in controlled automation contexts. Destructive uninstalls need `bmx uninstall --force` when stdin is not a TTY (or with `--no-input`).

`--rm` uses a temporary `BMX_HOME` for caches and installs, but still loads **`$HOME/.bmx/trust.toml`** and **`$HOME/.bmx/trust/`** (copy at process start) so your signer allowlists and deny rules apply; general `config.toml` stays ephemeral defaults.

To delete orphaned temp trees (e.g. after a crash), run **`bmx clean`** (`--dry-run` lists what would be removed). This only removes directories named `bmx-ephemeral-*`, `bmx-trust-import-*`, or `bmx-trust-check-*` directly under the system temp directory.

## Development

```bash
bin/test
```

`bin/test` runs formatting, linting, file-size limits, tests, and coverage gate.

File size policy used by `bin/test`:

- Soft limit: 150 lines (reported as `[SOFT]`).
- Hard limit: 300 lines (fails with non-zero exit).

## Contributing

Bug reports and pull requests are welcome on GitHub at [amkisko/bmx.rs](https://github.com/amkisko/bmx.rs).

For questions, expectations, and how to propose changes, see [CONTRIBUTING.md](CONTRIBUTING.md). Community standards are in [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). Release history is in [CHANGELOG.md](CHANGELOG.md); maintainer roles and decisions are described in [GOVERNANCE.md](GOVERNANCE.md).

## Security

If you discover a security vulnerability, please report it responsibly. Do not open a public issue. See [SECURITY.md](SECURITY.md) for how to report.

## Links

- [GitHub](https://github.com/amkisko/bmx.rs)
- [GitLab](https://gitlab.com/amkisko/bmx.rs)
- [crates.io](https://crates.io/crates/bmx_rs)
- [libraries.io](https://libraries.io/cargo/bmx_rs)
- [Deps.dev](https://deps.dev/cargo/bmx_rs)
- [SonarCloud](https://sonarcloud.io/project/overview?id=amkisko_bmx.rs)
- [Snyk](https://snyk.io/test/github/amkisko/bmx.rs)
- [Codecov](https://app.codecov.io/github/amkisko/bmx.rs)
- [OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/amkisko/bmx.rs)

## License

MIT. See [LICENSE.md](LICENSE.md).
