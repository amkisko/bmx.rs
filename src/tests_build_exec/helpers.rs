use std::fs;
use std::path::Path;

#[cfg(unix)]
pub(crate) fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod");
}

pub(crate) fn write_stub_command(dir: &Path, name: &str) {
    let cmd = dir.join(name);
    fs::write(&cmd, "#!/bin/sh\nexit 0\n").expect("write stub");
    #[cfg(unix)]
    make_executable(&cmd);
}

pub(crate) fn write_cwd_stub_command(dir: &Path, name: &str) {
    let cmd = dir.join(name);
    fs::write(
        &cmd,
        "#!/bin/sh\nif [ -n \"$BMX_TEST_CWD_OUT\" ]; then\n  pwd > \"$BMX_TEST_CWD_OUT\"\nfi\nexit 0\n",
    )
    .expect("write stub");
    #[cfg(unix)]
    make_executable(&cmd);
}
