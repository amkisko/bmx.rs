# bmx

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
curl -fsSL <setup.sh-url> | bash -s -- --repo https://github.com/amkisko/bmx.rs.git
```

User-local install without touching `/usr/local`:

```bash
./setup.sh --user
```

## Everyday use

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

## Contributing and community

If you want to change the code, read [CONTRIBUTING.md](CONTRIBUTING.md). For security reports and community expectations, see [SECURITY.md](SECURITY.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md). The project is released under [LICENSE.md](LICENSE.md). [CHANGELOG.md](CHANGELOG.md) lists releases; [GOVERNANCE.md](GOVERNANCE.md) describes how roles and decisions work.
