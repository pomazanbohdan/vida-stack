use std::ffi::OsString;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::ReleaseInstallArgs;

#[cfg(test)]
static RELEASE_INSTALL_PROGRESS_DIR_OVERRIDE: std::sync::OnceLock<
    std::sync::Mutex<Option<PathBuf>>,
> = std::sync::OnceLock::new();

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseInstallReceipt {
    pub status: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub build: ReleaseBuildReceipt,
    pub asset_update: ReleaseAssetUpdateReceipt,
    pub install_layout: Option<ReleaseInstallLayout>,
    pub source_binary_path: String,
    pub source_binary_fingerprint: Option<String>,
    pub requested_target: String,
    pub installed_targets: Vec<ReleaseInstalledTarget>,
    pub io_error: Option<ReleaseIoErrorDetail>,
    pub error_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseBuildReceipt {
    pub status: String,
    pub phase: String,
    pub skipped: bool,
    pub command: Option<Vec<String>>,
    pub exit_code: Option<i32>,
    pub process_id: Option<u32>,
    pub child_state: Option<String>,
    pub progress_path: Option<String>,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseInstallProgressStatusReceipt {
    pub surface: String,
    pub status: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub latest_status: Option<String>,
    pub latest_phase: Option<String>,
    pub latest_path: String,
    pub progress_path: Option<String>,
    pub process_id: Option<u32>,
    pub child_state: Option<String>,
    pub installed_targets: Vec<ReleaseInstalledTarget>,
    pub artifact_refs: Vec<String>,
    pub latest_event: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseAssetUpdateReceipt {
    pub status: String,
    pub refreshed_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseInstalledTarget {
    pub target: String,
    pub path: String,
    pub fingerprint: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseInstallLayout {
    pub install_root: String,
    pub current_root: String,
    pub runtime_bin_dir: String,
    pub env_file: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseIoErrorDetail {
    pub operation: String,
    pub target_path: Option<String>,
    pub staging_path: Option<String>,
    pub error_kind: String,
    pub error_message: String,
    pub next_action_hint: String,
}

const INSTALL_BINARY_RETRY_LIMIT: usize = 6;

pub(crate) fn run_release_install(args: ReleaseInstallArgs) -> ExitCode {
    if args.status {
        let receipt = release_install_status_receipt();
        return emit_release_install_status_receipt(&receipt, args.json);
    }
    let receipt = release_install_receipt(&args);
    emit_release_install_receipt(&receipt, args.json)
}

fn emit_release_install_status_receipt(
    receipt: &ReleaseInstallProgressStatusReceipt,
    json: bool,
) -> ExitCode {
    if json {
        match serde_json::to_string_pretty(receipt) {
            Ok(body) => println!("{body}"),
            Err(error) => {
                eprintln!("failed to render release install status receipt: {error}");
                return ExitCode::from(1);
            }
        }
    } else if receipt.status == "pass" || receipt.status == "running" {
        println!("release install status: {}", receipt.status);
        if let Some(process_id) = receipt.process_id {
            println!(
                "build process: pid={} state={}",
                process_id,
                receipt.child_state.as_deref().unwrap_or("unknown")
            );
        }
        if let Some(progress_path) = receipt.progress_path.as_deref() {
            println!("progress artifact: {progress_path}");
        }
        for target in &receipt.installed_targets {
            println!(
                "installed target: {} path={} version={} fingerprint={}",
                target.target,
                target.path,
                target.version.as_deref().unwrap_or("unknown"),
                target.fingerprint
            );
        }
        for action in &receipt.next_actions {
            println!("next action: {action}");
        }
    } else {
        eprintln!(
            "release install status: blocked ({})",
            receipt.blocker_codes.join(", ")
        );
        if let Some(progress_path) = receipt.progress_path.as_deref() {
            eprintln!("progress artifact: {progress_path}");
        }
        for action in &receipt.next_actions {
            eprintln!("next action: {action}");
        }
    }

    if receipt.status == "blocked" {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn emit_release_install_receipt(receipt: &ReleaseInstallReceipt, json: bool) -> ExitCode {
    if json {
        match serde_json::to_string_pretty(receipt) {
            Ok(body) => println!("{body}"),
            Err(error) => {
                eprintln!("failed to render release install receipt: {error}");
                return ExitCode::from(1);
            }
        }
    } else if receipt.status == "pass" {
        println!(
            "release install: pass (installed {} target(s))",
            receipt.installed_targets.len()
        );
        if let Some(progress_path) = receipt.build.progress_path.as_deref() {
            println!("progress artifact: {progress_path}");
        }
    } else {
        eprintln!(
            "release install: blocked ({})",
            receipt.blocker_codes.join(", ")
        );
        if let Some(progress_path) = receipt.build.progress_path.as_deref() {
            eprintln!("progress artifact: {progress_path}");
        }
    }

    if receipt.status == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(crate) fn release_install_receipt(args: &ReleaseInstallArgs) -> ReleaseInstallReceipt {
    let requested_target = args.target.trim().to_string();
    let source_binary = args
        .source_binary
        .clone()
        .unwrap_or_else(default_source_binary_path);
    let source_binary_path = source_binary.display().to_string();
    let install_layout = release_install_layout(args.install_root.as_deref());

    let target_paths = match install_target_paths(&requested_target, args.install_root.as_deref()) {
        Ok(paths) => paths,
        Err(receipt) => {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                ReleaseBuildReceipt {
                    status: if args.skip_build {
                        "skipped".to_string()
                    } else {
                        "not_started".to_string()
                    },
                    phase: "build".to_string(),
                    skipped: args.skip_build,
                    command: None,
                    exit_code: None,
                    process_id: None,
                    child_state: None,
                    progress_path: None,
                    artifact_refs: Vec::new(),
                },
                receipt,
            );
        }
    };

    if !args.skip_build {
        let status_receipt = release_install_status_receipt();
        if status_receipt.status == "running" {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                ReleaseBuildReceipt {
                    status: "blocked".to_string(),
                    phase: "build".to_string(),
                    skipped: false,
                    command: Some(release_build_command()),
                    exit_code: None,
                    process_id: status_receipt.process_id,
                    child_state: status_receipt.child_state.clone(),
                    progress_path: status_receipt.progress_path.clone(),
                    artifact_refs: status_receipt.artifact_refs.clone(),
                },
                BlockedRelease {
                    blocker_code: "release_install_build_already_running",
                    next_action:
                        "Release install build is already running; run `vida release install --status --json` and wait before starting another install."
                            .to_string(),
                    io_error: None,
                },
            );
        }
    }

    let mut build = release_build_receipt(args.skip_build);
    if build.status == "blocked" {
        return blocked_receipt(
            requested_target,
            source_binary_path,
            build,
            BlockedRelease {
                blocker_code: "release_build_failed",
                next_action:
                    "Fix release build failures, then rerun `vida release install --json`."
                        .to_string(),
                io_error: None,
            },
        );
    }

    if !source_binary.is_file() {
        return blocked_receipt(
            requested_target,
            source_binary_path,
            build,
            BlockedRelease {
                blocker_code: "missing_source_binary",
                next_action:
                    "Run `cargo build -p vida --release` or pass `--source-binary <path>`."
                        .to_string(),
                io_error: None,
            },
        );
    }

    let source_binary_fingerprint = match binary_fingerprint(&source_binary) {
        Ok(fingerprint) => fingerprint,
        Err(io_error) => {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                build,
                BlockedRelease {
                    blocker_code: "missing_source_binary",
                    next_action: "Ensure the source binary is readable, then rerun `vida release install --json`."
                        .to_string(),
                    io_error: Some(io_error),
                },
            );
        }
    };

    let mut installed_targets = Vec::new();
    for (target, path) in target_paths {
        if let Err(blocked) =
            install_release_binary_target(&source_binary, &path, target, &mut installed_targets)
        {
            record_release_install_phase_progress(
                &build,
                "blocked",
                Some(1),
                Some("install_failed"),
            );
            return blocked_receipt(requested_target, source_binary_path, build, blocked);
        }
    }

    if args.source_binary.is_none() {
        let companion_targets = match companion_runtime_install_target_paths(
            &requested_target,
            args.install_root.as_deref(),
        ) {
            Ok(paths) => paths,
            Err(blocked) => {
                return blocked_receipt(requested_target, source_binary_path, build, blocked);
            }
        };
        let pi_agent_source = default_pi_agent_source_binary_path();
        if !companion_targets.is_empty() && !pi_agent_source.is_file() {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                build,
                BlockedRelease {
                    blocker_code: "missing_source_binary",
                    next_action: "Run `cargo build -p vida-pi-agent --release`, or rerun without `--skip-build`."
                        .to_string(),
                    io_error: None,
                },
            );
        }
        for (target, path) in companion_targets {
            if let Err(blocked) = install_release_binary_target(
                &pi_agent_source,
                &path,
                target,
                &mut installed_targets,
            ) {
                record_release_install_phase_progress(
                    &build,
                    "blocked",
                    Some(1),
                    Some("install_failed"),
                );
                return blocked_receipt(requested_target, source_binary_path, build, blocked);
            }
        }
    }

    let asset_update = if installed_targets
        .iter()
        .any(|target| target.target == "current")
    {
        let current_root = match install_layout.as_ref() {
            Some(layout) => PathBuf::from(&layout.current_root),
            None => {
                record_release_install_phase_progress(
                    &build,
                    "blocked",
                    Some(1),
                    Some("asset_failed"),
                );
                return blocked_receipt(
                    requested_target,
                    source_binary_path,
                    build,
                    BlockedRelease {
                        blocker_code: "release_asset_materialization_failed",
                        next_action:
                            "Resolve the current install layout and rerun `vida release install --json`."
                                .to_string(),
                        io_error: None,
                    },
                );
            }
        };
        match materialize_release_runtime_assets(&current_root) {
            Ok(update) => update,
            Err(io_error) => {
                record_release_install_phase_progress(
                    &build,
                    "blocked",
                    Some(1),
                    Some("asset_failed"),
                );
                return blocked_receipt(
                    requested_target,
                    source_binary_path,
                    build,
                    BlockedRelease {
                        blocker_code: "release_asset_materialization_failed",
                        next_action: io_error.next_action_hint.clone(),
                        io_error: Some(io_error),
                    },
                );
            }
        }
    } else {
        ReleaseAssetUpdateReceipt {
            status: "skipped_non_current_target".to_string(),
            refreshed_paths: Vec::new(),
        }
    };

    record_skip_build_release_install_progress(&mut build);
    record_release_install_phase_progress(&build, "pass", Some(0), Some("completed"));

    ReleaseInstallReceipt {
        status: "pass".to_string(),
        blocker_codes: Vec::new(),
        next_actions: vec![
            "Run `vida --help` from a new shell and verify the expected binary is first on PATH."
                .to_string(),
            "Run `vida init` in downstream projects to refresh framework-owned project assets."
                .to_string(),
        ],
        asset_update,
        build,
        install_layout,
        source_binary_path,
        source_binary_fingerprint: Some(source_binary_fingerprint),
        requested_target,
        installed_targets,
        io_error: None,
        error_kind: None,
    }
}

fn install_release_binary_target(
    source: &Path,
    destination: &Path,
    target: String,
    installed_targets: &mut Vec<ReleaseInstalledTarget>,
) -> Result<(), BlockedRelease> {
    if let Some(parent) = destination.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            let io_error = io_error_detail("create_dir", Some(parent), None, &error);
            return Err(BlockedRelease {
                blocker_code: release_install_error_blocker_code(&io_error.error_kind),
                next_action: io_error.next_action_hint.clone(),
                io_error: Some(io_error),
            });
        }
    }
    if let Err(io_error) = install_binary(source, destination) {
        return Err(BlockedRelease {
            blocker_code: release_install_error_blocker_code(&io_error.error_kind),
            next_action: io_error.next_action_hint.clone(),
            io_error: Some(io_error),
        });
    }
    let fingerprint = binary_fingerprint(destination).map_err(|io_error| BlockedRelease {
        blocker_code: release_install_error_blocker_code(&io_error.error_kind),
        next_action: io_error.next_action_hint.clone(),
        io_error: Some(io_error),
    })?;
    installed_targets.push(ReleaseInstalledTarget {
        target,
        path: destination.display().to_string(),
        fingerprint: fingerprint.clone(),
        version: None,
    });
    let _ = write_binary_fingerprint_metadata(destination, &fingerprint);
    Ok(())
}

fn record_release_install_phase_progress(
    build: &ReleaseBuildReceipt,
    status: &str,
    exit_code: Option<i32>,
    child_state: Option<&str>,
) {
    let Some(progress_path) = build.progress_path.as_deref() else {
        return;
    };
    let command = vec![
        "vida".to_string(),
        "release".to_string(),
        "install".to_string(),
    ];
    let _ = write_release_install_progress_event_with_child(
        Path::new(progress_path),
        status,
        "install",
        &command,
        exit_code,
        None,
        child_state,
    );
}

fn release_build_command() -> Vec<String> {
    vec![
        "cargo".to_string(),
        "build".to_string(),
        "-p".to_string(),
        "vida".to_string(),
        "-p".to_string(),
        "vida-pi-agent".to_string(),
        "--release".to_string(),
    ]
}

fn progress_artifact_refs(progress_path: Option<&Path>) -> Vec<String> {
    progress_path
        .map(|path| vec![path.display().to_string()])
        .unwrap_or_default()
}

fn release_build_progress_write_failed_receipt(
    command: Vec<String>,
    progress_path: Option<&Path>,
) -> ReleaseBuildReceipt {
    ReleaseBuildReceipt {
        status: "blocked".to_string(),
        phase: "build".to_string(),
        skipped: false,
        command: Some(command),
        exit_code: None,
        process_id: None,
        child_state: Some("progress_write_failed".to_string()),
        progress_path: progress_path.map(|path| path.display().to_string()),
        artifact_refs: progress_artifact_refs(progress_path),
    }
}

pub(crate) fn release_build_receipt(skip_build: bool) -> ReleaseBuildReceipt {
    if skip_build {
        return ReleaseBuildReceipt {
            status: "skipped".to_string(),
            phase: "build".to_string(),
            skipped: true,
            command: None,
            exit_code: None,
            process_id: None,
            child_state: None,
            progress_path: None,
            artifact_refs: Vec::new(),
        };
    }

    let command = release_build_command();
    let progress_path = release_install_progress_path();
    if let Some(path) = progress_path.as_ref() {
        eprintln!("release install progress: {}", path.display());
        if write_release_install_progress_event_with_child(
            path,
            "started",
            "build",
            &command,
            None,
            None,
            Some("starting"),
        )
        .is_err()
        {
            return release_build_progress_write_failed_receipt(command, progress_path.as_deref());
        }
    }
    let mut cargo = Command::new("cargo");
    cargo
        .args(command.iter().skip(1).map(String::as_str))
        .current_dir(trusted_workspace_root());
    match cargo.spawn() {
        Ok(mut child) => {
            let process_id = child.id();
            if let Some(path) = progress_path.as_ref() {
                let _ = write_release_install_progress_event_with_child(
                    path,
                    "started",
                    "build",
                    &command,
                    None,
                    Some(process_id),
                    Some("running"),
                );
            }
            match child.wait() {
                Ok(status) if status.success() => {
                    if let Some(path) = progress_path.as_ref() {
                        let _ = write_release_install_progress_event_with_child(
                            path,
                            "pass",
                            "build",
                            &command,
                            status.code(),
                            Some(process_id),
                            Some("completed"),
                        );
                    }
                    ReleaseBuildReceipt {
                        status: "pass".to_string(),
                        phase: "build".to_string(),
                        skipped: false,
                        command: Some(command),
                        exit_code: status.code(),
                        process_id: Some(process_id),
                        child_state: Some("completed".to_string()),
                        progress_path: progress_path
                            .as_ref()
                            .map(|path| path.display().to_string()),
                        artifact_refs: progress_artifact_refs(progress_path.as_deref()),
                    }
                }
                Ok(status) => {
                    if let Some(path) = progress_path.as_ref() {
                        let _ = write_release_install_progress_event_with_child(
                            path,
                            "blocked",
                            "build",
                            &command,
                            status.code(),
                            Some(process_id),
                            Some("completed_failed"),
                        );
                    }
                    ReleaseBuildReceipt {
                        status: "blocked".to_string(),
                        phase: "build".to_string(),
                        skipped: false,
                        command: Some(command),
                        exit_code: status.code(),
                        process_id: Some(process_id),
                        child_state: Some("completed_failed".to_string()),
                        progress_path: progress_path
                            .as_ref()
                            .map(|path| path.display().to_string()),
                        artifact_refs: progress_artifact_refs(progress_path.as_deref()),
                    }
                }
                Err(_) => {
                    if let Some(path) = progress_path.as_ref() {
                        let _ = write_release_install_progress_event_with_child(
                            path,
                            "blocked",
                            "build",
                            &command,
                            None,
                            Some(process_id),
                            Some("wait_failed"),
                        );
                    }
                    ReleaseBuildReceipt {
                        status: "blocked".to_string(),
                        phase: "build".to_string(),
                        skipped: false,
                        command: Some(command),
                        exit_code: None,
                        process_id: Some(process_id),
                        child_state: Some("wait_failed".to_string()),
                        progress_path: progress_path
                            .as_ref()
                            .map(|path| path.display().to_string()),
                        artifact_refs: progress_artifact_refs(progress_path.as_deref()),
                    }
                }
            }
        }
        Err(_) => {
            if let Some(path) = progress_path.as_ref() {
                let _ = write_release_install_progress_event_with_child(
                    path,
                    "blocked",
                    "build",
                    &command,
                    None,
                    None,
                    Some("spawn_failed"),
                );
            }
            ReleaseBuildReceipt {
                status: "blocked".to_string(),
                phase: "build".to_string(),
                skipped: false,
                command: Some(command),
                exit_code: None,
                process_id: None,
                child_state: Some("spawn_failed".to_string()),
                progress_path: progress_path
                    .as_ref()
                    .map(|path| path.display().to_string()),
                artifact_refs: progress_artifact_refs(progress_path.as_deref()),
            }
        }
    }
}

fn release_install_status_receipt() -> ReleaseInstallProgressStatusReceipt {
    let progress_dir = release_install_progress_dir();
    let latest_path = release_install_progress_latest_path();
    let latest_path_string = latest_path.display().to_string();
    if let Err(error) = release_install_progress_readable_dir(&progress_dir) {
        return release_install_progress_unreadable_status_receipt(
            latest_path_string,
            &latest_path,
            error.to_string(),
        );
    }
    match fs::symlink_metadata(&latest_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return release_install_progress_unreadable_status_receipt(
                latest_path_string,
                &latest_path,
                "release install progress marker is a symlink".to_string(),
            );
        }
        Ok(metadata) if !metadata.is_file() => {
            return release_install_progress_unreadable_status_receipt(
                latest_path_string,
                &latest_path,
                "release install progress marker is not a regular file".to_string(),
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if let Some((latest_event, progress_path)) =
                latest_release_install_progress_artifact_event()
            {
                return release_install_status_receipt_from_event(
                    latest_path_string,
                    latest_event,
                    Vec::new(),
                    Some(progress_path),
                );
            }
            return ReleaseInstallProgressStatusReceipt {
                surface: "vida release install --status".to_string(),
                status: "blocked".to_string(),
                blocker_codes: vec!["release_install_progress_missing".to_string()],
                next_actions: vec![
                    "Run `vida release install --json` to start a release install, or inspect the install target directly."
                        .to_string(),
                ],
                latest_status: None,
                latest_phase: None,
                latest_path: latest_path_string,
                progress_path: None,
                process_id: None,
                child_state: None,
                installed_targets: Vec::new(),
                artifact_refs: Vec::new(),
                latest_event: None,
            };
        }
        Err(error) => {
            return release_install_progress_unreadable_status_receipt(
                latest_path_string,
                &latest_path,
                error.to_string(),
            );
        }
    }
    let latest_raw =
        match read_release_install_progress_file_without_following_symlinks(&latest_path) {
            Ok(raw) => raw,
            Err(error) => {
                return release_install_progress_unreadable_status_receipt(
                    latest_path_string,
                    &latest_path,
                    error.to_string(),
                );
            }
        };
    let latest_event: serde_json::Value = match serde_json::from_str(&latest_raw) {
        Ok(value) => value,
        Err(error) => {
            return ReleaseInstallProgressStatusReceipt {
                surface: "vida release install --status".to_string(),
                status: "blocked".to_string(),
                blocker_codes: vec!["release_install_progress_invalid".to_string()],
                next_actions: vec![format!(
                    "Repair or remove invalid release install progress marker `{}`: {error}",
                    latest_path.display()
                )],
                latest_status: None,
                latest_phase: None,
                latest_path: latest_path_string,
                progress_path: None,
                process_id: None,
                child_state: None,
                installed_targets: Vec::new(),
                artifact_refs: vec![latest_path.display().to_string()],
                latest_event: None,
            };
        }
    };
    release_install_status_receipt_from_event(
        latest_path_string,
        latest_event,
        vec![latest_path.display().to_string()],
        None,
    )
}

fn release_install_status_receipt_from_event(
    latest_path_string: String,
    latest_event: serde_json::Value,
    mut artifact_refs: Vec<String>,
    fallback_progress_path: Option<String>,
) -> ReleaseInstallProgressStatusReceipt {
    let latest_status = latest_event
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let latest_phase = latest_event
        .get("phase")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let process_id = latest_event
        .get("process_id")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let recorded_child_state = latest_event
        .get("child_state")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let child_state = if latest_status.as_deref() == Some("started") {
        process_id
            .map(release_install_process_liveness)
            .or(recorded_child_state)
    } else {
        recorded_child_state
    };
    let progress_path = latest_event
        .get("progress_path")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .or_else(release_install_progress_latest_path_marker_contents)
        .or(fallback_progress_path);
    if let Some(path) = progress_path.as_ref() {
        if !artifact_refs.iter().any(|artifact| artifact == path) {
            artifact_refs.push(path.clone());
        }
    }
    let (status, blocker_codes, next_actions) = release_install_progress_status_contract(
        latest_status.as_deref(),
        latest_phase.as_deref(),
        child_state.as_deref(),
    );
    let installed_targets = if status == "pass" && latest_phase.as_deref() == Some("install") {
        current_release_installed_target_identity()
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };
    ReleaseInstallProgressStatusReceipt {
        surface: "vida release install --status".to_string(),
        status,
        blocker_codes,
        next_actions,
        latest_status,
        latest_phase,
        latest_path: latest_path_string,
        progress_path,
        process_id,
        child_state,
        installed_targets,
        artifact_refs,
        latest_event: Some(latest_event),
    }
}

fn release_install_progress_unreadable_status_receipt(
    latest_path_string: String,
    latest_path: &Path,
    detail: String,
) -> ReleaseInstallProgressStatusReceipt {
    ReleaseInstallProgressStatusReceipt {
        surface: "vida release install --status".to_string(),
        status: "blocked".to_string(),
        blocker_codes: vec!["release_install_progress_unreadable".to_string()],
        next_actions: vec![format!(
            "Inspect release install progress marker `{}`: {detail}",
            latest_path.display()
        )],
        latest_status: None,
        latest_phase: None,
        latest_path: latest_path_string,
        progress_path: None,
        process_id: None,
        child_state: None,
        installed_targets: Vec::new(),
        artifact_refs: vec![latest_path.display().to_string()],
        latest_event: None,
    }
}

fn release_install_progress_readable_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        .unwrap_or(false)
}

fn release_install_progress_readable_dir(path: &Path) -> io::Result<()> {
    reject_existing_symlinks_in_path(path)?;
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "release install progress directory is not a directory: {}",
                path.display()
            ),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn release_install_progress_latest_path_marker_contents() -> Option<String> {
    let path_marker = release_install_progress_latest_path().with_extension("path");
    let metadata = fs::symlink_metadata(&path_marker).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }

    read_release_install_progress_file_without_following_symlinks(&path_marker).ok()
}

fn read_release_install_progress_file_without_following_symlinks(
    path: &Path,
) -> io::Result<String> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "release install progress artifact is not a regular file or is a symlink: {}",
                path.display()
            ),
        ));
    }

    let mut options = OpenOptions::new();
    options.read(true);
    apply_no_follow_open_options(&mut options);
    let mut file = options.open(path)?;
    let opened_metadata = file.metadata()?;
    if !opened_metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "release install progress artifact is not a regular file: {}",
                path.display()
            ),
        ));
    }

    let mut body = String::new();
    file.read_to_string(&mut body)?;
    Ok(body)
}

