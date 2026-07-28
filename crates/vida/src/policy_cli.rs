use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use operator_output::toon_report;
use serde_json::{json, Value};
use vida_policy_rhai::fixture::MAX_FIXTURE_CORPUS_BYTES;
use vida_policy_rhai::{
    build_policy_engine, run_fixture_jsonl, BundleCacheStatus, FixtureReport, FixtureRunError,
    Limits, PolicyBundle, PolicyBundleCache,
};

const MAX_POLICY_BUNDLE_BYTES: u64 = 64 * 1024;
const POLICY_CLI_SCHEMA_VERSION: &str = "vida-policy-cli-v1";

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
        crate::PolicyCommand::Test(args) => run_test(&args.bundle, &args.fixtures, args.json),
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

fn run_test(bundle_path: &Path, fixtures_path: &Path, as_json: bool) -> ExitCode {
    let payload = test_payload(bundle_path, fixtures_path);
    emit("vida policy test", &payload, as_json);
    if payload["status"] == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn test_payload(bundle_path: &Path, fixtures_path: &Path) -> Value {
    let checked = match check_bundle(bundle_path) {
        Ok(bundle) => bundle,
        Err(error) => return blocked_payload("vida policy test", bundle_path, error),
    };
    let fixtures = match read_bounded_fixture(fixtures_path) {
        Ok(fixtures) => fixtures,
        Err(error) => {
            return fixture_issue_payload(
                bundle_path,
                fixtures_path,
                &checked,
                policy_failure_value(&error),
            )
        }
    };
    let engine = build_policy_engine(Limits::default());
    match run_fixture_jsonl(&engine, &checked.bundle, &fixtures) {
        Ok(report) => fixture_report_payload(bundle_path, fixtures_path, &checked, report),
        Err(error) => fixture_issue_payload(
            bundle_path,
            fixtures_path,
            &checked,
            fixture_run_error_value(&error),
        ),
    }
}

fn fixture_report_payload(
    bundle_path: &Path,
    fixtures_path: &Path,
    checked: &CheckedBundle,
    report: FixtureReport,
) -> Value {
    let passed = report.is_pass();
    let blocker_codes = if passed {
        Vec::new()
    } else {
        vec!["canonical_gate_blocked".to_string()]
    };
    let next_actions = if passed {
        Vec::new()
    } else {
        vec!["Correct failing policy fixtures and rerun `vida policy test`.".to_string()]
    };
    build_payload(
        "vida policy test",
        blocker_codes,
        next_actions,
        json!({
            "bundle": bundle_path.display().to_string(),
            "fixtures": fixtures_path.display().to_string(),
        }),
        json!({
            "schema_version": POLICY_CLI_SCHEMA_VERSION,
            "check": checked_bundle_value(checked),
            "fixture_execution": {
                "status": if passed { "pass" } else { "fail" },
                "report": report,
            },
        }),
    )
}

fn fixture_issue_payload(
    bundle_path: &Path,
    fixtures_path: &Path,
    checked: &CheckedBundle,
    issue: Value,
) -> Value {
    build_payload(
        "vida policy test",
        vec!["canonical_gate_blocked".to_string()],
        vec!["Correct the fixture corpus and rerun `vida policy test`.".to_string()],
        json!({
            "bundle": bundle_path.display().to_string(),
            "fixtures": fixtures_path.display().to_string(),
        }),
        json!({
            "schema_version": POLICY_CLI_SCHEMA_VERSION,
            "check": checked_bundle_value(checked),
            "fixture_execution": {
                "status": "fail",
                "issues": [issue.clone()],
            },
            "issues": [issue],
        }),
    )
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
    read_bounded_text(path, MAX_POLICY_BUNDLE_BYTES, "policy_bundle_too_large")
}

fn read_bounded_fixture(path: &Path) -> Result<String, PolicyFailure> {
    read_bounded_text(
        path,
        MAX_FIXTURE_CORPUS_BYTES as u64,
        "fixture_corpus_too_large",
    )
    .map_err(|error| match error.code {
        "policy_bundle_read_failed" => {
            PolicyFailure::new("fixture_corpus_read_failed", error.detail)
        }
        "policy_bundle_metadata_failed" => {
            PolicyFailure::new("fixture_corpus_metadata_failed", error.detail)
        }
        "policy_bundle_not_utf8" => PolicyFailure::new("fixture_corpus_not_utf8", error.detail),
        _ => error,
    })
}

fn read_bounded_text(
    path: &Path,
    max_bytes: u64,
    too_large_code: &'static str,
) -> Result<String, PolicyFailure> {
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
    if declared_size > max_bytes {
        return Err(PolicyFailure::new(
            too_large_code,
            format!("file is {declared_size} bytes; limit is {max_bytes} bytes"),
        ));
    }

    let mut bytes = Vec::with_capacity(declared_size as usize);
    file.by_ref()
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            PolicyFailure::new(
                "policy_bundle_read_failed",
                format!("{}: {error}", path.display()),
            )
        })?;
    if bytes.len() as u64 > max_bytes {
        return Err(PolicyFailure::new(
            too_large_code,
            format!("file exceeds {max_bytes} bytes"),
        ));
    }
    String::from_utf8(bytes)
        .map_err(|error| PolicyFailure::new("policy_bundle_not_utf8", error.to_string()))
}

