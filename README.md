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

Each install lives in `~/.bmx/apps/<id>/` (`repo/` is the clone, `install.toml` is metadata). On install or first run, bmx resolves the spec (bare name, `owner/repo`, URL, `registry:repo`, `name@ref` / semver), syncs git, checks out the ref, picks a stack (**Rust → CMake → Make → Homebrew → AUR**), builds, then stores the executable. Per-repo `bmx.toml` can set `strategy`, `workdir`, `run`, and `[bmx.hooks]` (`pre_run`, `post_install`). Workspace members: `owner/repo.rs:crate@ref`. Host vs **Docker/Podman/nerdctl**: `bmx isolation`; optional `integrity_check` in `config.toml` compares live HEAD to metadata on run.

Where is the code, and how to rebuild after you edit it:

```bash
bmx show my-app                    # files under the cached checkout
bmx show my-app --would-remove    # dry-run what uninstall deletes
# edit ~/.bmx/apps/<id>/repo/...
bmx reinstall my-app              # sync + build again (keeps the directory)
bmx doctor                        # home, backends, isolation, broken installs
```

### Manage

```bash
bmx install APP
bmx install APP --as other        # force app id / second checkout
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

Optional `[registries]` in `~/.bmx/config.toml` (URL aliases). **Undo:** `bmx history`, then `bmx undo`, `bmx undo ID`, or `bmx undo --only APP` after a mass update. **Shims (Unix):** `bmx shim init`, `bmx shim path`, `bmx shim add APP`. **No persistent state:** add `--rm` to any invocation (temp `BMX_HOME`).

### Execute

```bash
bmx APP -- ARGS                    # install-if-needed, then run (forward args after --)
bmx exec APP -- ARGS
bmx exec --pin -- ARGS             # app line from .bmx/pin (walks parents)
bmx APP --pin -- ARGS              # same, default command
```

`-v` / `--verbose` prints app id, commit, and binary path; the child **prepends** that binary’s directory to `PATH`. With `integrity_check = true`, run/exec can verify the checkout matches `install.toml`. Details: `bmx --help` and [SPEC.md](SPEC.md).

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

If you discover a security vulnerability, please report it responsibly. **Do not** open a public issue. See [SECURITY.md](SECURITY.md) for how to report.

## License

MIT. See [LICENSE.md](LICENSE.md).