fn latest_release_install_progress_artifact_event() -> Option<(serde_json::Value, String)> {
    let progress_dir = release_install_progress_dir();
    release_install_progress_readable_dir(&progress_dir).ok()?;
    let mut candidates = fs::read_dir(progress_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            if !file_name.starts_with("release-install-") || !file_name.ends_with(".jsonl") {
                return None;
            }
            if !release_install_progress_readable_regular_file(&path) {
                return None;
            }
            let body = read_release_install_progress_file_without_following_symlinks(&path).ok()?;
            let event = body
                .lines()
                .rev()
                .find(|line| !line.trim().is_empty())
                .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())?;
            let recorded_at = event
                .get("recorded_at_unix_ms")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            Some((recorded_at, path, event))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| right.1.cmp(&left.1)));

    candidates
        .into_iter()
        .next()
        .map(|(_, path, event)| (event, path.display().to_string()))
}

fn release_install_progress_status_contract(
    latest_status: Option<&str>,
    latest_phase: Option<&str>,
    child_state: Option<&str>,
) -> (String, Vec<String>, Vec<String>) {
    match latest_status {
        Some("pass") if latest_phase == Some("install") => (
            "pass".to_string(),
            Vec::new(),
            vec!["Release install completed; verify `vida --version` and PATH if needed.".to_string()],
        ),
        Some("pass") => (
            "blocked".to_string(),
            vec!["release_install_progress_missing_install_phase".to_string()],
            vec![
                "Latest release install progress only proves the build phase; rerun `vida release install --json` or inspect the target hash before treating the install as complete."
                    .to_string(),
            ],
        ),
        Some("started")
            if matches!(
                child_state,
                Some("alive" | "unknown" | "starting" | "running") | None
            ) =>
        {
            (
            "running".to_string(),
            Vec::new(),
            vec![
                "Release install build is still running or its liveness is not yet proven; do not start another release install yet."
                    .to_string(),
                "Rerun `vida release install --status --json` to poll the existing build."
                    .to_string(),
            ],
            )
        }
        Some("started") => (
            "blocked".to_string(),
            vec!["release_install_progress_stale_started".to_string()],
            vec![
                "Latest release install progress is `started`, but the recorded build process is not alive; inspect the progress artifact before rerunning `vida release install --json`."
                    .to_string(),
            ],
        ),
        Some("blocked") => (
            "blocked".to_string(),
            vec!["release_install_progress_blocked".to_string()],
            vec![
                "Inspect the progress artifact, fix the recorded release build blocker, then rerun `vida release install --json`."
                    .to_string(),
            ],
        ),
        _ => (
            "blocked".to_string(),
            vec!["release_install_progress_unknown".to_string()],
            vec![
                "Inspect the latest release install progress marker before starting another release install."
                    .to_string(),
            ],
        ),
    }
}

