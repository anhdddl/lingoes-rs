use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask must be in the workspace root")
        .to_path_buf()
}

fn validator() -> Command {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
}

fn valid_ledger(artifact_path: &str, digest: &str) -> String {
    format!(
        r#"schema_version = 1
matrix_source = "docs/audit/parity-gap-matrix.md"

[[artifacts]]
id = "ART-001"
path = "{artifact_path}"
sha256 = "{digest}"
kind = "independent-fixture"

[[capabilities]]
id = "UI-01"
workflow = "Open and control the main window"
evidence_paths = ["docs/audit/parity-gap-matrix.md#UI-01"]
evidence_status = "observed-static"
implementation_status = "deferred"
automated_tests = ["LEDGER-VALIDATION"]
windows_scenarios = ["WIN-UI-01"]
supported_platforms = ["windows"]
owner = "desktop-shell"
acceptance = "Window controls pass the recorded Windows scenario."
deviation = "Runtime behavior has not been observed."
evidence_artifacts = ["ART-001"]
"#
    )
}

fn write_case(ledger: &str, artifact: &[u8]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temp repo");
    fs::create_dir_all(root.path().join("docs/parity")).expect("parity dir");
    fs::create_dir_all(root.path().join("fixtures")).expect("fixture dir");
    fs::write(root.path().join("docs/parity/parity-ledger.toml"), ledger).expect("ledger");
    fs::write(root.path().join("fixtures/evidence.txt"), artifact).expect("artifact");
    root
}

fn run_case(root: &Path) -> std::process::Output {
    validator()
        .args(["parity", "validate", "--root"])
        .arg(root)
        .output()
        .expect("run validator")
}

#[test]
fn published_schema_requires_milestone_zero_fields() {
    let schema_path = repo_root().join("docs/parity/parity-ledger.schema.json");
    let schema: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(schema_path).expect("read schema"))
            .expect("parse schema");
    let required = schema["$defs"]["capability"]["required"]
        .as_array()
        .expect("capability required list");
    for field in [
        "evidence_status",
        "implementation_status",
        "acceptance",
        "owner",
        "deviation",
    ] {
        assert!(
            required.iter().any(|value| value == field),
            "missing {field}"
        );
    }
}

#[test]
fn repository_ledger_covers_every_gap_matrix_id() {
    let output = validator()
        .args(["parity", "validate", "--root"])
        .arg(repo_root())
        .output()
        .expect("run repository validator");
    assert!(
        output.status.success(),
        "validator failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("108 capabilities"));
}

#[test]
fn rejects_missing_required_evidence_paths() {
    let ledger = valid_ledger(
        "fixtures/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .replace(
        "evidence_paths = [\"docs/audit/parity-gap-matrix.md#UI-01\"]\n",
        "",
    );
    let root = write_case(&ledger, b"hello");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("evidence_paths"));
}

#[test]
fn rejects_missing_required_evidence_status() {
    let ledger = valid_ledger(
        "fixtures/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .replace("evidence_status = \"observed-static\"\n", "");
    let root = write_case(&ledger, b"hello");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("evidence_status"));
}

#[test]
fn rejects_missing_required_implementation_status() {
    let ledger = valid_ledger(
        "fixtures/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .replace("implementation_status = \"deferred\"\n", "");
    let root = write_case(&ledger, b"hello");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("implementation_status"));
}

#[test]
fn rejects_missing_required_owner() {
    let ledger = valid_ledger(
        "fixtures/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .replace("owner = \"desktop-shell\"\n", "");
    let root = write_case(&ledger, b"hello");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("owner"));
}

#[test]
fn rejects_missing_required_acceptance() {
    let ledger = valid_ledger(
        "fixtures/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .replace(
        "acceptance = \"Window controls pass the recorded Windows scenario.\"\n",
        "",
    );
    let root = write_case(&ledger, b"hello");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("acceptance"));
}

#[test]
fn rejects_missing_required_deviation() {
    let ledger = valid_ledger(
        "fixtures/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .replace(
        "deviation = \"Runtime behavior has not been observed.\"\n",
        "",
    );
    let root = write_case(&ledger, b"hello");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("deviation"));
}

#[test]
fn rejects_hash_mismatch() {
    let root = write_case(
        &valid_ledger(
            "fixtures/evidence.txt",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ),
        b"hello",
    );
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SHA-256 mismatch"));
}

#[test]
fn rejects_artifact_manifest_paths_under_release_directories() {
    let ledger = valid_ledger(
        "release/original/extracted/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    );
    let root = tempfile::tempdir().expect("temp repo");
    fs::create_dir_all(root.path().join("docs/parity")).expect("parity dir");
    fs::create_dir_all(root.path().join("release/original/extracted")).expect("release dir");
    fs::write(root.path().join("docs/parity/parity-ledger.toml"), ledger).expect("ledger");
    fs::write(
        root.path().join("release/original/extracted/evidence.txt"),
        b"hello",
    )
    .expect("artifact");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("release/package path"));
}

#[test]
fn rejects_original_reference_asset_hash_copied_into_release() {
    let ledger = valid_ledger(
        "fixtures/evidence.txt",
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
    )
    .replace(
        "kind = \"independent-fixture\"",
        "kind = \"reference-asset\"",
    );
    let root = write_case(&ledger, b"hello");
    fs::create_dir_all(root.path().join("dist/assets")).expect("dist dir");
    fs::write(root.path().join("dist/assets/copied.bin"), b"hello").expect("copied asset");
    let output = run_case(root.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("original reference asset hash"));
}

#[test]
fn accepts_a_valid_hashed_artifact_outside_release_paths() {
    let root = write_case(
        &valid_ledger(
            "fixtures/evidence.txt",
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        ),
        b"hello",
    );
    let output = run_case(root.path());
    assert!(
        output.status.success(),
        "validator failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
