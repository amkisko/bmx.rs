# Packaging

Distribution artifacts and package descriptors for bmx.

| Distribution | Path | Notes |
|-------------|------|--------|
| **Homebrew** | [homebrew/bmx.rb](homebrew/bmx.rb) | `make sync-packaging` updates the tag URL; fill `sha256` after the release tarball exists. Installs completions and man page. |
| **Nix** | [nix/](nix/) | `nix build` from packaging/nix or adapt flake; keep version in sync via `make sync-packaging` |
| **Flatpak** | [flatpak/io.github.amkisko.bmx.yml](flatpak/io.github.amkisko.bmx.yml) | May require Rust SDK; adjust base/SDK as needed |
| **Arch AUR** | [aur/PKGBUILD](aur/PKGBUILD) | Run `updpkgsums` after setting `pkgver`; submit to AUR |
| **FreeBSD** | [freebsd/](freebsd/) | Port template; run `make cargo-crates` then submit to Ports tree or use as local port. Or `cargo install --path .` with `pkg install rust`. |
| **Gentoo** | [gentoo/app-misc/bmx/](gentoo/app-misc/bmx/) | Ebuild template; for full offline build run `cargo ebuild` and use generated ebuild. Or `cargo install --path .`. |

**BSD (FreeBSD, OpenBSD, NetBSD):** No official packages yet. On FreeBSD use the port template in [freebsd/](freebsd/) or install Rust (`pkg install rust`) and run `cargo install --path .` from the repo.

**Gentoo:** Use the ebuild in a local overlay or generate a full ebuild with `cargo ebuild` (see [gentoo/app-misc/bmx/README.md](gentoo/app-misc/bmx/README.md)).

All packaging is best-effort; prefer `cargo install --path .` or the official install method documented in the main README when in doubt.