fn release_install_process_liveness(process_id: u32) -> String {
    if process_id == process::id() {
        return "alive".to_string();
    }
    release_install_process_liveness_impl(process_id)
}

#[cfg(target_os = "windows")]
fn release_install_process_liveness_impl(process_id: u32) -> String {
    let tasklist_path = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32")
        .join("tasklist.exe");
    let Ok(output) = Command::new(tasklist_path)
        .args(["/FI", &format!("PID eq {process_id}"), "/FO", "CSV", "/NH"])
        .output()
    else {
        return "unknown".to_string();
    };
    if !output.status.success() {
        return "unknown".to_string();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let needle = format!(",\"{process_id}\",");
    if stdout.contains(&needle) {
        "alive".to_string()
    } else if !stdout
        .lines()
        .any(|line| line.trim_start().starts_with('"'))
    {
        "dead".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(unix)]
fn release_install_process_liveness_impl(process_id: u32) -> String {
    match Command::new("ps")
        .args(["-p", &process_id.to_string(), "-o", "pid="])
        .output()
    {
        Ok(output) if output.status.success() => "alive".to_string(),
        Ok(_) => "dead".to_string(),
        Err(_) => "unknown".to_string(),
    }
}

#[cfg(not(any(unix, target_os = "windows")))]
fn release_install_process_liveness_impl(_process_id: u32) -> String {
    "unknown".to_string()
}

fn record_skip_build_release_install_progress(build: &mut ReleaseBuildReceipt) {
    if !build.skipped {
        return;
    }
    let command = vec![
        "vida".to_string(),
        "release".to_string(),
        "install".to_string(),
        "--skip-build".to_string(),
    ];
    let progress_path = release_install_progress_path();
    if let Some(path) = progress_path.as_ref() {
        let _ = write_release_install_progress_event(path, "pass", &command, Some(0));
    }
    build.command = Some(command);
    build.exit_code = Some(0);
    build.phase = "build".to_string();
    build.process_id = None;
    build.child_state = Some("skipped".to_string());
    build.progress_path = progress_path
        .as_ref()
        .map(|path| path.display().to_string());
    build.artifact_refs = progress_path
        .as_ref()
        .map(|path| vec![path.display().to_string()])
        .unwrap_or_default();
}

fn release_install_progress_path() -> Option<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some(release_install_progress_dir().join(format!("release-install-{stamp}.jsonl")))
}

fn release_install_progress_dir() -> PathBuf {
    #[cfg(test)]
    if let Some(path) = RELEASE_INSTALL_PROGRESS_DIR_OVERRIDE
        .get_or_init(|| std::sync::Mutex::new(None))
        .lock()
        .expect("release install progress dir override should not be poisoned")
        .clone()
    {
        return path;
    }

    trusted_workspace_root()
        .join(".vida")
        .join("data")
        .join("state")
        .join("release-install-progress")
}

fn release_install_progress_latest_path() -> PathBuf {
    release_install_progress_dir().join("latest.json")
}

fn write_release_install_progress_event(
    path: &Path,
    status: &str,
    command: &[String],
    exit_code: Option<i32>,
) -> io::Result<()> {
    write_release_install_progress_event_with_child(
        path, status, "build", command, exit_code, None, None,
    )
}

fn write_release_install_progress_event_with_child(
    path: &Path,
    status: &str,
    phase: &str,
    command: &[String],
    exit_code: Option<i32>,
    process_id: Option<u32>,
    child_state: Option<&str>,
) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        ensure_safe_release_state_dir(parent)?;
    }
    let mut file = open_release_artifact_for_append(path)?;
    let event = serde_json::json!({
        "surface": "vida release install",
        "status": status,
        "phase": phase,
        "command": command,
        "exit_code": exit_code,
        "process_id": process_id,
        "child_state": child_state,
        "progress_path": path.display().to_string(),
        "recorded_at_unix_ms": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default(),
    });
    writeln!(file, "{event}")?;
    let latest_path = release_install_progress_latest_path();
    if let Some(parent) = latest_path.parent() {
        ensure_safe_release_state_dir(parent)?;
    }
    let latest_body = serde_json::to_string_pretty(&event)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    write_release_artifact_atomically(&latest_path, latest_body.as_bytes())?;
    write_release_artifact_atomically(
        &latest_path.with_extension("path"),
        path.display().to_string().as_bytes(),
    )?;
    Ok(())
}

fn ensure_safe_release_state_dir(path: &Path) -> io::Result<()> {
    reject_existing_symlinks_in_path(path)?;
    fs::create_dir_all(path)?;
    reject_existing_symlinks_in_path(path)
}

fn reject_existing_symlinks_in_path(path: &Path) -> io::Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if current.exists() {
            reject_symlink_path(&current)?;
        }
    }
    Ok(())
}

fn reject_symlink_path(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("release artifact path uses a symlink: {}", path.display()),
        ));
    }
    Ok(())
}

fn open_release_artifact_for_append(path: &Path) -> io::Result<fs::File> {
    if path.exists() {
        reject_symlink_path(path)?;
    }
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    apply_no_follow_open_options(&mut options);
    options.open(path)
}

