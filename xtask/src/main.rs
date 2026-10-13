use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const LEDGER_PATH: &str = "docs/parity/parity-ledger.toml";
const EVIDENCE_STATUSES: &[&str] = &[
    "inferred",
    "observed-static",
    "observed-runtime",
    "not-applicable",
];
const IMPLEMENTATION_STATUSES: &[&str] = &["implemented", "verified", "deferred", "not-applicable"];

#[derive(Debug, Deserialize)]
struct Ledger {
    schema_version: Option<u32>,
    matrix_source: Option<String>,
    #[serde(default)]
    artifacts: Vec<Artifact>,
    #[serde(default)]
    capabilities: Vec<Capability>,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    id: Option<String>,
    path: Option<String>,
    sha256: Option<String>,
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Capability {
    id: Option<String>,
    workflow: Option<String>,
    evidence_paths: Option<Vec<String>>,
    evidence_status: Option<String>,
    implementation_status: Option<String>,
    automated_tests: Option<Vec<String>>,
    windows_scenarios: Option<Vec<String>>,
    supported_platforms: Option<Vec<String>>,
    owner: Option<String>,
    acceptance: Option<String>,
    deviation: Option<String>,
    evidence_artifacts: Option<Vec<String>>,
}

fn main() {
    if let Err(errors) = run(env::args().skip(1).collect()) {
        for error in errors {
            eprintln!("error: {error}");
        }
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), Vec<String>> {
    let root = parse_args(&args).map_err(|error| vec![error])?;
    let count = validate(&root)?;
    println!("parity ledger valid: {count} capabilities");
    Ok(())
}

fn parse_args(args: &[String]) -> Result<PathBuf, String> {
    if args.len() < 2 || args[0] != "parity" || args[1] != "validate" {
        return Err("usage: cargo run -p xtask -- parity validate [--root PATH]".into());
    }
    match &args[2..] {
        [] => env::current_dir().map_err(|error| format!("current directory: {error}")),
        [flag, root] if flag == "--root" => Ok(PathBuf::from(root)),
        _ => Err("usage: cargo run -p xtask -- parity validate [--root PATH]".into()),
    }
}

fn validate(root: &Path) -> Result<usize, Vec<String>> {
    let ledger_path = root.join(LEDGER_PATH);
    let source = fs::read_to_string(&ledger_path)
        .map_err(|error| vec![format!("read {}: {error}", ledger_path.display())])?;
    let ledger: Ledger = toml::from_str(&source)
        .map_err(|error| vec![format!("parse {}: {error}", ledger_path.display())])?;
    let mut errors = Vec::new();

    if ledger.schema_version != Some(1) {
        errors.push("schema_version must be 1".into());
    }
    required_text(
        "matrix_source",
        ledger.matrix_source.as_deref(),
        &mut errors,
    );

    let artifacts = validate_artifacts(root, &ledger.artifacts, &mut errors);
    let capability_ids = validate_capabilities(&ledger.capabilities, &artifacts, &mut errors);

    if let Some(matrix_source) = ledger
        .matrix_source
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        validate_matrix_coverage(root, matrix_source, &capability_ids, &mut errors);
    }

    if errors.is_empty() {
        Ok(ledger.capabilities.len())
    } else {
        Err(errors)
    }
}

fn validate_artifacts(
    root: &Path,
    artifacts: &[Artifact],
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let mut reference_hashes = BTreeMap::new();

    for (index, artifact) in artifacts.iter().enumerate() {
        let label = format!("artifacts[{index}]");
        required_text(&format!("{label}.id"), artifact.id.as_deref(), errors);
        required_text(&format!("{label}.path"), artifact.path.as_deref(), errors);
        required_text(
            &format!("{label}.sha256"),
            artifact.sha256.as_deref(),
            errors,
        );
        required_text(&format!("{label}.kind"), artifact.kind.as_deref(), errors);

        if let Some(id) = artifact.id.as_deref().filter(|value| !value.is_empty())
            && !ids.insert(id.to_owned())
        {
            errors.push(format!("duplicate artifact id {id}"));
        }

        let Some(relative_path) = artifact.path.as_deref() else {
            continue;
        };
        let path = Path::new(relative_path);
        if path.is_absolute() || path.components().any(|part| part == Component::ParentDir) {
            errors.push(format!(
                "{label}.path must be repository-relative: {relative_path}"
            ));
            continue;
        }
        if is_release_path(path) {
            errors.push(format!(
                "{label}.path is under a forbidden release/package path: {relative_path}"
            ));
            continue;
        }

        let Some(expected) = artifact.sha256.as_deref() else {
            continue;
        };
        if !is_sha256(expected) {
            errors.push(format!(
                "{label}.sha256 must be 64 lowercase hexadecimal characters"
            ));
            continue;
        }
        match fs::read(root.join(path)) {
            Ok(bytes) => {
                let actual = format!("{:x}", Sha256::digest(bytes));
                if actual != expected {
                    errors.push(format!(
                        "SHA-256 mismatch for {relative_path}: expected {expected}, got {actual}"
                    ));
                }
                if artifact.kind.as_deref() == Some("reference-asset") {
                    reference_hashes.insert(expected.to_owned(), relative_path.to_owned());
                }
            }
            Err(error) => errors.push(format!("read artifact {relative_path}: {error}")),
        }
    }

    reject_reference_hashes_in_release_paths(root, &reference_hashes, errors);
    ids
}

fn validate_capabilities(
    capabilities: &[Capability],
    artifacts: &BTreeSet<String>,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for (index, capability) in capabilities.iter().enumerate() {
        let id = capability.id.as_deref().unwrap_or("");
        let label = if id.is_empty() {
            format!("capabilities[{index}]")
        } else {
            id.to_owned()
        };
        required_text(&format!("{label}.id"), capability.id.as_deref(), errors);
        required_text(
            &format!("{label}.workflow"),
            capability.workflow.as_deref(),
            errors,
        );
        required_list(
            &format!("{label}.evidence_paths"),
            capability.evidence_paths.as_deref(),
            errors,
        );
        required_status(
            &format!("{label}.evidence_status"),
            capability.evidence_status.as_deref(),
            EVIDENCE_STATUSES,
            errors,
        );
        required_status(
            &format!("{label}.implementation_status"),
            capability.implementation_status.as_deref(),
            IMPLEMENTATION_STATUSES,
            errors,
        );
        required_list(
            &format!("{label}.automated_tests"),
            capability.automated_tests.as_deref(),
            errors,
        );
        required_list(
            &format!("{label}.windows_scenarios"),
            capability.windows_scenarios.as_deref(),
            errors,
        );
        required_list(
            &format!("{label}.supported_platforms"),
            capability.supported_platforms.as_deref(),
            errors,
        );
        required_text(
            &format!("{label}.owner"),
            capability.owner.as_deref(),
            errors,
        );
        required_text(
            &format!("{label}.acceptance"),
            capability.acceptance.as_deref(),
            errors,
        );
        required_text(
            &format!("{label}.deviation"),
            capability.deviation.as_deref(),
            errors,
        );
        required_list(
            &format!("{label}.evidence_artifacts"),
            capability.evidence_artifacts.as_deref(),
            errors,
        );

        if !id.is_empty() && !ids.insert(id.to_owned()) {
            errors.push(format!("duplicate capability id {id}"));
        }
        if let Some(references) = capability.evidence_artifacts.as_deref() {
            for reference in references {
                if !artifacts.contains(reference) {
                    errors.push(format!("{label} references unknown artifact {reference}"));
                }
            }
        }
    }
    ids
}

fn validate_matrix_coverage(
    root: &Path,
    matrix_source: &str,
    capability_ids: &BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let matrix_path = root.join(matrix_source);
    if !matrix_path.exists() {
        return;
    }
    let source = match fs::read_to_string(&matrix_path) {
        Ok(source) => source,
        Err(error) => {
            errors.push(format!("read matrix {}: {error}", matrix_path.display()));
            return;
        }
    };
    let matrix_ids: BTreeSet<String> = source.lines().filter_map(matrix_row_id).collect();
    for missing in matrix_ids.difference(capability_ids) {
        errors.push(format!(
            "matrix capability {missing} is missing from ledger"
        ));
    }
    for extra in capability_ids.difference(&matrix_ids) {
        errors.push(format!("ledger capability {extra} is absent from matrix"));
    }
}

fn matrix_row_id(line: &str) -> Option<String> {
    let mut columns = line.split('|');
    columns.next()?;
    let candidate = columns.next()?.trim();
    let (prefix, number) = candidate.rsplit_once('-')?;
    if !prefix.is_empty()
        && prefix
            .chars()
            .all(|character| character.is_ascii_uppercase())
        && number.len() == 2
        && number.chars().all(|character| character.is_ascii_digit())
    {
        Some(candidate.to_owned())
    } else {
        None
    }
}

fn required_text(field: &str, value: Option<&str>, errors: &mut Vec<String>) {
    if value.is_none_or(str::is_empty) {
        errors.push(format!("missing or empty required field {field}"));
    }
}

fn required_list(field: &str, value: Option<&[String]>, errors: &mut Vec<String>) {
    if value.is_none_or(|items| items.is_empty() || items.iter().any(String::is_empty)) {
        errors.push(format!("missing or empty required field {field}"));
    }
}

fn required_status(field: &str, value: Option<&str>, allowed: &[&str], errors: &mut Vec<String>) {
    match value {
        Some(status) if allowed.contains(&status) => {}
        Some(status) => errors.push(format!("invalid {field} value {status}")),
        None => errors.push(format!("missing required field {field}")),
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_release_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name) if matches!(name.to_str(), Some("release" | "releases" | "package" | "packages" | "packaging" | "dist")))
    })
}

fn reject_reference_hashes_in_release_paths(
    root: &Path,
    reference_hashes: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    if reference_hashes.is_empty() {
        return;
    }
    for directory in [
        "release",
        "releases",
        "package",
        "packages",
        "packaging",
        "dist",
    ] {
        let path = root.join(directory);
        scan_release_tree(&path, reference_hashes, errors);
    }
}

fn scan_release_tree(
    path: &Path,
    reference_hashes: &BTreeMap<String, String>,
    errors: &mut Vec<String>,
) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            scan_release_tree(&entry.path(), reference_hashes, errors);
        }
    } else if metadata.is_file()
        && let Ok(bytes) = fs::read(path)
    {
        let digest = format!("{:x}", Sha256::digest(bytes));
        if let Some(source) = reference_hashes.get(&digest) {
            errors.push(format!(
                "original reference asset hash from {source} occurs in release/package path {}",
                path.display()
            ));
        }
    }
}
