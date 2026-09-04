# RFC 0006: Search and source resolution

- Feature Name: search-and-source-resolution
- Type: Standards Track
- Status: Stable
- Created: 2026-08-18
- Author: Andrei Makarov
- Relates: RFC 0002, RFC 0003

## Summary

`bmx search` discovers installable sources without cloning. Backends are GitHub, GitLab, AUR, and Homebrew. Optional `[registries]` entries in `config.toml` are git URL aliases only. This design has no multi-version installs, shared compile cache, or pluggable registries beyond static URL prefixes.

## Motivation

Search that cloned every hit would fill `~/.bmx` before the operator chose an app. Treating `[registries]` as npm or cargo indices would make bmx a language package manager, which RFC 0002 rejects.

## Guide-level explanation

`bmx search [WORDS …] [--backend auto|github|gitlab|aur|homebrew]`. `--backend auto` uses the GitHub or GitLab host from `default_source`. Hits are probed unless `--no-probe`: the repository root must contain a file bmx already recognizes (`Cargo.toml`, `CMakeLists.txt`, `PKGBUILD`, `Brewfile`, `Makefile`/`makefile`, `bmx.toml`, or a `.rb` formula stub).

An app argument may be a repo URL, `host.tld/owner/repo`, a bare name combined with `default_source`, or `name@ref`. Optional `:cargo_package` selects a Cargo workspace member. `registry_key:repo` with no `/` stays a `[registries]` alias.

## Reference-level explanation

AUR uses clone URL `https://aur.archlinux.org/<PackageBase>.git`. Homebrew reads `formulae.brew.sh/api/formula.json` and keeps formulas whose homepage or tarball maps to a GitHub or GitLab clone URL. Adding multi-version side-by-side installs, a shared compile cache, or pluggable registry protocols is a new RFC.

## Registrar

Search backends: `auto`, `github`, `gitlab`, `aur`, `homebrew`. Config table: `[registries]`.

## Drawbacks

Probe traffic hits forges. Homebrew and AUR mappings are heuristics. No side-by-side versions means upgrades replace the install.

## Rationale and alternatives

Cloning to search would be accurate and would fill the cache. A cargo-like registry protocol would duplicate crates.io. Doing nothing leaves operators pasting git URLs only.

## Prior art

`brew search`, `pacman -Ss`, GitHub repository search. cargo and npm registries as counter-examples this RFC refuses to copy. RFC 0003 resolves the chosen source into `install.toml`.

## Unresolved questions

Whether recognized root files should freeze before a new build strategy is added.
