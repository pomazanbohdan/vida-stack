use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Serialize;

use crate::ReleaseInstallArgs;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseInstallReceipt {
    pub status: String,
    pub blocker_codes: Vec<String>,
    pub next_actions: Vec<String>,
    pub build: ReleaseBuildReceipt,
    pub source_binary_path: String,
    pub source_binary_fingerprint: Option<String>,
    pub requested_target: String,
    pub installed_targets: Vec<ReleaseInstalledTarget>,
    pub io_error: Option<ReleaseIoErrorDetail>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseBuildReceipt {
    pub status: String,
    pub skipped: bool,
    pub command: Option<Vec<String>>,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(crate) struct ReleaseInstalledTarget {
    pub target: String,
    pub path: String,
    pub fingerprint: String,
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
        if let Some(parent) = path.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                return blocked_receipt(
                    requested_target,
                    source_binary_path,
                    build,
                    BlockedRelease {
                        blocker_code: "install_target_write_failed",
                        next_action: next_action_for_io_error(&error).to_string(),
                        io_error: Some(io_error_detail("create_dir", Some(parent), None, &error)),
                    },
                );
            }
        }
        if let Err(io_error) = install_binary(&source_binary, &path) {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                build,
                BlockedRelease {
                    blocker_code: "install_target_write_failed",
                    next_action: io_error.next_action_hint.clone(),
                    io_error: Some(io_error),
                },
            );
        }
        let fingerprint = match binary_fingerprint(&path) {
            Ok(fingerprint) => fingerprint,
            Err(io_error) => {
                return blocked_receipt(
                    requested_target,
                    source_binary_path,
                    build,
                    BlockedRelease {
                        blocker_code: "install_target_write_failed",
                        next_action: io_error.next_action_hint.clone(),
                        io_error: Some(io_error),
                    },
                );
            }
        };
        installed_targets.push(ReleaseInstalledTarget {
            target,
            path: path.display().to_string(),
            fingerprint,
        });
    }

    ReleaseInstallReceipt {
        status: "pass".to_string(),
        blocker_codes: Vec::new(),
        next_actions: vec![
            "Run `vida --help` from a new shell and verify the expected binary is first on PATH."
                .to_string(),
        ],
        build,
        source_binary_path,
        source_binary_fingerprint: Some(source_binary_fingerprint),
        requested_target,
        installed_targets,
        io_error: None,
    }
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

    let command = vec![
        "cargo".to_string(),
        "build".to_string(),
        "-p".to_string(),
        "vida".to_string(),
        "--release".to_string(),
    ];
    match Command::new("cargo")
        .args(["build", "-p", "vida", "--release"])
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
    ReleaseInstallReceipt {
        status: "blocked".to_string(),
        blocker_codes: vec![blocked.blocker_code.to_string()],
        next_actions: vec![blocked.next_action],
        build,
        source_binary_path,
        source_binary_fingerprint: None,
        requested_target,
        installed_targets: Vec::new(),
        io_error: blocked.io_error,
    }
}

fn default_source_binary_path() -> PathBuf {
    PathBuf::from("target").join("release").join("vida")
}

fn install_target_paths(
    requested_target: &str,
    install_root: Option<&Path>,
) -> Result<Vec<(String, PathBuf)>, BlockedRelease> {
    let root = install_root
        .map(Path::to_path_buf)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .ok_or(BlockedRelease {
            blocker_code: "install_target_unresolved",
            next_action: "Set HOME or pass `--install-root <path>`.".to_string(),
            io_error: None,
        })?;
    match requested_target {
        "all" => Ok(vec![
            ("local".to_string(), root.join(".local/bin/vida")),
            ("cargo".to_string(), root.join(".cargo/bin/vida")),
        ]),
        "local" => Ok(vec![("local".to_string(), root.join(".local/bin/vida"))]),
        "cargo" => Ok(vec![("cargo".to_string(), root.join(".cargo/bin/vida"))]),
        _ => Err(BlockedRelease {
            blocker_code: "unsupported_install_target",
            next_action: "Use `--target all`, `--target local`, or `--target cargo`.".to_string(),
            io_error: None,
        }),
    }
}

fn binary_fingerprint(path: &Path) -> Result<String, ReleaseIoErrorDetail> {
    let bytes = fs::read(path)
        .map_err(|error| io_error_detail("read_fingerprint", Some(path), None, &error))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
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
    let staging_path = parent.join(format!(".{file_name}.installing"));

    fs::copy(source, &staging_path)
        .map_err(|error| io_error_detail("copy", Some(destination), Some(&staging_path), &error))?;
    let permissions = fs::metadata(source)
        .map_err(|error| io_error_detail("read_source_metadata", Some(source), None, &error))?
        .permissions();
    fs::set_permissions(&staging_path, permissions).map_err(|error| {
        io_error_detail(
            "set_permissions",
            Some(destination),
            Some(&staging_path),
            &error,
        )
    })?;
    fs::rename(&staging_path, destination).map_err(|error| {
        let _ = fs::remove_file(&staging_path);
        io_error_detail("rename", Some(destination), Some(&staging_path), &error)
    })?;
    Ok(())
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
        error_kind: format!("{:?}", error.kind()),
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
    if error.kind() == io::ErrorKind::PermissionDenied {
        "Check install target permissions, choose a writable `--install-root`, or rerun with an explicitly approved install path."
    } else if error.raw_os_error() == Some(30) || message.contains("read-only file system") {
        "The install target is on a read-only filesystem or blocked by sandboxing; choose a writable `--install-root` such as `/tmp/...` or rerun with explicit filesystem approval."
    } else {
        "Inspect the structured IO error detail, choose a writable install target, and rerun the install command."
    }
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
        assert!(help.contains("--source-binary"));
        assert!(help.contains("--install-root"));
    }

    #[test]
    fn release_install_skip_build_installs_fake_binary_to_local_target() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "local".to_string(),
            skip_build: true,
            source_binary: Some(source.clone()),
            install_root: Some(harness.path().join("install-root")),
            json: true,
        });

        assert_eq!(receipt.status, "pass");
        assert_eq!(receipt.build.status, "skipped");
        assert_eq!(receipt.io_error, None);
        assert_eq!(receipt.installed_targets.len(), 1);
        assert_eq!(receipt.installed_targets[0].target, "local");
        assert_eq!(
            receipt.source_binary_fingerprint.as_deref(),
            Some(receipt.installed_targets[0].fingerprint.as_str())
        );
        assert!(PathBuf::from(&receipt.installed_targets[0].path).is_file());
    }

    #[test]
    fn release_install_skip_build_blocks_missing_source_binary() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "local".to_string(),
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
    fn release_install_create_dir_failure_records_precise_io_detail() {
        let harness = TempStateHarness::new().expect("temp harness should initialize");
        let source = harness.path().join("fake-vida");
        fs::write(&source, b"fake vida binary").expect("fake source should write");
        let install_root_file = harness.path().join("not-a-directory");
        fs::write(&install_root_file, b"file blocks directory creation")
            .expect("blocking file should write");

        let receipt = release_install_receipt(&ReleaseInstallArgs {
            target: "local".to_string(),
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
            Some(install_root_file.join(".local/bin").display().to_string())
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
        assert_eq!(
            detail.staging_path,
            Some(
                destination
                    .parent()
                    .expect("destination parent")
                    .join(".vida.installing")
                    .display()
                    .to_string()
            )
        );
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
}
