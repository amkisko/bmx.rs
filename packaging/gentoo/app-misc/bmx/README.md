# Gentoo Ebuild for bmx

This is a Gentoo ebuild template for bmx.

## Local Installation

To use this ebuild in a local overlay:

```bash
# Create overlay directory structure
mkdir -p /usr/local/portage/app-misc/bmx

# Copy the ebuild
cp bmx-0.1.4.ebuild /usr/local/portage/app-misc/bmx/

# Generate the manifest
ebuild /usr/local/portage/app-misc/bmx/bmx-0.1.4.ebuild manifest

# Install
emerge app-misc/bmx
```

## Full Offline Build

For a complete offline build with all crate sources:

```bash
# Install cargo-ebuild
cargo install cargo-ebuild

# Generate full ebuild with CARGO_CRATE_URIS
cargo ebuild

# Use the generated ebuild instead of this template
```

## Alternative: Cargo Install

You can also install directly with cargo:

```bash
cargo install --path /path/to/bmx.rs
```
