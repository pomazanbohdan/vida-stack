use std::fs;
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

    let build = if args.skip_build {
        ReleaseBuildReceipt {
            status: "skipped".to_string(),
            skipped: true,
            command: None,
            exit_code: None,
        }
    } else {
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
            Ok(status) => {
                return blocked_receipt(
                    requested_target,
                    source_binary_path,
                    ReleaseBuildReceipt {
                        status: "blocked".to_string(),
                        skipped: false,
                        command: Some(command),
                        exit_code: status.code(),
                    },
                    BlockedRelease {
                        blocker_code: "release_build_failed",
                        next_action: "Fix release build failures, then rerun `vida release install --json`.",
                    },
                );
            }
            Err(_) => {
                return blocked_receipt(
                    requested_target,
                    source_binary_path,
                    ReleaseBuildReceipt {
                        status: "blocked".to_string(),
                        skipped: false,
                        command: Some(command),
                        exit_code: None,
                    },
                    BlockedRelease {
                        blocker_code: "release_build_failed",
                        next_action: "Ensure `cargo build -p vida --release` can run, then rerun `vida release install --json`.",
                    },
                );
            }
        }
    };

    if !source_binary.is_file() {
        return blocked_receipt(
            requested_target,
            source_binary_path,
            build,
            BlockedRelease {
                blocker_code: "missing_source_binary",
                next_action: "Run `cargo build -p vida --release` or pass `--source-binary <path>`.",
            },
        );
    }

    let source_binary_fingerprint = match binary_fingerprint(&source_binary) {
        Ok(fingerprint) => fingerprint,
        Err(_) => {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                build,
                BlockedRelease {
                    blocker_code: "missing_source_binary",
                    next_action: "Ensure the source binary is readable, then rerun `vida release install --json`.",
                },
            );
        }
    };

    let mut installed_targets = Vec::new();
    for (target, path) in target_paths {
        if let Some(parent) = path.parent() {
            if fs::create_dir_all(parent).is_err() {
                return blocked_receipt(
                    requested_target,
                    source_binary_path,
                    build,
                    BlockedRelease {
                        blocker_code: "install_target_write_failed",
                        next_action: "Create the install target directory or choose another `--install-root`.",
                    },
                );
            }
        }
        if install_binary(&source_binary, &path).is_err() {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                build,
                BlockedRelease {
                    blocker_code: "install_target_write_failed",
                    next_action: "Check install target permissions and rerun the install command.",
                },
            );
        }
        let Ok(fingerprint) = binary_fingerprint(&path) else {
            return blocked_receipt(
                requested_target,
                source_binary_path,
                build,
                BlockedRelease {
                    blocker_code: "install_target_write_failed",
                    next_action: "Verify installed binary readability and rerun the install command.",
                },
            );
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
    }
}

struct BlockedRelease {
    blocker_code: &'static str,
    next_action: &'static str,
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
        next_actions: vec![blocked.next_action.to_string()],
        build,
        source_binary_path,
        source_binary_fingerprint: None,
        requested_target,
        installed_targets: Vec::new(),
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
            next_action: "Set HOME or pass `--install-root <path>`.",
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
            next_action: "Use `--target all`, `--target local`, or `--target cargo`.",
        }),
    }
}

fn binary_fingerprint(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn install_binary(source: &Path, destination: &Path) -> Result<(), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| "install destination has no parent directory".to_string())?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "install destination has no file name".to_string())?;
    let staging_path = parent.join(format!(".{file_name}.installing"));

    fs::copy(source, &staging_path).map_err(|error| error.to_string())?;
    let permissions = fs::metadata(source)
        .map_err(|error| error.to_string())?
        .permissions();
    fs::set_permissions(&staging_path, permissions).map_err(|error| error.to_string())?;
    fs::rename(&staging_path, destination).map_err(|error| {
        let _ = fs::remove_file(&staging_path);
        error.to_string()
    })?;
    Ok(())
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
        assert!(receipt.installed_targets.is_empty());
    }
}
