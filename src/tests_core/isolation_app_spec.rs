use serial_test::serial;
use temp_env::with_var;

use crate::app_spec::{layout_id, parse_app_spec};
use crate::isolation::resolve_backend;
use crate::types::BuildIsolation;

#[test]
#[serial]
fn isolation_backend_resolution_errors_without_available_backend() {
    with_var("PATH", Some(""), || {
        assert!(resolve_backend(BuildIsolation::Auto).is_err());
        assert!(resolve_backend(BuildIsolation::Docker).is_err());
    });
}

#[test]
fn parse_app_spec_handles_plain_url_and_ref_inputs() {
    let plain = parse_app_spec("tool");
    assert_eq!(plain.source, "tool");
    assert_eq!(plain.requested_ref, None);
    assert_eq!(plain.cargo_package, None);

    let semver = parse_app_spec("tool@^1.2");
    assert_eq!(semver.source, "tool");
    assert_eq!(semver.requested_ref.as_deref(), Some("^1.2"));
    assert_eq!(semver.cargo_package, None);

    let sha = parse_app_spec("file:///tmp/repo@deadbeef");
    assert_eq!(sha.source, "file:///tmp/repo");
    assert_eq!(sha.requested_ref.as_deref(), Some("deadbeef"));
    assert_eq!(sha.cargo_package, None);

    let ssh = parse_app_spec("git@github.com:acme/tool.git");
    assert_eq!(ssh.source, "git@github.com:acme/tool.git");
    assert_eq!(ssh.requested_ref, None);
    assert_eq!(ssh.cargo_package, None);

    let ssh_with_ref = parse_app_spec("git@github.com:acme/tool.git@v1.0.0");
    assert_eq!(ssh_with_ref.source, "git@github.com:acme/tool.git");
    assert_eq!(ssh_with_ref.requested_ref.as_deref(), Some("v1.0.0"));
    assert_eq!(ssh_with_ref.cargo_package, None);

    let https_with_user = parse_app_spec("https://user@example.com/acme/tool.git");
    assert_eq!(
        https_with_user.source,
        "https://user@example.com/acme/tool.git"
    );
    assert_eq!(https_with_user.requested_ref, None);
    assert_eq!(https_with_user.cargo_package, None);

    let trailing_at = parse_app_spec("tool@");
    assert_eq!(trailing_at.source, "tool@");
    assert_eq!(trailing_at.requested_ref, None);
    assert_eq!(trailing_at.cargo_package, None);

    let https_pkg = parse_app_spec("https://github.com/acme/foo.rs:bar");
    assert_eq!(https_pkg.source, "https://github.com/acme/foo.rs");
    assert_eq!(https_pkg.cargo_package.as_deref(), Some("bar"));
    assert_eq!(layout_id(&https_pkg), "foo-rs__bar");

    let https_pkg_ref = parse_app_spec("https://github.com/acme/foo:bar@main");
    assert_eq!(https_pkg_ref.source, "https://github.com/acme/foo");
    assert_eq!(https_pkg_ref.cargo_package.as_deref(), Some("bar"));
    assert_eq!(https_pkg_ref.requested_ref.as_deref(), Some("main"));

    let ssh_pkg = parse_app_spec("git@github.com:acme/tool.git:scout");
    assert_eq!(ssh_pkg.source, "git@github.com:acme/tool.git");
    assert_eq!(ssh_pkg.cargo_package.as_deref(), Some("scout"));

    let path_pkg_ref = parse_app_spec("acme/foo.rs:bar@v1");
    assert_eq!(path_pkg_ref.source, "acme/foo.rs");
    assert_eq!(path_pkg_ref.cargo_package.as_deref(), Some("bar"));
    assert_eq!(path_pkg_ref.requested_ref.as_deref(), Some("v1"));
}

#[test]
fn parse_app_spec_does_not_treat_registry_colons_as_cargo_package() {
    let reg = parse_app_spec("corp:widgets/extra");
    assert_eq!(reg.source, "corp:widgets/extra");
    assert_eq!(reg.cargo_package, None);
}
