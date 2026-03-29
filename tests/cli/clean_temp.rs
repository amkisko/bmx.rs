use std::fs;
use std::sync::Mutex;

use tempfile::TempDir;

/// `bmx clean` deletes every matching dir under the system temp dir; serialize these tests.
static CLEAN_TEST_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn clean_removes_bmx_managed_temp_directories() {
    let _lock = CLEAN_TEST_LOCK.lock().expect("clean test lock");
    let tmp = std::env::temp_dir();
    let marker = format!(
        "bmx-ephemeral-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let dir = tmp.join(&marker);
    fs::create_dir_all(dir.join("apps")).expect("seed temp dir");

    let home = TempDir::new().expect("home");
    assert_cmd::Command::cargo_bin("bmx")
        .expect("binary")
        .env("HOME", home.path())
        .args(["clean"])
        .assert()
        .success();

    assert!(!dir.exists(), "clean should remove {}", dir.display());
}

#[test]
fn clean_dry_run_does_not_remove() {
    let _lock = CLEAN_TEST_LOCK.lock().expect("clean test lock");
    let tmp = std::env::temp_dir();
    let marker = format!(
        "bmx-trust-import-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let dir = tmp.join(&marker);
    fs::create_dir_all(&dir).expect("seed temp dir");

    let home = TempDir::new().expect("home");
    assert_cmd::Command::cargo_bin("bmx")
        .expect("binary")
        .env("HOME", home.path())
        .args(["clean", "--dry-run"])
        .assert()
        .success();

    assert!(dir.exists(), "dry-run must keep {}", dir.display());
    fs::remove_dir_all(&dir).expect("test cleanup");
}