fn policy_failure_value(error: &PolicyFailure) -> Value {
    json!({
        "kind": "corpus",
        "code": error.code,
        "detail": error.detail,
    })
}

fn fixture_run_error_value(error: &FixtureRunError) -> Value {
    match error {
        FixtureRunError::Corpus(error) => json!({
            "kind": "corpus",
            "code": error.code.as_str(),
            "line": error.line,
            "fixture_id": error.fixture_id,
            "detail": error.detail,
        }),
        FixtureRunError::Policy { source } => json!({
            "kind": "policy",
            "code": source.code().as_str(),
            "detail": source.to_string(),
        }),
    }
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
    fn policy_test_help_requires_bounded_fixture_input() {
        let error = crate::Cli::try_parse_from(["vida", "policy", "test", "--help"])
            .expect_err("help should be clap display error");
        let help = error.to_string();
        assert!(help.contains("--fixtures <PATH>"));
        assert!(help.contains("bounded JSONL fixture corpus"));
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
    fn policy_test_runs_fixture_runner_and_returns_pass() {
        let (root, path) = bundle_path("ctx.value");
        let fixtures = root.join("fixtures.jsonl");
        std::fs::write(
            &fixtures,
            r#"{"fixture_id":"positive","context":{"value":42},"expected":42}"#,
        )
        .expect("fixtures");
        let payload = test_payload(&path, &fixtures);
        assert_eq!(payload["status"], "pass");
        assert_eq!(payload["fixture_execution"]["status"], "pass");
        assert_eq!(payload["fixture_execution"]["report"]["passed"], 1);
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn policy_test_reports_typed_fixture_failure() {
        let (root, path) = bundle_path("ctx.value");
        let fixtures = root.join("fixtures.jsonl");
        std::fs::write(
            &fixtures,
            r#"{"fixture_id":"mismatch","context":{"value":41},"expected":42}"#,
        )
        .expect("fixtures");
        let payload = test_payload(&path, &fixtures);
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["fixture_execution"]["status"], "fail");
        assert_eq!(payload["fixture_execution"]["report"]["failed"], 1);
        assert_eq!(
            payload["fixture_execution"]["report"]["results"][0]["failure"]["code"],
            "output_mismatch"
        );
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn policy_test_reports_typed_fixture_corpus_error() {
        let (root, path) = bundle_path("1");
        let fixtures = root.join("fixtures.jsonl");
        std::fs::write(&fixtures, r#"{"fixture_id":"broken""#).expect("fixtures");
        let payload = test_payload(&path, &fixtures);
        assert_eq!(payload["status"], "blocked");
        assert_eq!(payload["fixture_execution"]["status"], "fail");
        assert_eq!(payload["issues"][0]["code"], "fixture_jsonl_malformed");
        assert_eq!(payload["issues"][0]["kind"], "corpus");
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