fn write_release_artifact_atomically(path: &Path, body: &[u8]) -> io::Result<()> {
    if path.exists() {
        reject_symlink_path(path)?;
    }
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("release artifact path has no parent: {}", path.display()),
        )
    })?;
    ensure_safe_release_state_dir(parent)?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "release artifact path has invalid file name: {}",
                    path.display()
                ),
            )
        })?;
    let temp_path = parent.join(format!(
        ".{file_name}.{}.tmp",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    let write_result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        apply_no_follow_open_options(&mut options);
        let mut file = options.open(&temp_path)?;
        file.write_all(body)?;
        file.sync_all()?;
        if path.exists() {
            reject_symlink_path(path)?;
        }
        fs::rename(&temp_path, path)
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

#[cfg(unix)]
fn apply_no_follow_open_options(options: &mut OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;

    options.custom_flags(libc::O_NOFOLLOW);
}

#[cfg(windows)]
const WINDOWS_FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

#[cfg(windows)]
fn windows_no_follow_open_flags() -> u32 {
    WINDOWS_FILE_FLAG_OPEN_REPARSE_POINT
}

#[cfg(windows)]
fn apply_no_follow_open_options(options: &mut OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;

    options.custom_flags(windows_no_follow_open_flags());
}

#[cfg(all(not(unix), not(windows)))]
fn apply_no_follow_open_options(_options: &mut OpenOptions) {}

#[derive(Debug)]
struct BlockedRelease {
    blocker_code: &'static str,
    next_action: String,
    io_error: Option<ReleaseIoErrorDetail>,
}

fn blocked_receipt(
    requested_target: String,
    source_binary_path: String,
    build: ReleaseBuildReceipt,
    blocked: BlockedRelease,
) -> ReleaseInstallReceipt {
    let error_kind = blocked
        .io_error
        .as_ref()
        .map(|error| error.error_kind.clone());
    ReleaseInstallReceipt {
        status: "blocked".to_string(),
        blocker_codes: vec![blocked.blocker_code.to_string()],
        next_actions: vec![blocked.next_action],
        build,
        asset_update: ReleaseAssetUpdateReceipt {
            status: "not_started".to_string(),
            refreshed_paths: Vec::new(),
        },
        install_layout: None,
        source_binary_path,
        source_binary_fingerprint: None,
        requested_target,
        installed_targets: Vec::new(),
        io_error: blocked.io_error,
        error_kind,
    }
}

fn default_source_binary_path() -> PathBuf {
    trusted_workspace_root()
        .join("target")
        .join("release")
        .join(vida_binary_file_name())
}

fn default_pi_agent_source_binary_path() -> PathBuf {
    trusted_workspace_root()
        .join("target")
        .join("release")
        .join(pi_agent_binary_file_name())
}

fn release_asset_source_root() -> PathBuf {
    let repo_root = crate::repo_runtime_root();
    if crate::init_surfaces::looks_like_init_bootstrap_source_root(&repo_root) {
        return repo_root;
    }
    crate::init_surfaces::resolve_init_bootstrap_source_root()
}

fn materialize_release_runtime_assets(
    current_root: &Path,
) -> Result<ReleaseAssetUpdateReceipt, ReleaseIoErrorDetail> {
    let source_root = release_asset_source_root();
    let mut refreshed_paths = Vec::new();

    copy_release_tree_replace(
        &source_root.join("vida/config"),
        &current_root.join("vida/config"),
        &mut refreshed_paths,
        "vida/config",
    )?;
    if source_root.join(".codex").is_dir() {
        copy_release_tree_replace(
            &source_root.join(".codex"),
            &current_root.join(".codex"),
            &mut refreshed_paths,
            ".codex",
        )?;
    }
    if source_root.join("install/assets").is_dir() {
        copy_release_tree_replace(
            &source_root.join("install/assets"),
            &current_root.join("install/assets"),
            &mut refreshed_paths,
            "install/assets",
        )?;
    }
    if source_root.join("docs/framework/templates").is_dir() {
        copy_release_tree_replace(
            &source_root.join("docs/framework/templates"),
            &current_root.join("docs/framework/templates"),
            &mut refreshed_paths,
            "docs/framework/templates",
        )?;
    }
    if source_root.join("docs/product/spec/templates").is_dir() {
        copy_release_tree_replace(
            &source_root.join("docs/product/spec/templates"),
            &current_root.join("docs/product/spec/templates"),
            &mut refreshed_paths,
            "docs/product/spec/templates",
        )?;
    }

    copy_release_file_replace(
        &crate::init_surfaces::resolve_init_agents_source(&source_root)
            .map_err(|error| release_asset_error("resolve_agents_source", current_root, error))?,
        &current_root.join("AGENTS.md"),
        &mut refreshed_paths,
        "AGENTS.md",
    )?;
    copy_release_file_replace(
        &crate::init_surfaces::resolve_init_sidecar_source(&source_root)
            .map_err(|error| release_asset_error("resolve_sidecar_source", current_root, error))?,
        &current_root.join("AGENTS.sidecar.md"),
        &mut refreshed_paths,
        "AGENTS.sidecar.md",
    )?;
    let config_template =
        crate::init_surfaces::resolve_init_config_template_source(&source_root)
            .map_err(|error| release_asset_error("resolve_config_template", current_root, error))?;
    copy_release_file_replace(
        &config_template,
        &current_root.join("install/assets/vida.config.yaml.template"),
        &mut refreshed_paths,
        "install/assets/vida.config.yaml.template",
    )?;
    if !current_root.join("vida.config.yaml").exists() {
        copy_release_file_replace(
            &config_template,
            &current_root.join("vida.config.yaml"),
            &mut refreshed_paths,
            "vida.config.yaml",
        )?;
    }
    copy_release_file_replace(
        &crate::init_surfaces::resolve_feature_design_template_source(&source_root).map_err(
            |error| release_asset_error("resolve_feature_template", current_root, error),
        )?,
        &current_root.join("install/assets/feature-design-document.template.md"),
        &mut refreshed_paths,
        "install/assets/feature-design-document.template.md",
    )?;

    refreshed_paths.sort();
    refreshed_paths.dedup();
    Ok(ReleaseAssetUpdateReceipt {
        status: "refreshed".to_string(),
        refreshed_paths,
    })
}

fn copy_release_tree_replace(
    source_root: &Path,
    target_root: &Path,
    refreshed_paths: &mut Vec<String>,
    label: &str,
) -> Result<(), ReleaseIoErrorDetail> {
    crate::init_surfaces::copy_tree_replace(source_root, target_root)
        .map_err(|error| release_asset_error("copy_tree", target_root, error))?;
    refreshed_paths.push(label.to_string());
    Ok(())
}

fn copy_release_file_replace(
    source: &Path,
    target: &Path,
    refreshed_paths: &mut Vec<String>,
    label: &str,
) -> Result<(), ReleaseIoErrorDetail> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| io_error_detail("create_dir", Some(parent), None, &error))?;
    }
    fs::copy(source, target)
        .map_err(|error| io_error_detail("copy", Some(target), None, &error))?;
    refreshed_paths.push(label.to_string());
    Ok(())
}

fn release_asset_error(
    operation: &'static str,
    target_path: &Path,
    error: String,
) -> ReleaseIoErrorDetail {
    synthetic_io_error_detail(
        operation,
        Some(target_path),
        None,
        &format!("{error}. Refresh release runtime assets from the source repository."),
    )
}

fn trusted_workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("vida crate should be nested under workspace root")
        .to_path_buf()
}

fn install_target_paths(
    requested_target: &str,
    install_root: Option<&Path>,
) -> Result<Vec<(String, PathBuf)>, BlockedRelease> {
    install_target_paths_with_path_env(requested_target, install_root, std::env::var_os("PATH"))
}

fn install_target_paths_with_path_env(
    requested_target: &str,
    install_root: Option<&Path>,
    path_env: Option<OsString>,
) -> Result<Vec<(String, PathBuf)>, BlockedRelease> {
    let root = release_install_root(install_root);
    let binary_name = vida_binary_file_name();
    match requested_target {
        "current" | "cur" | "all" | "local" | "cargo" | "path" => {
            let root = root.ok_or_else(unresolved_install_target)?;
            let current_path = root.join("current").join("bin").join(binary_name);
            if install_root.is_none() {
                fail_if_default_current_target_is_not_active_path(&current_path, path_env)?;
            }
            Ok(vec![("current".to_string(), current_path)])
        }
        _ => Err(BlockedRelease {
            blocker_code: "unsupported_install_target",
            next_action: "Use `--target current` or `--target cur`.".to_string(),
            io_error: None,
        }),
    }
}

fn fail_if_default_current_target_is_not_active_path(
    current_path: &Path,
    path_env: Option<OsString>,
) -> Result<(), BlockedRelease> {
    let Some(active_path) = resolve_vida_from_path_env(path_env) else {
        return Ok(());
    };
    if release_paths_match(&active_path, current_path) {
        return Ok(());
    }
    Err(BlockedRelease {
        blocker_code: "release_install_active_path_mismatch",
        next_action: format!(
            "Default `--target current` would install `{}` but the first `vida` on PATH is `{}`; put current/bin first on PATH and remove the non-canonical `vida` binary.",
            current_path.display(),
            active_path.display()
        ),
        io_error: None,
    })
}

fn release_paths_match(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => release_path_key(left) == release_path_key(right),
    }
}

fn release_path_key(path: &Path) -> String {
    let key = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key.to_ascii_lowercase()
    } else {
        key
    }
}

fn companion_runtime_install_target_paths(
    requested_target: &str,
    install_root: Option<&Path>,
) -> Result<Vec<(String, PathBuf)>, BlockedRelease> {
    match requested_target {
        "current" | "cur" | "all" | "local" | "cargo" | "path" => {
            let root = release_install_root(install_root).ok_or_else(unresolved_install_target)?;
            Ok(vec![(
                "current:vida-pi-agent".to_string(),
                root.join("current")
                    .join("bin")
                    .join(pi_agent_binary_file_name()),
            )])
        }
        _ => Ok(Vec::new()),
    }
}

pub(crate) fn release_install_layout(install_root: Option<&Path>) -> Option<ReleaseInstallLayout> {
    let root = release_install_root(install_root)?;
    let current_root = root.join("current");
    let runtime_bin_dir = current_root.join("bin");
    Some(ReleaseInstallLayout {
        env_file: root.join(release_env_file_name()).display().to_string(),
        install_root: root.display().to_string(),
        current_root: current_root.display().to_string(),
        runtime_bin_dir: runtime_bin_dir.display().to_string(),
        platform: std::env::consts::OS.to_string(),
    })
}

pub(crate) fn release_install_root(install_root: Option<&Path>) -> Option<PathBuf> {
    install_root
        .map(Path::to_path_buf)
        .or_else(default_release_install_root)
}

pub(crate) fn default_release_install_root() -> Option<PathBuf> {
    default_release_install_root_from_values(
        std::env::var_os("VIDA_HOME"),
        std::env::var_os("LOCALAPPDATA"),
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
        std::env::var_os("HOMEDRIVE"),
        std::env::var_os("HOMEPATH"),
    )
}

fn default_release_install_root_from_values(
    vida_home: Option<OsString>,
    local_app_data: Option<OsString>,
    home: Option<OsString>,
    userprofile: Option<OsString>,
    homedrive: Option<OsString>,
    homepath: Option<OsString>,
) -> Option<PathBuf> {
    if let Some(vida_home) = non_empty_env_path_value(vida_home) {
        return Some(PathBuf::from(vida_home));
    }
    #[cfg(windows)]
    {
        if let Some(local_app_data) = non_empty_env_path_value(local_app_data) {
            return Some(PathBuf::from(local_app_data).join("vida-stack"));
        }
    }
    user_home_dir_from_values(home, userprofile, homedrive, homepath).map(|home| {
        if cfg!(windows) {
            home.join("AppData").join("Local").join("vida-stack")
        } else {
            home.join(".local").join("share").join("vida-stack")
        }
    })
}

fn release_env_file_name() -> &'static str {
    if cfg!(windows) {
        "env.ps1"
    } else {
        "env.sh"
    }
}

fn user_home_dir() -> Option<PathBuf> {
    user_home_dir_from_values(
        std::env::var_os("HOME"),
        std::env::var_os("USERPROFILE"),
        std::env::var_os("HOMEDRIVE"),
        std::env::var_os("HOMEPATH"),
    )
}

fn user_home_dir_from_values(
    home: Option<OsString>,
    userprofile: Option<OsString>,
    homedrive: Option<OsString>,
    homepath: Option<OsString>,
) -> Option<PathBuf> {
    non_empty_env_path_value(home)
        .or_else(|| non_empty_env_path_value(userprofile))
        .or_else(|| {
            let drive = non_empty_env_path_value(homedrive)?;
            let path = non_empty_env_path_value(homepath)?;
            let mut combined = std::ffi::OsString::from(drive);
            combined.push(path);
            Some(combined)
        })
        .map(PathBuf::from)
}

fn non_empty_env_path_value(value: Option<OsString>) -> Option<OsString> {
    let value = non_empty_os_string(value)?;
    if has_unexpanded_windows_env_placeholder(&value) {
        None
    } else {
        Some(value)
    }
}

