use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;

const SOURCE_KIND: &str = "external_provider_config_snapshot";
const TRUST_CLASS: &str = "diagnostic_only_until_validated";
const MAX_IMPORT_SOURCE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PRICE_ROW_SUMMARY_NODES: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ImportMode {
    DryRun,
    Apply,
}

#[derive(Debug)]
struct ImportOptions {
    source_file: Option<PathBuf>,
    mode: Option<ImportMode>,
    json: bool,
    blockers: Vec<&'static str>,
}

#[derive(Debug)]
struct SnapshotSummary {
    provider_count: usize,
    model_count: usize,
    price_like_row_count: usize,
}

#[derive(Debug)]
struct PricingImportSourceError {
    blocker_code: &'static str,
    next_action: String,
}

pub(crate) fn run_taskflow_pricing(args: &[String]) -> ExitCode {
    let rest = if matches!(args.first().map(String::as_str), Some("pricing")) {
        &args[1..]
    } else {
        args
    };

    match rest.first().map(String::as_str) {
        None | Some("--help" | "-h" | "help") => {
            print_taskflow_pricing_help();
            ExitCode::SUCCESS
        }
        Some("status") => run_pricing_status(rest),
        Some("providers") => run_pricing_providers(rest),
        Some("models") => run_pricing_models(rest),
        Some("receipt") => run_pricing_receipt(rest),
        Some("receipts") => run_pricing_receipts(rest),
        Some("import") => run_pricing_import(rest),
        Some(other) => fail_closed(
            rest.iter().any(|arg| arg == "--json"),
            "vida taskflow pricing",
            vec!["unsupported_pricing_command"],
            vec![format!(
                "Unsupported `vida taskflow pricing {other}` command. Use `vida taskflow pricing --help`."
            )],
        ),
    }
}

pub(crate) fn print_taskflow_pricing_help() {
    println!("VIDA TaskFlow help: pricing");
    println!();
    println!("Purpose:");
    println!(
        "  Inspect and bridge provider price snapshots through the VIDA price-catalog lifecycle."
    );
    println!(
        "  Imported snapshots are diagnostic until explicit dry-run/apply receipts and policy allow use."
    );
    println!();
    println!("Source of truth:");
    println!("  Hot-path routing uses active VIDA profile-local normalized cost units.");
    println!(
        "  External provider snapshots use source_kind `{SOURCE_KIND}` and trust_class `{TRUST_CLASS}`."
    );
    println!(
        "  Snapshot bridge output is not runtime authority and cannot override active prices."
    );
    println!();
    println!("Commands:");
    println!("  vida taskflow pricing status [--summary] [--json]");
    println!("  vida taskflow pricing providers [--json]");
    println!("  vida taskflow pricing models --provider <provider-id> [--json]");
    println!("  vida taskflow pricing receipt <receipt-id> [--json]");
    println!("  vida taskflow pricing receipts latest [--json]");
    println!("  vida taskflow pricing import --source-file <path> --dry-run [--json]");
    println!("  vida taskflow pricing import --source-file <path> --apply [--json]");
    println!();
    println!("Rules:");
    println!("  `--dry-run` and `--apply` are mutually exclusive.");
    println!("  `import` requires `--source-file` and one explicit mode.");
    println!(
        "  Phase-1 apply is fail-closed unless an authoritative price-catalog state store is available."
    );
}

fn run_pricing_status(args: &[String]) -> ExitCode {
    let json_output = args.iter().any(|arg| arg == "--json");
    let payload = pricing_status_payload(args.iter().any(|arg| arg == "--summary"));
    if json_output {
        print_json(&payload);
    } else {
        println!("VIDA price catalog readiness: profile-local compatibility mode");
        println!("Use `vida taskflow pricing status` for operator readiness; machine-readable fields are explicit opt-in.");
    }
    ExitCode::SUCCESS
}

