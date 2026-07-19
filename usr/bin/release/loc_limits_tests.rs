use super::*;
use std::fs;
use std::path::Path;

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}

fn fixture_root(label: &str) -> PathBuf {
    static NEXT_FIXTURE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let sequence = NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "scout-loc-limits-{}-{}-{sequence}",
        label,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create fixture root");

    write_file(
        &root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"demo\"]\n",
    );
    write_file(
        &root.join("demo/Cargo.toml"),
        "[package]\nname = \"demo\"\n",
    );
    root
}

#[test]
fn warns_when_file_exceeds_soft_limit() {
    let root = fixture_root("soft");
    let lines = (0..151)
        .map(|index| format!("let _ = {index};"))
        .collect::<Vec<_>>();
    write_file(&root.join("demo/src/soft.rs"), &lines.join("\n"));

    let report = check_loc_limits(&root).expect("check loc limits");
    assert!(report.errors.is_empty());
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.warnings[0].kind, LocFindingKind::SoftLimit);
}

#[test]
fn errors_when_new_file_exceeds_hard_limit() {
    let root = fixture_root("hard");
    let lines = (0..301)
        .map(|index| format!("let _ = {index};"))
        .collect::<Vec<_>>();
    write_file(&root.join("demo/src/hard.rs"), &lines.join("\n"));

    let report = check_loc_limits(&root).expect("check loc limits");
    assert_eq!(report.errors.len(), 1);
    assert_eq!(report.errors[0].kind, LocFindingKind::HardLimit);
}

#[test]
fn errors_when_baselined_file_grows() {
    let root = fixture_root("baseline");
    let lines = (0..305)
        .map(|index| format!("let _ = {index};"))
        .collect::<Vec<_>>();
    write_file(&root.join("demo/src/legacy.rs"), &lines.join("\n"));
    write_file(&root.join(BASELINE_FILE), "demo/src/legacy.rs\t304\n");

    let report = check_loc_limits(&root).expect("check loc limits");
    assert_eq!(report.errors.len(), 1);
    assert_eq!(
        report.errors[0].kind,
        LocFindingKind::BaselineExceeded { allowed: 304 }
    );
}

#[test]
fn write_baseline_records_only_hard_limit_files() {
    let root = fixture_root("write");
    let soft = (0..160)
        .map(|index| format!("let _ = {index};"))
        .collect::<Vec<_>>();
    let hard = (0..301)
        .map(|index| format!("let _ = {index};"))
        .collect::<Vec<_>>();
    write_file(&root.join("demo/src/soft.rs"), &soft.join("\n"));
    write_file(&root.join("demo/src/hard.rs"), &hard.join("\n"));

    let recorded = write_baseline(&root).expect("write baseline");
    assert_eq!(recorded, vec!["demo/src/hard.rs".to_string()]);
}