fn non_empty_os_string(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

fn has_unexpanded_windows_env_placeholder(value: &std::ffi::OsStr) -> bool {
    if !cfg!(windows) {
        return false;
    }
    let text = value.to_string_lossy();
    let mut remainder = text.as_ref();
    while let Some(start) = remainder.find('%') {
        let after_start = &remainder[start + 1..];
        if let Some(end) = after_start.find('%') {
            if end > 0 {
                return true;
            }
            remainder = &after_start[end + 1..];
        } else {
            return false;
        }
    }
    false
}

fn unresolved_install_target() -> BlockedRelease {
    BlockedRelease {
        blocker_code: "install_target_unresolved",
        next_action: "Set HOME/USERPROFILE or pass `--install-root <path>`.".to_string(),
        io_error: None,
    }
}

pub(crate) fn vida_binary_file_name() -> String {
    format!("vida{}", std::env::consts::EXE_SUFFIX)
}

fn pi_agent_binary_file_name() -> String {
    format!("vida-pi-agent{}", std::env::consts::EXE_SUFFIX)
}

fn vida_path_candidate_names() -> Vec<String> {
    let canonical = vida_binary_file_name();
    if canonical == "vida" {
        vec![canonical]
    } else {
        vec![canonical, "vida".to_string()]
    }
}

fn path_candidate_is_safe_file(candidate: &Path) -> bool {
    fs::symlink_metadata(candidate)
        .map(|metadata| metadata.file_type().is_file())
        .unwrap_or(false)
}

fn resolve_vida_from_path_env(path_env: Option<std::ffi::OsString>) -> Option<PathBuf> {
    let path_env = path_env?;
    for dir in std::env::split_paths(&path_env) {
        for file_name in vida_path_candidate_names() {
            let candidate = dir.join(file_name);
            if path_candidate_is_safe_file(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

fn binary_fingerprint(path: &Path) -> Result<String, ReleaseIoErrorDetail> {
    let bytes = fs::read(path)
        .map_err(|error| io_error_detail("read_fingerprint", Some(path), None, &error))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn current_release_installed_target_identity() -> Option<ReleaseInstalledTarget> {
    let layout = release_install_layout(None)?;
    let path = PathBuf::from(layout.runtime_bin_dir).join(vida_binary_file_name());
    if !path.is_file() {
        return None;
    }
    let fingerprint = binary_fingerprint(&path).ok()?;
    Some(ReleaseInstalledTarget {
        target: "current".to_string(),
        path: path.display().to_string(),
        fingerprint,
        version: None,
    })
}

fn write_binary_fingerprint_metadata(
    path: &Path,
    fingerprint: &str,
) -> Result<(), ReleaseIoErrorDetail> {
    let metadata = fs::metadata(path)
        .map_err(|error| io_error_detail("metadata", Some(path), None, &error))?;
    let modified_unix_ms = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let payload = serde_json::json!({
        "schema_version": "vida-binary-fingerprint-v1",
        "path": path.display().to_string(),
        "len": metadata.len(),
        "modified_unix_ms": modified_unix_ms,
        "fingerprint": fingerprint,
    });
    let body = serde_json::to_string_pretty(&payload).map_err(|error| {
        synthetic_io_error_detail(
            "serialize_fingerprint_metadata",
            Some(path),
            None,
            &error.to_string(),
        )
    })?;
    fs::write(binary_fingerprint_metadata_path(path), body)
        .map_err(|error| io_error_detail("write_fingerprint_metadata", Some(path), None, &error))
}

fn binary_fingerprint_metadata_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.fingerprint.json",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("binary")
    ))
}

fn install_binary(source: &Path, destination: &Path) -> Result<(), ReleaseIoErrorDetail> {
    let parent = destination.parent().ok_or_else(|| {
        synthetic_io_error_detail(
            "resolve_parent",
            Some(destination),
            None,
            "install destination has no parent directory",
        )
    })?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            synthetic_io_error_detail(
                "resolve_file_name",
                Some(destination),
                None,
                "install destination has no file name",
            )
        })?;
    for attempt in 0..INSTALL_BINARY_RETRY_LIMIT {
        let staging_path = release_install_staging_path(parent, file_name, attempt);
        if attempt > 0 {
            thread::sleep(install_binary_retry_delay(attempt));
            let _ = fs::remove_file(&staging_path);
        }

        match install_binary_once(source, destination, &staging_path) {
            Ok(()) => return Ok(()),
            Err((error, operation)) => {
                if is_text_file_busy_error(&error) && attempt + 1 < INSTALL_BINARY_RETRY_LIMIT {
                    continue;
                }
                let detail =
                    io_error_detail(operation, Some(destination), Some(&staging_path), &error);
                let _ = fs::remove_file(&staging_path);
                return Err(detail);
            }
        }
    }

    unreachable!("install_binary retry loop must return or continue")
}

fn install_binary_once(
    source: &Path,
    destination: &Path,
    staging_path: &Path,
) -> Result<(), (io::Error, &'static str)> {
    fs::copy(source, staging_path).map_err(|error| (error, "copy"))?;
    let permissions = fs::metadata(source)
        .map_err(|error| (error, "read_source_metadata"))?
        .permissions();
    fs::set_permissions(staging_path, permissions).map_err(|error| (error, "set_permissions"))?;
    replace_destination_binary(staging_path, destination)?;
    Ok(())
}

#[cfg(not(windows))]
fn replace_destination_binary(
    staging_path: &Path,
    destination: &Path,
) -> Result<(), (io::Error, &'static str)> {
    fs::rename(staging_path, destination).map_err(|error| (error, "rename"))
}

#[cfg(windows)]
fn replace_destination_binary(
    staging_path: &Path,
    destination: &Path,
) -> Result<(), (io::Error, &'static str)> {
    let backup_path = release_install_backup_path(staging_path);
    let _ = fs::remove_file(&backup_path);
    if destination.exists() {
        fs::rename(destination, &backup_path)
            .map_err(|error| (error, "rename_existing_destination"))?;
    }
    match fs::rename(staging_path, destination) {
        Ok(()) => {
            let _ = fs::remove_file(&backup_path);
            Ok(())
        }
        Err(error) => {
            if backup_path.exists() {
                let _ = fs::rename(&backup_path, destination);
            }
            Err((error, "rename"))
        }
    }
}

fn install_binary_retry_delay(attempt: usize) -> Duration {
    Duration::from_millis(match attempt {
        1 => 25,
        2 => 50,
        3 => 100,
        4 => 150,
        _ => 200,
    })
}

fn release_install_staging_path(parent: &Path, file_name: &str, attempt: usize) -> PathBuf {
    parent.join(format!(
        ".{file_name}.installing.{}.{}",
        process::id(),
        attempt + 1
    ))
}

#[cfg(windows)]
fn release_install_backup_path(staging_path: &Path) -> PathBuf {
    let file_name = staging_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.replace(".installing.", ".replaced."))
        .unwrap_or_else(|| format!(".vida.replaced.{}", process::id()));
    staging_path.with_file_name(file_name)
}

fn is_text_file_busy_error(error: &io::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    error.raw_os_error() == Some(26)
        || message.contains("text file busy")
        || message.contains("text file is busy")
}

fn io_error_detail(
    operation: &str,
    target_path: Option<&Path>,
    staging_path: Option<&Path>,
    error: &io::Error,
) -> ReleaseIoErrorDetail {
    ReleaseIoErrorDetail {
        operation: operation.to_string(),
        target_path: target_path.map(|path| path.display().to_string()),
        staging_path: staging_path.map(|path| path.display().to_string()),
        error_kind: release_install_error_kind(error),
        error_message: error.to_string(),
        next_action_hint: next_action_for_io_error(error).to_string(),
    }
}

fn synthetic_io_error_detail(
    operation: &str,
    target_path: Option<&Path>,
    staging_path: Option<&Path>,
    message: &str,
) -> ReleaseIoErrorDetail {
    ReleaseIoErrorDetail {
        operation: operation.to_string(),
        target_path: target_path.map(|path| path.display().to_string()),
        staging_path: staging_path.map(|path| path.display().to_string()),
        error_kind: "InvalidInput".to_string(),
        error_message: message.to_string(),
        next_action_hint: "Choose a valid release install destination path.".to_string(),
    }
}

fn next_action_for_io_error(error: &io::Error) -> &'static str {
    let message = error.to_string().to_ascii_lowercase();
    if is_text_file_busy_error(error) {
        "The destination binary is in use (`text file is busy`). Stop the running process and rerun `vida release install --json`."
    } else if error.kind() == io::ErrorKind::PermissionDenied {
        "Check install target permissions, choose a writable `--install-root`, or rerun with an explicitly approved install path."
    } else if error.raw_os_error() == Some(30) || message.contains("read-only file system") {
        "The install target is on a read-only filesystem or blocked by sandboxing; choose a writable `--install-root` such as `/tmp/...` or rerun with explicit filesystem approval."
    } else {
        "Inspect the structured IO error detail, choose a writable install target, and rerun the install command."
    }
}

fn release_install_error_blocker_code(error_kind: &str) -> &'static str {
    match error_kind {
        "text_file_busy" => "install_target_text_file_busy",
        "install_target_permission_denied" => "install_target_permission_denied",
        "install_target_read_only_or_sandbox_blocked" => {
            "install_target_read_only_or_sandbox_blocked"
        }
        _ => "install_target_write_failed",
    }
}

fn release_install_error_kind(error: &io::Error) -> String {
    if is_text_file_busy_error(error) {
        "text_file_busy".to_string()
    } else if is_read_only_or_sandbox_error(error) {
        "install_target_read_only_or_sandbox_blocked".to_string()
    } else if error.kind() == io::ErrorKind::PermissionDenied {
        "install_target_permission_denied".to_string()
    } else {
        format!("{:?}", error.kind()).to_ascii_lowercase()
    }
}