fn run_pricing_providers(args: &[String]) -> ExitCode {
    let json_output = args.iter().any(|arg| arg == "--json");
    let payload = json!({
        "surface": "vida taskflow pricing providers",
        "status": "pass",
        "active_snapshot_id": serde_json::Value::Null,
        "providers": [],
        "price_catalog_readiness": pricing_readiness_payload(),
        "validity_scope": diagnostic_validity_scope(),
        "next_actions": [
            "Import a provider snapshot with `vida taskflow pricing import --source-file <path> --dry-run`."
        ]
    });
    print_or_summarize(
        json_output,
        &payload,
        "VIDA price catalog providers: no active snapshot",
    );
    ExitCode::SUCCESS
}

fn run_pricing_models(args: &[String]) -> ExitCode {
    let json_output = args.iter().any(|arg| arg == "--json");
    let provider = option_value(args, "--provider");
    if provider.is_none() {
        return fail_closed(
            json_output,
            "vida taskflow pricing models",
            vec!["pricing_models_provider_required"],
            vec!["Provide `--provider <provider-id>`.".to_string()],
        );
    }
    let payload = json!({
        "surface": "vida taskflow pricing models",
        "status": "pass",
        "provider_id": provider,
        "active_snapshot_id": serde_json::Value::Null,
        "models": [],
        "price_catalog_readiness": pricing_readiness_payload(),
        "validity_scope": diagnostic_validity_scope(),
        "next_actions": [
            "Import a provider snapshot with `vida taskflow pricing import --source-file <path> --dry-run`."
        ]
    });
    print_or_summarize(
        json_output,
        &payload,
        "VIDA price catalog models: no active snapshot",
    );
    ExitCode::SUCCESS
}

fn run_pricing_receipt(args: &[String]) -> ExitCode {
    let json_output = args.iter().any(|arg| arg == "--json");
    let receipt_id = args.get(1).filter(|value| !value.starts_with("--"));
    if receipt_id.is_none() {
        return fail_closed(
            json_output,
            "vida taskflow pricing receipt",
            vec!["pricing_receipt_id_required"],
            vec!["Provide `vida taskflow pricing receipt <receipt-id>`.".to_string()],
        );
    }
    fail_closed(
        json_output,
        "vida taskflow pricing receipt",
        vec!["price_catalog_receipt_not_found"],
        vec![format!(
            "No active price-catalog receipt `{}` is available in Phase 1 compatibility mode.",
            receipt_id.expect("receipt id checked")
        )],
    )
}

fn run_pricing_receipts(args: &[String]) -> ExitCode {
    let json_output = args.iter().any(|arg| arg == "--json");
    if !matches!(args.get(1).map(String::as_str), Some("latest")) {
        return fail_closed(
            json_output,
            "vida taskflow pricing receipts",
            vec!["unsupported_pricing_receipts_command"],
            vec!["Use `vida taskflow pricing receipts latest`.".to_string()],
        );
    }
    let payload = json!({
        "surface": "vida taskflow pricing receipts latest",
        "status": "pass",
        "latest_receipt_id": serde_json::Value::Null,
        "latest_receipt": serde_json::Value::Null,
        "price_catalog_readiness": pricing_readiness_payload(),
        "validity_scope": diagnostic_validity_scope(),
        "next_actions": [
            "Run `vida taskflow pricing import --source-file <path> --dry-run` to emit a dry-run receipt."
        ]
    });
    print_or_summarize(json_output, &payload, "VIDA price catalog receipts: none");
    ExitCode::SUCCESS
}

