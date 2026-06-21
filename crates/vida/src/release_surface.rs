use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::Duration;

use serde::Serialize;

use crate::ReleaseInstallArgs;

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
    pub skipped: bool,
    pub command: Option<Vec<String>>,
    pub exit_code: Option<i32>,
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
    let receipt = release_install_receipt(&args);
    emit_release_install_receipt(&receipt, args.json)
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
    } else {
        eprintln!(
            "release install: blocked ({})",
            receipt.blocker_codes.join(", ")
        );
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
                    skipped: args.skip_build,
                    command: None,
                    exit_code: None,
                },
                receipt,
            );
        }
    };

    let build = release_build_receipt(args.skip_build);
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
    });
    let _ = write_binary_fingerprint_metadata(destination, &fingerprint);
    Ok(())
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

pub(crate) fn release_build_receipt(skip_build: bool) -> ReleaseBuildReceipt {
    if skip_build {
        return ReleaseBuildReceipt {
            status: "skipped".to_string(),
            skipped: true,
            command: None,
            exit_code: None,
        };
    }

    let command = release_build_command();
    match Command::new("cargo")
        .args(command.iter().skip(1).map(String::as_str))
        .current_dir(trusted_workspace_root())
        .status()
    {
        Ok(status) if status.success() => ReleaseBuildReceipt {
            status: "pass".to_string(),
            skipped: false,
            command: Some(command),
            exit_code: status.code(),
        },
        Ok(status) => ReleaseBuildReceipt {
            status: "blocked".to_string(),
            skipped: false,
            command: Some(command),
            exit_code: status.code(),
        },
        Err(_) => ReleaseBuildReceipt {
            status: "blocked".to_string(),
            skipped: false,
            command: Some(command),
            exit_code: None,
        },
    }
}

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
    let root = release_install_root(install_root);
    let binary_name = vida_binary_file_name();
    match requested_target {
        "current" | "all" | "local" | "cargo" => {
            let root = root.ok_or_else(unresolved_install_target)?;
            Ok(vec![(
                "current".to_string(),
                root.join("current").join("bin").join(binary_name),
            )])
        }
        "path" => resolve_vida_from_path_env(std::env::var_os("PATH"))
            .map(|path| vec![("path".to_string(), path)])
            .ok_or(BlockedRelease {
                blocker_code: "install_target_unresolved",
                next_action: "Ensure `vida` is on PATH, or pass `--target current` with `--install-root <path>`.".to_string(),
                io_error: None,
            }),
        _ => Err(BlockedRelease {
            blocker_code: "unsupported_install_target",
            next_action: "Use `--target current` or `--target path`."
                .to_string(),
            io_error: None,
        }),
    }
}

fn companion_runtime_install_target_paths(
    requested_target: &str,
    install_root: Option<&Path>,
) -> Result<Vec<(String, PathBuf)>, BlockedRelease> {
    match requested_target {
        "current" | "all" | "local" | "cargo" => {
            let root = release_install_root(install_root).ok_or_else(unresolved_install_target)?;
            Ok(vec![(
                "current:vida-pi-agent".to_string(),
                root.join("current")
                    .join("bin")
                    .join(pi_agent_binary_file_name()),
            )])
        }
        "path" => Ok(Vec::new()),
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
    if let Some(vida_home) = non_empty_env_var("VIDA_HOME") {
        return Some(PathBuf::from(vida_home));
    }
    #[cfg(windows)]
    {
        if let Some(local_app_data) = non_empty_env_var("LOCALAPPDATA") {
            return Some(PathBuf::from(local_app_data).join("vida-stack"));
        }
    }
    user_home_dir().map(|home| {
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
    non_empty_env_var("HOME")
        .or_else(|| non_empty_env_var("USERPROFILE"))
        .or_else(|| {
            let drive = non_empty_env_var("HOMEDRIVE")?;
            let path = non_empty_env_var("HOMEPATH")?;
            let mut combined = std::ffi::OsString::from(drive);
            combined.push(path);
            Some(combined)
        })
        .map(PathBuf::from)
}

fn non_empty_env_var(name: &str) -> Option<OsString> {
    non_empty_os_string(std::env::var_os(name))
}

fn non_empty_os_string(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
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

    #[test]
    fn release_install_help_exposes_options() {
        let error = Cli::try_parse_from(["vida", "release", "install", "--help"])
            .expect_err("help should render clap display error");
        let help = error.to_string();

        assert!(help.contains("--json"));
        assert!(help.contains("--skip-build"));
        assert!(help.contains("--target"));
        assert!(help.contains("path"));
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
    fn release_install_path_target_does_not_guess_pi_agent_companion_destination() {
        let paths = companion_runtime_install_target_paths("path", None)
            .expect("path companion target should be a no-op");

        assert!(paths.is_empty());
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
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: true,
            source_binary: Some(source.clone()),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.build.status, "skipped");
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
    fn release_install_skip_build_blocks_missing_source_binary() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "current".to_string(),
            skip_build: true,
            source_binary: Some(harness.path().join("missing-vida")),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["missing_source_binary"]);
        assert_eq!(receipt.io_error, None);
        assert!(receipt.installed_targets.is_empty());
    }

    #[test]
    fn release_install_blocks_unsupported_target() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "global".to_string(),
            skip_build: true,
            source_binary: Some(source),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "blocked");
        assert_eq!(receipt.blocker_codes, vec!["unsupported_install_target"]);
        assert_eq!(receipt.io_error, None);
        assert!(receipt.installed_targets.is_empty());
    }

    #[test]
    fn release_install_blocks_unresolved_path_target() {
        assert_eq!(resolve_vida_from_path_env(None), None);
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
                skipped: true,
                command: None,
                exit_code: Some(0),
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
