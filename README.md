# bmx

[![Test Status](https://github.com/amkisko/bmx.rs/actions/workflows/test.yml/badge.svg)](https://github.com/amkisko/bmx.rs/actions/workflows/test.yml)

Command-line tool that installs, builds, and runs software from source repositories. The usual shape is: you name an app or repo, bmx makes sure it is built and cached locally, then runs it. State lives under `~/.bmx` so everything stays on your machine.

For behaviour, commands, storage layout, and limits, see [SPEC.md](SPEC.md).

## Requirements

- Rust stable toolchain (2024 edition) for building from source, or install via `setup.sh` (see below).

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

Examples:

```bash
bmx my-app -- --help
bmx install my-app
bmx update my-app
bmx doctor
bmx source show
bmx isolation show
```

Subcommands match what you would expect: install, uninstall, reinstall, update, exec, plus defaults for where code is fetched from and how builds are isolated. Details and flags are in the spec and `bmx --help`.

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