fn run_pricing_import(args: &[String]) -> ExitCode {
    let options = parse_import_options(args);
    if !options.blockers.is_empty() {
        return fail_closed(
            options.json,
            "vida taskflow pricing import",
            options.blockers,
            vec!["Provide exactly one mode and one source file.".to_string()],
        );
    }

    let Some(source_file) = options.source_file.as_ref() else {
        return fail_closed(
            options.json,
            "vida taskflow pricing import",
            vec!["pricing_import_source_file_required"],
            vec!["Provide `--source-file <path>`.".to_string()],
        );
    };
    let Some(mode) = options.mode else {
        return fail_closed(
            options.json,
            "vida taskflow pricing import",
            vec!["pricing_import_mode_required"],
            vec!["Provide `--dry-run` or `--apply`.".to_string()],
        );
    };

    let bytes = match read_import_source(source_file) {
        Ok(bytes) => bytes,
        Err(error) => {
            return fail_closed(
                options.json,
                "vida taskflow pricing import",
                vec![error.blocker_code],
                vec![error.next_action],
            )
        }
    };

    let payload = match build_import_payload(source_file, &bytes, mode) {
        Ok(payload) => payload,
        Err(blocker) => {
            return fail_closed(
                options.json,
                "vida taskflow pricing import",
                vec![blocker],
                vec!["Source file must be a JSON provider snapshot.".to_string()],
            )
        }
    };

    if options.json {
        print_json(&payload);
    } else {
        println!(
            "VIDA price catalog import {}",
            payload["status"].as_str().unwrap_or("blocked")
        );
        println!("Machine-readable receipt details are explicit opt-in.");
    }

    if payload["status"] == "blocked" {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}

fn read_import_source(source_file: &PathBuf) -> Result<Vec<u8>, PricingImportSourceError> {
    let metadata = fs::metadata(source_file).map_err(|error| PricingImportSourceError {
        blocker_code: "pricing_import_source_file_unreadable",
        next_action: format!("Could not inspect source file: {error}"),
    })?;

    if !metadata.is_file() {
        return Err(PricingImportSourceError {
            blocker_code: "pricing_import_source_file_not_regular",
            next_action: "Provide a regular JSON snapshot file; special files and directories are not supported."
                .to_string(),
        });
    }

    if metadata.len() > MAX_IMPORT_SOURCE_BYTES {
        return Err(PricingImportSourceError {
            blocker_code: "pricing_import_source_file_too_large",
            next_action: format!(
                "Provide a JSON snapshot no larger than {MAX_IMPORT_SOURCE_BYTES} bytes."
            ),
        });
    }

    let mut file = File::open(source_file).map_err(|error| PricingImportSourceError {
        blocker_code: "pricing_import_source_file_unreadable",
        next_action: format!("Could not open source file: {error}"),
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take(MAX_IMPORT_SOURCE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| PricingImportSourceError {
            blocker_code: "pricing_import_source_file_unreadable",
            next_action: format!("Could not read source file: {error}"),
        })?;

    if bytes.len() as u64 > MAX_IMPORT_SOURCE_BYTES {
        return Err(PricingImportSourceError {
            blocker_code: "pricing_import_source_file_too_large",
            next_action: format!(
                "Provide a JSON snapshot no larger than {MAX_IMPORT_SOURCE_BYTES} bytes."
            ),
        });
    }

    Ok(bytes)
}

fn parse_import_options(args: &[String]) -> ImportOptions {
    let mut source_file = None;
    let mut mode = None;
    let mut json = false;
    let mut blockers = Vec::new();
    let mut idx = 1;

    while idx < args.len() {
        match args[idx].as_str() {
            "--json" => {
                json = true;
                idx += 1;
            }
            "--dry-run" => {
                if mode.replace(ImportMode::DryRun).is_some() {
                    blockers.push("pricing_import_mode_conflict");
                }
                idx += 1;
            }
            "--apply" => {
                if mode.replace(ImportMode::Apply).is_some() {
                    blockers.push("pricing_import_mode_conflict");
                }
                idx += 1;
            }
            "--source-file" => {
                if let Some(path) = args.get(idx + 1) {
                    source_file = Some(PathBuf::from(path));
                    idx += 2;
                } else {
                    blockers.push("pricing_import_source_file_required");
                    idx += 1;
                }
            }
            "--receipt-note" => {
                idx += if args.get(idx + 1).is_some() { 2 } else { 1 };
            }
            "--fail-on-stale" => {
                idx += 1;
            }
            "--max-age-seconds" => {
                idx += if args.get(idx + 1).is_some() { 2 } else { 1 };
            }
            _ => {
                blockers.push("unsupported_pricing_import_option");
                idx += 1;
            }
        }
    }

    ImportOptions {
        source_file,
        mode,
        json,
        blockers,
    }
}

pub(crate) fn pricing_readiness_payload() -> serde_json::Value {
    json!({
        "enabled": false,
        "source_mode": "profile_local_compatibility",
        "active_snapshot_id": serde_json::Value::Null,
        "latest_receipt_id": serde_json::Value::Null,
        "freshness_status": "profile_local_fallback",
        "stale_provider_count": 0,
        "stale_model_count": 0,
        "missing_price_count": 0,
        "blocked_provider_count": 0,
        "fail_closed": false,
        "diagnostic_only_warnings": [
            "active_price_catalog_snapshot_missing"
        ],
        "next_actions": [
            "Use `vida taskflow pricing import --source-file <path> --dry-run` to preview external provider snapshot evidence."
        ]
    })
}

fn pricing_status_payload(summary: bool) -> serde_json::Value {
    json!({
        "surface": "vida taskflow pricing status",
        "status": "pass",
        "summary": summary,
        "price_catalog_readiness": pricing_readiness_payload(),
        "validity_scope": diagnostic_validity_scope()
    })
}

fn build_import_payload(
    source_file: &PathBuf,
    bytes: &[u8],
    mode: ImportMode,
) -> Result<serde_json::Value, &'static str> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "pricing_import_source_file_invalid_json")?;
    let summary = summarize_snapshot(&value);
    let digest = blake3::hash(bytes).to_hex().to_string();
    let short_digest = &digest[..16];
    let source_digest = format!("blake3:{digest}");
    let requested_scope = json!({
        "source_file": source_file.display().to_string(),
        "source_kind": SOURCE_KIND,
        "trust_class": TRUST_CLASS,
    });
    let proposed_changes = json!([{
        "change_class": "provider_snapshot_import_preview",
        "provider_count": summary.provider_count,
        "model_count": summary.model_count,
        "price_like_row_count": summary.price_like_row_count,
        "authority": "diagnostic_only"
    }]);
    let validity_scope = diagnostic_validity_scope();

    Ok(match mode {
        ImportMode::DryRun => json!({
            "surface": "vida taskflow pricing import",
            "status": "pass",
            "price_catalog_update_receipt": {
                "receipt_id": format!("price-catalog-dry-run-{short_digest}"),
                "status": "dry_run",
                "requested_scope": requested_scope,
                "source_digest": source_digest,
                "proposed_changes": proposed_changes,
                "rejected_changes": [],
                "blocker_codes": [],
                "would_activate_snapshot_id": format!("price-catalog-snapshot-preview-{short_digest}"),
                "next_actions": [
                    "Review dry-run evidence before any apply attempt.",
                    "Apply remains fail-closed until authoritative price-catalog state storage is enabled."
                ]
            },
            "validity_scope": validity_scope
        }),
        ImportMode::Apply => json!({
            "surface": "vida taskflow pricing import",
            "status": "blocked",
            "price_catalog_update_receipt": {
                "receipt_id": format!("price-catalog-apply-blocked-{short_digest}"),
                "status": "blocked",
                "requested_scope": requested_scope,
                "source_digest": source_digest,
                "applied_snapshot_id": serde_json::Value::Null,
                "applied_changes": [],
                "rejected_changes": proposed_changes,
                "freshness_status": "diagnostic_only_until_validated",
                "blocker_codes": [
                    "price_catalog_apply_requires_authoritative_lifecycle_store"
                ],
                "next_actions": [
                    "Use `--dry-run` for receipt-backed preview.",
                    "Enable authoritative price-catalog lifecycle storage before applying imported snapshots."
                ]
            },
            "blocker_codes": [
                "price_catalog_apply_requires_authoritative_lifecycle_store"
            ],
            "validity_scope": validity_scope
        }),
    })
}

fn summarize_snapshot(value: &serde_json::Value) -> SnapshotSummary {
    let provider_count = value
        .get("providers")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .or_else(|| {
            value
                .get("provider")
                .or_else(|| value.get("provider_id"))
                .map(|_| 1)
        })
        .unwrap_or(0);

    let mut model_count = value
        .get("models")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);

    if let Some(providers) = value.get("providers").and_then(serde_json::Value::as_array) {
        model_count += providers
            .iter()
            .filter_map(|provider| provider.get("models").and_then(serde_json::Value::as_array))
            .map(Vec::len)
            .sum::<usize>();
    }

    let price_like_row_count = count_price_like_rows(value);

    SnapshotSummary {
        provider_count,
        model_count,
        price_like_row_count,
    }
}

fn count_price_like_rows(value: &serde_json::Value) -> usize {
    let mut remaining_nodes = MAX_PRICE_ROW_SUMMARY_NODES;
    count_price_like_rows_bounded(value, &mut remaining_nodes)
}

fn count_price_like_rows_bounded(value: &serde_json::Value, remaining_nodes: &mut usize) -> usize {
    if *remaining_nodes == 0 {
        return 0;
    }
    *remaining_nodes -= 1;

    match value {
        serde_json::Value::Array(rows) => rows
            .iter()
            .map(|row| count_price_like_rows_bounded(row, remaining_nodes))
            .sum(),
        serde_json::Value::Object(map) => {
            let current = [
                "normalized_cost_units",
                "cost_units",
                "input_cost_per_million",
                "output_cost_per_million",
                "price",
                "pricing",
            ]
            .iter()
            .any(|key| map.contains_key(*key)) as usize;
            current
                + map
                    .values()
                    .map(|value| count_price_like_rows_bounded(value, remaining_nodes))
                    .sum::<usize>()
        }
        _ => 0,
    }
}

fn option_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == name)
        .and_then(|window| {
            if window[1].starts_with("--") {
                None
            } else {
                Some(window[1].clone())
            }
        })
}

