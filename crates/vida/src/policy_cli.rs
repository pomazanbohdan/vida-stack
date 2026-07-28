use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use operator_output::toon_report;
use serde_json::{json, Value};
use vida_policy_rhai::{
    build_policy_engine, BundleCacheStatus, Limits, PolicyBundle, PolicyBundleCache,
};

const MAX_POLICY_BUNDLE_BYTES: u64 = 64 * 1024;
const POLICY_CLI_SCHEMA_VERSION: &str = "vida-policy-cli-v1";
const FIXTURE_RUNNER_BLOCKER: &str = "policy_fixture_runner_unavailable";

#[derive(Debug)]
struct PolicyFailure {
    code: &'static str,
    detail: String,
}

impl PolicyFailure {
    fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

#[derive(Debug)]
struct CheckedBundle {
    bundle: PolicyBundle,
    digest: String,
    bundle_bytes: usize,
    source_bytes: usize,
    cache_status: &'static str,
}

pub(crate) fn run_policy(args: crate::PolicyArgs) -> ExitCode {
    match args.command {
        crate::PolicyCommand::Check(args) => run_check(&args.bundle, args.json),
        crate::PolicyCommand::Test(args) => run_test(&args.bundle, args.json),
    }
}

fn run_check(path: &Path, as_json: bool) -> ExitCode {
    let payload = match check_bundle(path) {
        Ok(bundle) => pass_payload("vida policy check", path, &bundle),
        Err(error) => blocked_payload("vida policy check", path, error),
    };
    emit("vida policy check", &payload, as_json);
    if payload["status"] == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn run_test(path: &Path, as_json: bool) -> ExitCode {
    let payload = match check_bundle(path) {
        Err(error) => blocked_payload("vida policy test", path, error),
        Ok(bundle) => blocked_payload_with_details(
            "vida policy test",
            path,
            vec!["canonical_gate_blocked".to_string()],
            vec![
                "Expose the predecessor fixture runner, then rerun `vida policy test`.".to_string(),
            ],
            json!({
                "check": checked_bundle_value(&bundle),
                "fixture_execution": {
                    "status": "blocked",
                    "blocker_code": FIXTURE_RUNNER_BLOCKER,
                    "mandatory": true,
                },
                "issues": [{
                    "code": FIXTURE_RUNNER_BLOCKER,
                    "detail": "The predecessor fixture-runner API is not present in this checkout.",
                }],
            }),
        ),
    };
    emit("vida policy test", &payload, as_json);
    ExitCode::from((payload["status"] != "pass") as u8)
}

fn check_bundle(path: &Path) -> Result<CheckedBundle, PolicyFailure> {
    let raw = read_bounded_bundle(path)?;
    let bundle = PolicyBundle::from_json(&raw)
        .map_err(|error| PolicyFailure::new("policy_bundle_manifest_invalid", error.to_string()))?;
    let limits = Limits::default();
    if bundle.source.len() > limits.max_script_size {
        return Err(PolicyFailure::new(
            "policy_source_too_large",
            format!(
                "normalized source is {} bytes; limit is {} bytes",
                bundle.source.len(),
                limits.max_script_size
            ),
        ));
    }

    let digest = bundle
        .digest()
        .map_err(|error| PolicyFailure::new("policy_digest_failed", error.to_string()))?;
    if digest.len() != 64 {
        return Err(PolicyFailure::new(
            "policy_digest_invalid",
            format!(
                "expected a 64-character BLAKE3 digest, got {}",
                digest.len()
            ),
        ));
    }

    let engine = build_policy_engine(limits);
    let mut cache = PolicyBundleCache::default();
    let (cached, cache_status) = cache
        .import_json(&engine, &raw)
        .map_err(|error| PolicyFailure::new("policy_bundle_compile_failed", error.to_string()))?;
    let cache_status = match cache_status {
        BundleCacheStatus::Compiled => "compiled",
        BundleCacheStatus::Hit => "cache_hit",
    };

    Ok(CheckedBundle {
        bundle,
        digest: cached.digest().to_string(),
        bundle_bytes: raw.len(),
        source_bytes: cached.bundle().source.len(),
        cache_status,
    })
}

fn read_bounded_bundle(path: &Path) -> Result<String, PolicyFailure> {
    let mut file = File::open(path).map_err(|error| {
        PolicyFailure::new(
            "policy_bundle_read_failed",
            format!("{}: {error}", path.display()),
        )
    })?;
    let declared_size = file
        .metadata()
        .map_err(|error| {
            PolicyFailure::new(
                "policy_bundle_metadata_failed",
                format!("{}: {error}", path.display()),
            )
        })?
        .len();
    if declared_size > MAX_POLICY_BUNDLE_BYTES {
        return Err(PolicyFailure::new(
            "policy_bundle_too_large",
            format!("bundle is {declared_size} bytes; limit is {MAX_POLICY_BUNDLE_BYTES} bytes"),
        ));
    }

    let mut bytes = Vec::with_capacity(declared_size as usize);
    file.by_ref()
        .take(MAX_POLICY_BUNDLE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            PolicyFailure::new(
                "policy_bundle_read_failed",
                format!("{}: {error}", path.display()),
            )
        })?;
    if bytes.len() as u64 > MAX_POLICY_BUNDLE_BYTES {
        return Err(PolicyFailure::new(
            "policy_bundle_too_large",
            format!("bundle exceeds {MAX_POLICY_BUNDLE_BYTES} bytes"),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|error| PolicyFailure::new("policy_bundle_not_utf8", error.to_string()))
}

fn pass_payload(surface: &str, path: &Path, bundle: &CheckedBundle) -> Value {
    build_payload(
        surface,
        Vec::new(),
        Vec::new(),
        json!({"bundle": path.display().to_string(), "content_digest": bundle.digest}),
        json!({
            "schema_version": POLICY_CLI_SCHEMA_VERSION,
            "policy": {
                "policy_id": bundle.bundle.policy_id,
                "version": bundle.bundle.version,
                "schema": bundle.bundle.schema,
                "engine_abi": bundle.bundle.engine_abi,
                "content_digest": bundle.digest,
            },
            "checks": [
                {"name": "bundle_size", "status": "pass", "actual_bytes": bundle.bundle_bytes, "max_bytes": MAX_POLICY_BUNDLE_BYTES},
                {"name": "schema", "status": "pass"},
                {"name": "engine_abi", "status": "pass"},
                {"name": "source_size", "status": "pass", "actual_bytes": bundle.source_bytes, "max_bytes": Limits::default().max_script_size},
                {"name": "syntax", "status": "pass", "cache_status": bundle.cache_status},
                {"name": "digest", "status": "pass", "algorithm": "blake3"},
            ],
        }),
    )
}

fn blocked_payload(surface: &str, path: &Path, error: PolicyFailure) -> Value {
    blocked_payload_with_details(
        surface,
        path,
        vec!["canonical_gate_blocked".to_string()],
        vec![format!(
            "Correct `{}` and rerun the command.",
            path.display()
        )],
        json!({
            "schema_version": POLICY_CLI_SCHEMA_VERSION,
            "issues": [{"code": error.code, "detail": error.detail}],
        }),
    )
}

fn blocked_payload_with_details(
    surface: &str,
    path: &Path,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    extra_fields: Value,
) -> Value {
    build_payload(
        surface,
        blocker_codes,
        next_actions,
        json!({"bundle": path.display().to_string()}),
        extra_fields,
    )
}

fn build_payload(
    surface: &str,
    blocker_codes: Vec<String>,
    next_actions: Vec<String>,
    artifact_refs: Value,
    extra_fields: Value,
) -> Value {
    crate::release1_operator_output::Release1OperatorOutputBuilder::new(surface)
        .blocker_codes(blocker_codes)
        .next_actions(next_actions)
        .artifact_refs(artifact_refs)
        .extra_fields(extra_fields)
        .build()
        .unwrap_or_else(|error| {
            json!({
                "surface": surface,
                "status": "blocked",
                "blocker_codes": ["canonical_gate_blocked"],
                "next_actions": ["Inspect the policy CLI envelope renderer."],
                "renderer_error": error,
            })
        })
}

fn checked_bundle_value(bundle: &CheckedBundle) -> Value {
    json!({
        "policy_id": bundle.bundle.policy_id,
        "version": bundle.bundle.version,
        "content_digest": bundle.digest,
        "source_bytes": bundle.source_bytes,
        "cache_status": bundle.cache_status,
    })
}

fn emit(surface: &str, payload: &Value, as_json: bool) {
    if as_json {
        crate::print_json_pretty(payload);
    } else {
        println!("{}", toon_report::render_value(surface, payload.clone()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn bundle_path(source: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "vida-policy-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).expect("fixture root");
        let path = root.join("bundle.json");
        std::fs::write(
            &path,
            serde_json::json!({
                "schema": 1,
                "policy_id": "rhai.runtime.authority",
                "version": 1,
                "engine_abi": "rhai-policy-engine-v1",
                "source": source,
            })
            .to_string(),
        )
        .expect("bundle");
        (root, path)
    }

    #[test]
    fn policy_help_exposes_check_and_test_bundle_contract() {
        let error = crate::Cli::try_parse_from(["vida", "policy", "check", "--help"])
            .expect_err("help should be clap display error");
        let help = error.to_string();
        assert!(help.contains("--bundle <PATH>"));
        assert!(help.contains("without writing runtime state"));
    }

    #[test]
    fn check_payload_is_release1_pass_with_digest_checks() {
        let (root, path) = bundle_path("1");
        let payload = pass_payload("vida policy check", &path, &check_bundle(&path).unwrap());
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["checks"][5]["name"], "digest");
        assert_eq!(payload["policy"]["engine_abi"], "rhai-policy-engine-v1");
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn check_rejects_oversized_source() {
        let (root, path) = bundle_path(&"x".repeat(Limits::default().max_script_size + 1));
        let error = check_bundle(&path).expect_err("source must be bounded");
        assert_eq!(error.code, "policy_source_too_large");
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn test_reports_missing_predecessor_fixture_runner_without_state_access() {
        let (root, path) = bundle_path("1");
        let checked = check_bundle(&path).expect("bundle check");
        let payload = blocked_payload_with_details(
            "vida policy test",
            &path,
            vec!["canonical_gate_blocked".to_string()],
            vec!["Expose the predecessor fixture runner, then rerun `vida policy test`.".into()],
            json!({"check": checked_bundle_value(&checked), "fixture_execution": {"status": "blocked", "blocker_code": FIXTURE_RUNNER_BLOCKER}}),
        );
        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["fixture_execution"]["blocker_code"],
            FIXTURE_RUNNER_BLOCKER
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
