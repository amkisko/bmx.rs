# FreeBSD Port for bmx

This is a FreeBSD port template for bmx.

## Local Installation

To use this as a local port:

```bash
# Copy this directory to your local ports tree
mkdir -p /usr/ports/sysutils/bmx
cp -r . /usr/ports/sysutils/bmx/

# Build and install
cd /usr/ports/sysutils/bmx
make install clean
```

## Submitting to Ports Tree

To submit to the official FreeBSD Ports tree:

1. Run `make cargo-crates` to generate the CARGO_CRATES list
2. Add the output to Makefile or create Makefile.crates
3. Follow the FreeBSD Porter's Handbook for submission process
4. Submit via PR or email to the ports mailing list

## Alternative: Cargo Install

If you have Rust installed (`pkg install rust`), you can also install directly:

```bash
cargo install --path /path/to/bmx.rs
```