fn diagnostic_validity_scope() -> serde_json::Value {
    json!({
        "diagnostic_only": true,
        "not_runtime_authority": true,
        "hot_path_mutation": false
    })
}

fn print_or_summarize(json_output: bool, payload: &serde_json::Value, summary: &str) {
    if json_output {
        print_json(payload);
    } else {
        println!("{summary}");
        println!("Machine-readable pricing diagnostics are explicit opt-in.");
    }
}

fn fail_closed(
    json_output: bool,
    surface: &str,
    blocker_codes: Vec<&str>,
    next_actions: Vec<String>,
) -> ExitCode {
    if json_output {
        print_json(&json!({
            "surface": surface,
            "status": "blocked",
            "blocker_codes": blocker_codes,
            "next_actions": next_actions,
        }));
    } else {
        eprintln!("{surface} blocked: {}", blocker_codes.join(","));
        for action in next_actions {
            eprintln!("next: {action}");
        }
    }
    ExitCode::from(2)
}

fn print_json(value: &serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("pricing payload should serialize")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pricing_status_payload_exposes_required_readiness_fields() {
        let payload = pricing_status_payload(true);
        let readiness = &payload["price_catalog_readiness"];
        assert_eq!(payload["surface"], "vida taskflow pricing status");
        assert_eq!(readiness["enabled"], false);
        assert_eq!(readiness["source_mode"], "profile_local_compatibility");
        assert_eq!(readiness["freshness_status"], "profile_local_fallback");
        assert_eq!(payload["validity_scope"]["not_runtime_authority"], true);
    }

    #[test]
    fn import_dry_run_receipt_is_diagnostic_only_and_source_neutral() {
        let bytes = br#"{
            "providers": [
                {
                    "id": "provider-a",
                    "models": [
                        {"model_ref": "model-a", "normalized_cost_units": 3}
                    ]
                }
            ]
        }"#;
        let payload =
            build_import_payload(&PathBuf::from("snapshot.json"), bytes, ImportMode::DryRun)
                .expect("dry-run import payload");
        let receipt = &payload["price_catalog_update_receipt"];
        assert_eq!(payload["status"], "pass");
        assert_eq!(receipt["status"], "dry_run");
        assert_eq!(
            receipt["requested_scope"]["source_kind"],
            "external_provider_config_snapshot"
        );
        assert_eq!(
            receipt["requested_scope"]["trust_class"],
            "diagnostic_only_until_validated"
        );
        assert_eq!(payload["validity_scope"]["not_runtime_authority"], true);
        assert_eq!(
            receipt["proposed_changes"][0]["price_like_row_count"]
                .as_u64()
                .expect("price-like count"),
            1
        );
    }

    #[test]
    fn import_apply_fails_closed_without_authoritative_lifecycle_store() {
        let bytes = br#"{"provider_id":"provider-a","models":[{"id":"model-a","pricing":{}}]}"#;
        let payload =
            build_import_payload(&PathBuf::from("snapshot.json"), bytes, ImportMode::Apply)
                .expect("apply payload");
        assert_eq!(payload["status"], "blocked");
        assert_eq!(
            payload["blocker_codes"][0],
            "price_catalog_apply_requires_authoritative_lifecycle_store"
        );
        assert!(payload["price_catalog_update_receipt"]["applied_snapshot_id"].is_null());
    }

    #[test]
    fn read_import_source_rejects_oversized_files_before_reading() {
        let path = unique_temp_path("oversized-pricing-snapshot.json");
        let file = File::create(&path).expect("create sparse oversized snapshot");
        file.set_len(MAX_IMPORT_SOURCE_BYTES + 1)
            .expect("mark file oversized");

        let error = read_import_source(&path).expect_err("oversized source should fail closed");
        assert_eq!(error.blocker_code, "pricing_import_source_file_too_large");

        fs::remove_file(path).expect("remove oversized snapshot");
    }

    #[test]
    fn read_import_source_rejects_non_regular_paths() {
        let path = unique_temp_path("pricing-snapshot-directory");
        fs::create_dir(&path).expect("create snapshot directory");

        let error = read_import_source(&path).expect_err("directory source should fail closed");
        assert_eq!(error.blocker_code, "pricing_import_source_file_not_regular");

        fs::remove_dir(path).expect("remove snapshot directory");
    }

    #[test]
    fn read_import_source_accepts_bounded_regular_file() {
        let path = unique_temp_path("bounded-pricing-snapshot.json");
        fs::write(&path, br#"{"provider_id":"provider-a"}"#).expect("write bounded snapshot");

        let bytes = read_import_source(&path).expect("bounded source should be readable");
        assert_eq!(bytes, br#"{"provider_id":"provider-a"}"#);

        fs::remove_file(path).expect("remove bounded snapshot");
    }

    #[test]
    fn import_parser_rejects_conflicting_modes() {
        let args = vec![
            "import".to_string(),
            "--source-file".to_string(),
            "snapshot.json".to_string(),
            "--dry-run".to_string(),
            "--apply".to_string(),
            "--json".to_string(),
        ];
        let options = parse_import_options(&args);
        assert!(options.json);
        assert!(options.blockers.contains(&"pricing_import_mode_conflict"));
    }
    fn unique_temp_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("vida-{nanos}-{name}"))
    }
}
