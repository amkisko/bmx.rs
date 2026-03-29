pub(crate) const DEFAULT_UA: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Root files bmx build detection looks for (see `build::BUILD_PLUGINS`).
pub(crate) const ROOT_BUILD_MARKERS: &[&str] = &[
    "Cargo.toml",
    "CMakeLists.txt",
    "PKGBUILD",
    "Brewfile",
    "Makefile",
    "makefile",
    "bmx.toml",
];