fn is_read_only_or_sandbox_error(error: &io::Error) -> bool {
    let message = error.to_string().to_ascii_lowercase();
    error.raw_os_error() == Some(30)
        || message.contains("read-only file system")
        || message.contains("operation not permitted")
        || message.contains("sandbox")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::temp_state::TempStateHarness;
    use clap::Parser;
    use std::sync::{Mutex, OnceLock};

    fn release_progress_test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("release progress test lock should not be poisoned")
    }

    struct ReleaseProgressDirOverrideGuard;

    impl Drop for ReleaseProgressDirOverrideGuard {
        fn drop(&mut self) {
            set_release_install_progress_dir_override(None);
        }
    }

    fn release_progress_dir_override(path: PathBuf) -> ReleaseProgressDirOverrideGuard {
        set_release_install_progress_dir_override(Some(path));
        ReleaseProgressDirOverrideGuard
    }

    fn set_release_install_progress_dir_override(path: Option<PathBuf>) {
        let mut override_path = RELEASE_INSTALL_PROGRESS_DIR_OVERRIDE
            .get_or_init(|| Mutex::new(None))
            .lock()
            .expect("release progress dir override should not be poisoned");
        *override_path = path;
    }

    fn clean_release_progress_latest_markers() {
        let latest_path = release_install_progress_latest_path();
        let _ = fs::remove_file(&latest_path);
        let _ = fs::remove_file(latest_path.with_extension("path"));
    }

    #[test]
    fn release_install_help_exposes_options() {
        let error = Cli::try_parse_from(["vida", "release", "install", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("--json"));
        assert!(help.contains("--skip-build"));
        assert!(help.contains("--status"));
        assert!(help.contains("--target"));
        assert!(help.contains("Install target: current or cur."));
        assert!(help.contains("Legacy all/local/cargo/path aliases resolve to current."));
        assert!(help.contains("--source-binary"));
        assert!(help.contains("--install-root"));
    }

    #[test]
    fn release_install_default_source_binary_uses_platform_executable_suffix() {
        assert_eq!(
            default_source_binary_path(),
            trusted_workspace_root()
                .join("target")
                .join("release")
                .join(vida_binary_file_name())
        );
    }

    #[test]
    fn release_install_explicit_root_uses_platform_executable_suffix() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let paths = install_target_paths("current", Some(harness.path()))
            .expect("current install target should resolve");

        assert_eq!(
            paths,
            vec![(
                "current".to_string(),
                harness
                    .path()
                    .join("current")
                    .join("bin")
                    .join(vida_binary_file_name())
            )]
        );
    }

    #[test]
    fn release_install_cur_alias_resolves_to_current_target() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let paths = install_target_paths("cur", Some(harness.path()))
            .expect("cur install target should resolve");

        assert_eq!(
            paths,
            vec![(
                "current".to_string(),
                harness
                    .path()
                    .join("current")
                    .join("bin")
                    .join(vida_binary_file_name())
            )]
        );
    }

    #[test]
    fn release_install_default_current_blocks_when_active_path_differs() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let active_dir = harness.path().join("active-bin");
        fs::create_dir_all(&active_dir).expect("active PATH bin should exist");
        let active_vida = active_dir.join(vida_binary_file_name());
        fs::write(&active_vida, b"old active vida").expect("active vida should write");
        let path_env = std::env::join_paths([active_dir]).expect("PATH should join");

        let blocked = install_target_paths_with_path_env("current", None, Some(path_env))
            .expect_err("default current target should block when PATH uses another vida");

        assert_eq!(blocked.blocker_code, "release_install_active_path_mismatch");
        assert!(blocked.next_action.contains("remove the non-canonical"));
        assert!(blocked.next_action.contains("current/bin"));
        assert!(blocked
            .next_action
            .contains(&active_vida.display().to_string()));
    }

    #[test]
    fn release_install_current_target_includes_pi_agent_companion_destination() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let paths = companion_runtime_install_target_paths("current", Some(harness.path()))
            .expect("current companion install target should resolve");

        assert_eq!(
            paths,
            vec![(
                "current:vida-pi-agent".to_string(),
                harness
                    .path()
                    .join("current")
                    .join("bin")
                    .join(pi_agent_binary_file_name())
            )]
        );
    }

    #[test]
    fn release_install_cur_alias_includes_pi_agent_companion_destination() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let paths = companion_runtime_install_target_paths("cur", Some(harness.path()))
            .expect("cur companion install target should resolve");

        assert_eq!(
            paths,
            vec![(
                "current:vida-pi-agent".to_string(),
                harness
                    .path()
                    .join("current")
                    .join("bin")
                    .join(pi_agent_binary_file_name())
            )]
        );
    }

    #[test]
    fn release_install_path_target_aliases_pi_agent_companion_to_current_destination() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let root = harness.path().join("install-root");
        let paths = companion_runtime_install_target_paths("path", Some(&root))
            .expect("legacy path companion target should resolve to canonical current target");

        assert_eq!(
            paths,
            vec![(
                "current:vida-pi-agent".to_string(),
                root.join("current")
                    .join("bin")
                    .join(pi_agent_binary_file_name())
            )]
        );
    }

    #[test]
    fn release_install_build_command_includes_pi_agent_companion_package() {
        assert_eq!(
            release_build_command(),
            vec![
                "cargo".to_string(),
                "build".to_string(),
                "-p".to_string(),
                "vida".to_string(),
                "-p".to_string(),
                "vida-pi-agent".to_string(),
                "--release".to_string(),
            ]
        );
    }

    #[test]
    fn release_install_progress_event_writes_durable_jsonl_artifact() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");
        let command = release_build_command();

        write_release_install_progress_event(&progress_path, "started", &command, None)
            .expect("progress start should write");
        write_release_install_progress_event(&progress_path, "pass", &command, Some(0))
            .expect("progress pass should write");

        let body = fs::read_to_string(&progress_path).expect("progress artifact should read");
        let lines = body.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let started: serde_json::Value =
            serde_json::from_str(lines[0]).expect("started event should parse");
        let passed: serde_json::Value =
            serde_json::from_str(lines[1]).expect("pass event should parse");
        assert_eq!(started["surface"], "vida release install");
        assert_eq!(started["status"], "started");
        assert_eq!(started["command"][0], "cargo");
        assert_eq!(passed["status"], "pass");
        assert_eq!(started["phase"], "build");
        assert_eq!(started["process_id"], serde_json::Value::Null);
        assert_eq!(started["child_state"], serde_json::Value::Null);
        assert_eq!(passed["exit_code"], 0);
        assert_eq!(passed["progress_path"], progress_path.display().to_string());
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_status_reports_running_started_child() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");

        write_release_install_progress_event_with_child(
            &progress_path,
            "started",
            "build",
            &release_build_command(),
            None,
            Some(std::process::id()),
            Some("running"),
        )
        .expect("started progress should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "running");
        assert_eq!(receipt.blocker_codes, Vec::<String>::new());
        assert_eq!(receipt.latest_status.as_deref(), Some("started"));
        assert_eq!(receipt.process_id, Some(std::process::id()));
        assert_eq!(receipt.child_state.as_deref(), Some("alive"));
        assert_eq!(
            receipt.progress_path.as_deref(),
            Some(progress_path.display().to_string().as_str())
        );
        assert!(receipt
            .next_actions
            .iter()
            .any(|action| action.contains("do not start another release install")));
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_status_treats_started_without_pid_as_running() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");

        write_release_install_progress_event_with_child(
            &progress_path,
            "started",
            "build",
            &release_build_command(),
            None,
            None,
            Some("starting"),
        )
        .expect("started progress should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "running");
        assert_eq!(receipt.blocker_codes, Vec::<String>::new());
        assert_eq!(receipt.latest_status.as_deref(), Some("started"));
        assert_eq!(receipt.process_id, None);
        assert_eq!(receipt.child_state.as_deref(), Some("starting"));
        assert!(receipt
            .next_actions
            .iter()
            .any(|action| action.contains("do not start another release install")));
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_status_contract_blocks_dead_started_child() {
        let (status, blocker_codes, next_actions) =
            release_install_progress_status_contract(Some("started"), Some("build"), Some("dead"));

        assert_eq!(status, "blocked");
        assert_eq!(
            blocker_codes,
            vec!["release_install_progress_stale_started".to_string()]
        );
        assert!(next_actions
            .iter()
            .any(|action| action.contains("recorded build process is not alive")));
    }

    #[test]
    fn release_install_status_blocks_build_pass_without_install_phase() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");

        write_release_install_progress_event_with_child(
            &progress_path,
            "pass",
            "build",
            &release_build_command(),
            Some(0),
            Some(std::process::id()),
            Some("completed"),
        )
        .expect("build pass progress should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.latest_status.as_deref(), Some("pass"));
        assert_eq!(receipt.latest_phase.as_deref(), Some("build"));
        assert_eq!(
            receipt.blocker_codes,
            vec!["release_install_progress_missing_install_phase".to_string()]
        );
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_status_recovers_missing_latest_marker_from_durable_progress() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");

        write_release_install_progress_event_with_child(
            &progress_path,
            "pass",
            "build",
            &release_build_command(),
            Some(0),
            Some(std::process::id()),
            Some("completed"),
        )
        .expect("build pass progress should write");
        write_release_install_progress_event_with_child(
            &progress_path,
            "pass",
            "install",
            &[
                "vida".to_string(),
                "release".to_string(),
                "install".to_string(),
            ],
            Some(0),
            None,
            Some("completed"),
        )
        .expect("install pass progress should write");
        fs::remove_file(release_install_progress_latest_path())
            .expect("latest marker should be removed for recovery test");
        let _ = fs::remove_file(release_install_progress_latest_path().with_extension("path"));

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.blocker_codes, Vec::<String>::new());
        assert_eq!(receipt.latest_status.as_deref(), Some("pass"));
        assert_eq!(receipt.latest_phase.as_deref(), Some("install"));
        assert_eq!(
            receipt.progress_path.as_deref(),
            Some(progress_path.display().to_string().as_str())
        );
        assert_eq!(
            receipt.artifact_refs,
            vec![progress_path.display().to_string()]
        );
    }

    #[test]
    fn release_install_status_uses_newest_durable_progress_event_timestamp() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_dir = release_install_progress_dir();
        fs::create_dir_all(&progress_dir).expect("progress dir should write");
        let older_path = progress_dir.join("release-install-older.jsonl");
        let newer_path = progress_dir.join("release-install-newer.jsonl");
        let older = serde_json::json!({
            "surface": "vida release install",
            "status": "pass",
            "phase": "build",
            "command": release_build_command(),
            "exit_code": 0,
            "process_id": null,
            "child_state": "completed",
            "progress_path": older_path.display().to_string(),
            "recorded_at_unix_ms": 1_u64,
        });
        let newer = serde_json::json!({
            "surface": "vida release install",
            "status": "pass",
            "phase": "install",
            "command": ["vida", "release", "install"],
            "exit_code": 0,
            "process_id": null,
            "child_state": "completed",
            "progress_path": newer_path.display().to_string(),
            "recorded_at_unix_ms": 2_u64,
        });
        fs::write(&older_path, format!("{older}\n")).expect("older progress should write");
        fs::write(&newer_path, format!("{newer}\n")).expect("newer progress should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.latest_phase.as_deref(), Some("install"));
        assert_eq!(
            receipt.progress_path.as_deref(),
            Some(newer_path.display().to_string().as_str())
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_install_status_ignores_symlinked_durable_progress_artifact() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_dir = release_install_progress_dir();
        fs::create_dir_all(&progress_dir).expect("progress dir should write");
        let victim = harness.path().join("victim-secret.jsonl");
        let symlink_progress = progress_dir.join("release-install-symlink.jsonl");
        fs::write(
            &victim,
            format!(
                "{}\n",
                serde_json::json!({
                    "surface": "vida release install",
                    "status": "pass",
                    "phase": "install",
                    "child_state": "completed",
                    "progress_path": symlink_progress.display().to_string(),
                    "recorded_at_unix_ms": 99_u64,
                    "attacker_secret": "SENSITIVE_TOKEN_ABC123",
                })
            ),
        )
        .expect("victim secret should write");
        std::os::unix::fs::symlink(&victim, &symlink_progress)
            .expect("symlinked progress artifact should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["release_install_progress_missing".to_string()]
        );
        assert!(receipt.latest_event.is_none());
        assert!(receipt.progress_path.is_none());
        assert!(receipt.artifact_refs.is_empty());
    }

    #[test]
    fn release_install_status_reads_regular_latest_path_marker_during_durable_fallback() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_dir = release_install_progress_dir();
        fs::create_dir_all(&progress_dir).expect("progress dir should write");
        let fallback_progress = progress_dir.join("release-install-fallback.jsonl");
        fs::write(
            &fallback_progress,
            format!(
                "{}\n",
                serde_json::json!({
                    "surface": "vida release install",
                    "status": "pass",
                    "phase": "install",
                    "child_state": "completed",
                    "recorded_at_unix_ms": 100_u64,
                })
            ),
        )
        .expect("fallback progress should write");
        let recorded_path = progress_dir.join("recorded-progress.jsonl");
        fs::write(
            release_install_progress_latest_path().with_extension("path"),
            recorded_path.display().to_string(),
        )
        .expect("latest.path marker should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.latest_phase.as_deref(), Some("install"));
        assert_eq!(
            receipt.progress_path.as_deref(),
            Some(recorded_path.display().to_string().as_str())
        );
        assert_eq!(
            receipt.artifact_refs,
            vec![recorded_path.display().to_string()]
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_install_status_ignores_symlinked_latest_path_marker_during_durable_fallback() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_dir = release_install_progress_dir();
        fs::create_dir_all(&progress_dir).expect("progress dir should write");
        let fallback_progress = progress_dir.join("release-install-fallback.jsonl");
        fs::write(
            &fallback_progress,
            format!(
                "{}\n",
                serde_json::json!({
                    "surface": "vida release install",
                    "status": "pass",
                    "phase": "install",
                    "child_state": "completed",
                    "recorded_at_unix_ms": 100_u64,
                })
            ),
        )
        .expect("fallback progress should write");
        let victim = harness.path().join("victim-latest-path-secret.txt");
        fs::write(&victim, "SENSITIVE_TOKEN_ABC123")
            .expect("victim latest.path secret should write");
        std::os::unix::fs::symlink(
            &victim,
            release_install_progress_latest_path().with_extension("path"),
        )
        .expect("latest.path symlink should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.latest_phase.as_deref(), Some("install"));
        assert_eq!(
            receipt.progress_path.as_deref(),
            Some(fallback_progress.display().to_string().as_str())
        );
        assert_eq!(
            receipt.artifact_refs,
            vec![fallback_progress.display().to_string()]
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_install_status_rejects_symlinked_latest_marker() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");
        write_release_install_progress_event_with_child(
            &progress_path,
            "pass",
            "install",
            &[
                "vida".to_string(),
                "release".to_string(),
                "install".to_string(),
            ],
            Some(0),
            None,
            Some("completed"),
        )
        .expect("durable install progress should write");
        let latest_path = release_install_progress_latest_path();
        let victim = harness.path().join("victim-latest-secret.json");
        fs::write(
            &victim,
            serde_json::json!({
                "surface": "vida release install",
                "status": "pass",
                "phase": "install",
                "child_state": "completed",
                "recorded_at_unix_ms": 100_u64,
                "attacker_secret": "SENSITIVE_TOKEN_ABC123",
            })
            .to_string(),
        )
        .expect("victim latest secret should write");
        fs::remove_file(&latest_path).expect("regular latest marker should be removed");
        std::os::unix::fs::symlink(&victim, &latest_path)
            .expect("latest marker symlink should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["release_install_progress_unreadable".to_string()]
        );
        assert!(receipt
            .next_actions
            .iter()
            .any(|action| action.contains("symlink")));
        assert!(receipt.latest_event.is_none());
        assert!(receipt.progress_path.is_none());
        assert_eq!(
            receipt.artifact_refs,
            vec![latest_path.display().to_string()]
        );
        clean_release_progress_latest_markers();
    }

    #[cfg(unix)]
    #[test]
    fn release_install_status_rejects_symlinked_progress_directory_before_reads() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let attacker_progress_dir = harness.path().join("attacker-progress");
        let progress_link = harness.path().join("progress-link");
        fs::create_dir_all(&attacker_progress_dir).expect("attacker progress dir should write");
        std::os::unix::fs::symlink(&attacker_progress_dir, &progress_link)
            .expect("progress dir symlink should write");
        let _progress_dir = release_progress_dir_override(progress_link);
        let latest_path = release_install_progress_latest_path();
        fs::write(
            attacker_progress_dir.join("latest.json"),
            serde_json::json!({
                "surface": "vida release install",
                "status": "pass",
                "phase": "install",
                "child_state": "completed",
                "recorded_at_unix_ms": 100_u64,
                "attacker_secret": "SENSITIVE_TOKEN_ABC123",
            })
            .to_string(),
        )
        .expect("attacker latest marker should write");
        fs::write(
            attacker_progress_dir.join("release-install-fallback.jsonl"),
            format!(
                "{}\n",
                serde_json::json!({
                    "surface": "vida release install",
                    "status": "pass",
                    "phase": "install",
                    "child_state": "completed",
                    "recorded_at_unix_ms": 101_u64,
                    "attacker_secret": "SENSITIVE_TOKEN_DEF456",
                })
            ),
        )
        .expect("attacker fallback progress should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["release_install_progress_unreadable".to_string()]
        );
        assert!(receipt
            .next_actions
            .iter()
            .any(|action| action.contains("symlink")));
        assert!(receipt.latest_event.is_none());
        assert!(receipt.progress_path.is_none());
        assert_eq!(
            receipt.artifact_refs,
            vec![latest_path.display().to_string()]
        );
    }

    #[test]
    fn release_install_status_rejects_invalid_latest_marker_before_fallback() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");
        write_release_install_progress_event_with_child(
            &progress_path,
            "pass",
            "install",
            &[
                "vida".to_string(),
                "release".to_string(),
                "install".to_string(),
            ],
            Some(0),
            None,
            Some("completed"),
        )
        .expect("install pass progress should write");
        fs::write(release_install_progress_latest_path(), "{invalid json")
            .expect("invalid latest marker should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["release_install_progress_invalid".to_string()]
        );
        assert!(receipt.latest_event.is_none());
    }

    #[test]
    fn release_install_receipt_blocks_duplicate_running_build() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let progress_path = release_install_progress_path()
            .expect("release progress path should be available for test");

        write_release_install_progress_event_with_child(
            &progress_path,
            "started",
            "build",
            &release_build_command(),
            None,
            None,
            Some("starting"),
        )
        .expect("started progress should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: false,
            status: false,
            source_binary: Some(harness.path().join("missing-source-should-not-be-read")),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["release_install_build_already_running".to_string()]
        );
        assert_eq!(receipt.build.status, "blocked");
        assert_eq!(receipt.build.child_state.as_deref(), Some("starting"));
        assert_eq!(
            receipt.build.progress_path.as_deref(),
            Some(progress_path.display().to_string().as_str())
        );
        assert!(receipt
            .next_actions
            .iter()
            .any(|action| action.contains("release install --status")));
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_status_blocks_missing_progress() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "blocked");
        assert_eq!(
            receipt.blocker_codes,
            vec!["release_install_progress_missing".to_string()]
        );
        assert!(receipt.latest_event.is_none());
        assert!(receipt.progress_path.is_none());
    }

    #[test]
    fn release_install_status_surface_does_not_start_build_or_install() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();

        let exit = run_release_install(ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: false,
            status: true,
            source_binary: Some(harness.path().join("missing-source-should-not-be-read")),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(exit, ExitCode::from(1));
        assert!(
            !release_install_progress_latest_path().exists(),
            "--status must not create a release install progress marker"
        );
        assert!(
            !harness.path().join("install-root").exists(),
            "--status must not materialize an install target"
        );
        clean_release_progress_latest_markers();
    }

    #[cfg(unix)]
    #[test]
    fn release_install_status_rejects_symlink_latest_path_sidecar() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let latest_path = release_install_progress_latest_path();
        let latest_sidecar_path = latest_path.with_extension("path");
        let victim = harness.path().join("secret.txt");
        if let Some(parent) = latest_path.parent() {
            fs::create_dir_all(parent).expect("latest parent should write");
        }
        fs::write(
            &latest_path,
            serde_json::json!({
                "surface": "vida release install",
                "status": "pass",
                "phase": "install",
                "command": release_build_command(),
                "exit_code": 0,
                "process_id": null,
                "child_state": null,
            })
            .to_string(),
        )
        .expect("latest marker should write");
        fs::write(&victim, "API_TOKEN=do-not-print").expect("victim should write");
        std::os::unix::fs::symlink(&victim, &latest_sidecar_path)
            .expect("latest path sidecar symlink should write");

        let receipt = release_install_status_receipt();

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.progress_path, None);
        assert_eq!(
            receipt.artifact_refs,
            vec![latest_path.display().to_string()]
        );
        let receipt_body =
            serde_json::to_string(&receipt).expect("status receipt should serialize");
        assert!(
            !receipt_body.contains("API_TOKEN=do-not-print"),
            "status receipt must not include symlink target contents"
        );
        clean_release_progress_latest_markers();
    }

    #[cfg(windows)]
    #[test]
    fn release_install_status_rejects_windows_symlink_latest_path_sidecar_when_available() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let latest_path = release_install_progress_latest_path();
        let latest_sidecar_path = latest_path.with_extension("path");
        let victim = harness.path().join("secret.txt");
        if let Some(parent) = latest_path.parent() {
            fs::create_dir_all(parent).expect("latest parent should write");
        }
        fs::write(
            &latest_path,
            serde_json::json!({
                "surface": "vida release install",
                "status": "pass",
                "phase": "install",
                "command": release_build_command(),
                "exit_code": 0,
                "process_id": null,
                "child_state": null,
            })
            .to_string(),
        )
        .expect("latest marker should write");
        fs::write(&victim, "API_TOKEN=[REDACTED:API key param]").expect("victim should write");
        match std::os::windows::fs::symlink_file(&victim, &latest_sidecar_path) {
            Ok(()) => {
                assert_eq!(windows_no_follow_open_flags(), 0x0020_0000);
                let receipt = release_install_status_receipt();
                assert_eq!(receipt.status, "pass");
                assert_eq!(receipt.progress_path, None);
                assert_eq!(
                    receipt.artifact_refs,
                    vec![latest_path.display().to_string()]
                );
                let receipt_body =
                    serde_json::to_string(&receipt).expect("status receipt should serialize");
                assert!(
                    !receipt_body.contains("API_TOKEN=[REDACTED:API key param]"),
                    "status receipt must not include symlink target contents"
                );
            }
            Err(error)
                if error.kind() == io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                assert_eq!(windows_no_follow_open_flags(), 0x0020_0000);
            }
            Err(error) => {
                panic!("latest path sidecar symlink should write or lack privilege: {error}")
            }
        }
        clean_release_progress_latest_markers();
    }

    #[cfg(unix)]
    #[test]
    fn release_install_progress_event_rejects_symlink_progress_artifact() {
        let _guard = release_progress_test_lock();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let _progress_dir = release_progress_dir_override(harness.path().join("progress"));
        clean_release_progress_latest_markers();
        let victim = harness.path().join("victim.jsonl");
        let progress_path = harness.path().join("progress-link.jsonl");
        fs::write(&victim, "unchanged").expect("victim should write");
        std::os::unix::fs::symlink(&victim, &progress_path).expect("symlink should write");

        let error = write_release_install_progress_event(
            &progress_path,
            "started",
            &release_build_command(),
            None,
        )
        .expect_err("symlinked progress artifact should be rejected");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            fs::read_to_string(&victim).expect("victim should read"),
            "unchanged"
        );
    }

    #[cfg(unix)]
    #[test]
    fn release_install_progress_event_rejects_symlink_latest_marker() {
        let _guard = release_progress_test_lock();
        clean_release_progress_latest_markers();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let progress_path = harness.path().join("release-install-progress.jsonl");
        let latest_path = release_install_progress_latest_path();
        let victim = harness.path().join("victim-latest.json");
        if let Some(parent) = latest_path.parent() {
            fs::create_dir_all(parent).expect("latest parent should write");
        }
        fs::write(&victim, "unchanged").expect("victim should write");
        std::os::unix::fs::symlink(&victim, &latest_path).expect("latest symlink should write");

        let error = write_release_install_progress_event(
            &progress_path,
            "started",
            &release_build_command(),
            None,
        )
        .expect_err("symlinked latest marker should be rejected");

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(
            fs::read_to_string(&victim).expect("victim should read"),
            "unchanged"
        );
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_path_target_resolves_first_vida_on_path() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let bin_dir = harness.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("path bin dir should write");
        let expected = bin_dir.join(vida_binary_file_name());
        fs::write(&expected, b"fake vida binary").expect("path vida should write");
        let path_env = std::env::join_paths([bin_dir]).expect("path env should join");

        let resolved =
            resolve_vida_from_path_env(Some(path_env)).expect("path target should resolve");

        assert_eq!(resolved, expected);
    }

    #[test]
    fn release_install_skip_build_installs_fake_binary_to_current_target() {
        let _guard = release_progress_test_lock();
        clean_release_progress_latest_markers();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: true,
            status: false,
            source_binary: Some(source.clone()),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.build.status, "skipped");
        assert_eq!(receipt.build.exit_code, Some(0));
        assert!(!receipt.build.artifact_refs.is_empty());
        let progress_path = receipt
            .build
            .progress_path
            .as_deref()
            .expect("skip-build install should record progress path");
        assert!(PathBuf::from(progress_path).is_file());
        let latest_progress = release_install_progress_latest_path();
        let latest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&latest_progress).expect("latest progress should write"),
        )
        .expect("latest progress should be json");
        assert_eq!(latest["status"], "pass");
        assert_eq!(latest["phase"], "install");
        assert_eq!(latest["exit_code"], 0);
        assert_eq!(latest["progress_path"], progress_path);
        assert_eq!(receipt.io_error, None);
        assert_eq!(receipt.asset_update.status, "refreshed");
        assert!(receipt
            .asset_update
            .refreshed_paths
            .iter()
            .any(|path| path == "vida/config"));
        assert!(harness
            .path()
            .join("install-root/current/vida/config/instructions/bundles/framework-source")
            .is_dir());
        assert!(harness
            .path()
            .join("install-root/current/install/assets/feature-design-document.template.md")
            .is_file());
        assert_eq!(receipt.installed_targets.len(), 1);
        assert_eq!(receipt.installed_targets[0].target, "current");
        assert_eq!(
            receipt.source_binary_fingerprint.as_deref(),
            Some(receipt.installed_targets[0].fingerprint.as_str())
        );
        assert!(PathBuf::from(&receipt.installed_targets[0].path).is_file());
        clean_release_progress_latest_markers();
    }

    #[cfg(unix)]
    #[test]
    fn release_install_does_not_execute_source_binary_for_version() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = release_progress_test_lock();
        clean_release_progress_latest_markers();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let marker = harness.path().join("executed-marker");
        let source = harness.path().join("fake-vida");
        fs::write(
            &source,
            format!(
                "#!/bin/sh\necho executed > {}\necho malicious-version\n",
                marker.display()
            ),
        )
        .expect("fake source should write");
        let mut permissions = fs::metadata(&source)
            .expect("fake source metadata should read")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&source, permissions).expect("fake source should be executable");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: true,
            status: false,
            source_binary: Some(source),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.installed_targets.len(), 1);
        assert_eq!(receipt.installed_targets[0].version, None);
        assert!(
            !marker.exists(),
            "release install identity reporting must not execute copied binaries"
        );
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_binary_replaces_existing_destination() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida-new");
        let destination = harness.path().join("bin").join(vida_binary_file_name());
        fs::create_dir_all(
            destination
                .parent()
                .expect("destination should have parent"),
        )
        .expect("destination parent should write");
        fs::write(&source, b"new fake vida binary").expect("fake source should write");
        fs::write(&destination, b"old fake vida binary").expect("destination should write");

        install_binary(&source, &destination).expect("install should replace existing binary");

        assert_eq!(
            fs::read(&destination).expect("destination should remain readable"),
            b"new fake vida binary"
        );
    }

    #[test]
    fn release_install_receipt_includes_explicit_cross_platform_layout() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");
        let install_root = harness.path().join("install-root");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: true,
            status: false,
            source_binary: Some(source),
            install_root: Some(install_root.clone()),
            json: true,
        });

        let layout = receipt
            .install_layout
            .as_ref()
            .expect("successful current install should expose layout");
        assert_eq!(layout.install_root, install_root.display().to_string());
        assert_eq!(
            layout.current_root,
            install_root.join("current").display().to_string()
        );
        assert_eq!(
            layout.runtime_bin_dir,
            install_root
                .join("current")
                .join("bin")
                .display()
                .to_string()
        );
        assert!(layout
            .env_file
            .ends_with(if cfg!(windows) { "env.ps1" } else { "env.sh" }));
        assert_eq!(layout.platform, std::env::consts::OS);
    }

    #[test]
    fn release_install_env_helpers_ignore_empty_values() {
        assert_eq!(super::non_empty_os_string(None), None);
        assert_eq!(
            super::non_empty_os_string(Some(std::ffi::OsString::new())),
            None
        );
        assert_eq!(
            super::non_empty_os_string(Some(std::ffi::OsString::from("configured"))),
            Some(std::ffi::OsString::from("configured"))
        );
    }

    #[test]
    fn release_install_env_path_helpers_ignore_unexpanded_windows_placeholders() {
        let placeholder = std::ffi::OsString::from(r"%SystemDrive%\ProgramData");
        let sanitized = super::non_empty_env_path_value(Some(placeholder.clone()));
        if cfg!(windows) {
            assert_eq!(sanitized, None);
        } else {
            assert_eq!(sanitized, Some(placeholder));
        }
    }

    #[test]
    fn release_install_default_root_skips_placeholder_env_paths() {
        let fallback_home = std::ffi::OsString::from(if cfg!(windows) {
            r"C:\Users\vida-test"
        } else {
            "/home/vida-test"
        });
        let root = super::default_release_install_root_from_values(
            Some(std::ffi::OsString::from(
                r"%SystemDrive%\ProgramData\vida-stack",
            )),
            Some(std::ffi::OsString::from(r"%SystemDrive%\ProgramData")),
            Some(fallback_home.clone()),
            None,
            None,
            None,
        )
        .expect("fallback home should produce install root");

        if cfg!(windows) {
            assert!(!root.to_string_lossy().contains("%SystemDrive%"));
            assert_eq!(
                root,
                PathBuf::from(fallback_home)
                    .join("AppData")
                    .join("Local")
                    .join("vida-stack")
            );
        } else {
            assert_eq!(root, PathBuf::from(r"%SystemDrive%\ProgramData\vida-stack"));
        }
    }

    #[test]
    fn release_install_skip_build_blocks_missing_source_binary() {
        let _guard = release_progress_test_lock();
        clean_release_progress_latest_markers();
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: true,
            status: false,
            source_binary: Some(harness.path().join("missing-vida")),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["missing_source_binary"]);
        assert_eq!(receipt.build.status, "skipped");
        assert_eq!(receipt.build.progress_path, None);
        assert!(!release_install_progress_latest_path().exists());
        assert_eq!(receipt.io_error, None);
        assert!(receipt.installed_targets.is_empty());
        clean_release_progress_latest_markers();
    }

    #[test]
    fn release_install_blocks_unsupported_target() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "global".to_string(),
            skip_build: true,
            status: false,
            source_binary: Some(source),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["unsupported_install_target"]);
        assert_eq!(
            receipt.next_actions,
            vec!["Use `--target current` or `--target cur`."]
        );
        assert_eq!(receipt.io_error, None);
        assert!(receipt.installed_targets.is_empty());
    }

    #[test]
    fn release_install_path_target_aliases_to_current_target() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let root = harness.path().join("install-root");

        let targets = install_target_paths_with_path_env("path", Some(&root), None)
            .expect("legacy path target should resolve to canonical current target");

        assert_eq!(
            targets,
            vec![(
                "current".to_string(),
                root.join("current")
                    .join("bin")
                    .join(vida_binary_file_name())
            )]
        );
    }

    #[test]
    #[cfg(unix)]
    fn release_install_path_target_ignores_symlink_entries() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let path_dir = harness.path().join("path-bin");
        fs::create_dir_all(&path_dir).expect("path dir should write");

        let target_file = harness.path().join("unrelated-file");
        fs::write(&target_file, b"unrelated").expect("target file should write");

        let symlink_path = path_dir.join(vida_binary_file_name());
        std::os::unix::fs::symlink(&target_file, &symlink_path).expect("symlink should write");

        let path_env = std::env::join_paths([path_dir]).expect("path env should join");

        assert_eq!(resolve_vida_from_path_env(Some(path_env)), None);
    }

    #[test]
    fn release_install_create_dir_failure_records_precise_io_detail() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");
        let install_root_file = harness.path().join("not-a-directory");
        fs::write(&install_root_file, b"file blocks directory creation")
            .expect("blocking file should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: true,
            status: false,
            source_binary: Some(source),
            install_root: Some(install_root_file.clone()),
            json: true,
        });

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["install_target_write_failed"]);
        let detail = receipt
            .io_error
            .as_ref()
            .expect("io detail should be recorded");
        assert_eq!(detail.operation, "create_dir");
        assert_eq!(
            detail.target_path,
            Some(
                install_root_file
                    .join("current")
                    .join("bin")
                    .display()
                    .to_string()
            )
        );
        assert_eq!(detail.staging_path, None);
        assert!(!detail.error_kind.is_empty());
        assert!(!detail.error_message.is_empty());
        assert_eq!(receipt.next_actions, vec![detail.next_action_hint.clone()]);

        let json = serde_json::to_value(&receipt).expect("receipt should serialize");
        assert_eq!(json["io_error"]["operation"], "create_dir");
        assert!(json["io_error"]["error_message"].as_str().is_some());
    }

    #[test]
    fn release_install_binary_copy_failure_records_staging_path() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let missing_source = harness.path().join("missing-source");
        let destination = harness.path().join("bin/vida");
        fs::create_dir_all(destination.parent().expect("destination parent"))
            .expect("destination parent should be writable");

        let detail = install_binary(&missing_source, &destination)
            .expect_err("missing source copy should fail with io detail");

        assert_eq!(detail.operation, "copy");
        assert_eq!(detail.target_path, Some(destination.display().to_string()));
        let staging_path = detail
            .staging_path
            .as_ref()
            .expect("staging path should be recorded");
        assert!(staging_path.contains(".vida.installing."));
        assert!(!detail.error_kind.is_empty());
        assert!(!detail.error_message.is_empty());
        assert!(!detail.next_action_hint.is_empty());
    }

    #[test]
    fn release_install_binary_fingerprint_failure_records_read_operation() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let missing_path = harness.path().join("missing-installed-vida");

        let detail = binary_fingerprint(&missing_path)
            .expect_err("missing fingerprint target should fail with io detail");

        assert_eq!(detail.operation, "read_fingerprint");
        assert_eq!(detail.target_path, Some(missing_path.display().to_string()));
        assert_eq!(detail.staging_path, None);
        assert!(!detail.error_kind.is_empty());
        assert!(!detail.error_message.is_empty());
        assert!(!detail.next_action_hint.is_empty());
    }

    #[test]
    fn release_install_detects_text_file_busy_error() {
        let destination = Path::new("/tmp");

        let text_file_busy_error = io::Error::from_raw_os_error(26);
        let detail = io_error_detail("rename", None, Some(destination), &text_file_busy_error);
        assert_eq!(detail.error_kind, "text_file_busy");
        assert_eq!(
            release_install_error_blocker_code(&detail.error_kind),
            "install_target_text_file_busy"
        );

        assert!(is_text_file_busy_error(&text_file_busy_error));
        assert!(is_text_file_busy_error(&io::Error::from_raw_os_error(26)));
        assert!(is_text_file_busy_error(&io::Error::new(
            io::ErrorKind::Other,
            "text file is busy"
        )));
        assert!(!is_text_file_busy_error(&io::Error::new(
            io::ErrorKind::PermissionDenied,
            "permission denied"
        )));

        assert!(is_text_file_busy_error(&io::Error::new(
            io::ErrorKind::Other,
            "Text file busy"
        )));
        assert_eq!(
            release_install_error_blocker_code("text_file_busy"),
            "install_target_text_file_busy"
        );
    }

    #[test]
    fn release_install_permission_denied_maps_to_blocker_and_error_kind() {
        let destination = Path::new("/tmp");
        let permission_denied_error =
            io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let detail = io_error_detail("copy", Some(destination), None, &permission_denied_error);

        assert_eq!(detail.error_kind, "install_target_permission_denied");
        assert_eq!(
            detail.next_action_hint,
            next_action_for_io_error(&permission_denied_error)
        );
        assert_eq!(
            release_install_error_blocker_code(&detail.error_kind),
            "install_target_permission_denied"
        );
    }

    #[test]
    fn release_install_read_only_sandbox_maps_to_blocker_and_error_kind() {
        let destination = Path::new("/tmp");
        let sandbox_error = io::Error::new(io::ErrorKind::Other, "read-only file system");
        let detail = io_error_detail("copy", Some(destination), None, &sandbox_error);

        assert_eq!(
            detail.error_kind,
            "install_target_read_only_or_sandbox_blocked"
        );
        assert_eq!(
            release_install_error_blocker_code(&detail.error_kind),
            "install_target_read_only_or_sandbox_blocked"
        );
    }

    #[test]
    fn release_install_blocked_receipt_includes_top_level_error_kind() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let destination = harness.path().join("bin").join("vida");
        let detail = io_error_detail(
            "copy",
            Some(&destination),
            Some(&harness.path().join(".vida.installing")),
            &io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
        );

        let receipt = blocked_receipt(
            "local".to_string(),
            "/tmp/source".to_string(),
            ReleaseBuildReceipt {
                status: "pass".to_string(),
                phase: "build".to_string(),
                skipped: true,
                command: None,
                exit_code: Some(0),
                process_id: None,
                child_state: Some("skipped".to_string()),
                progress_path: None,
                artifact_refs: Vec::new(),
            },
            BlockedRelease {
                blocker_code: release_install_error_blocker_code(&detail.error_kind),
                next_action: detail.next_action_hint.clone(),
                io_error: Some(detail),
            },
        );

        assert_eq!(
            receipt.error_kind,
            Some("install_target_permission_denied".to_string())
        );
        assert_eq!(
            receipt.blocker_codes,
            vec!["install_target_permission_denied"]
        );
        assert_eq!(
            receipt.next_actions,
            vec![receipt.io_error.as_ref().unwrap().next_action_hint.clone()]
        );
    }
}
