use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PacketPathPlatform {
    Windows,
    Posix,
}

#[derive(Clone, Copy, Debug)]
pub struct RuntimePacketReceiptIdentity<'a> {
    pub receipt_run_id: &'a str,
    pub receipt_dispatch_packet_path: Option<&'a str>,
    pub receipt_downstream_dispatch_packet_path: Option<&'a str>,
    pub packet_run_id: Option<&'a str>,
    pub packet_path: &'a str,
    pub packet_label: &'a str,
}

#[must_use]
pub fn normalize_persisted_runtime_path(path: &str) -> PathBuf {
    let trimmed = path.trim();
    #[cfg(windows)]
    {
        if let Some(rest) = trimmed.strip_prefix("/mnt/") {
            let mut parts = rest.splitn(2, '/');
            if let (Some(drive), Some(tail)) = (parts.next(), parts.next())
                && drive.len() == 1
                && drive.as_bytes()[0].is_ascii_alphabetic()
            {
                let mut normalized = String::new();
                normalized.push_str(&drive.to_ascii_uppercase());
                normalized.push_str(":\\");
                normalized.push_str(&tail.replace('/', "\\"));
                return PathBuf::from(normalized);
            }
        }
    }
    PathBuf::from(trimmed)
}

pub fn validate_runtime_packet_run_id_component(run_id: &str) -> Result<&str, String> {
    let value = run_id.trim();
    if value.is_empty() {
        return Err("Failed to write dispatch packet: receipt.run_id is empty".to_string());
    }
    if value.contains('/') || value.contains('\\') {
        return Err(format!(
            "Failed to write dispatch packet: receipt.run_id `{value}` contains path separators"
        ));
    }
    if value == "." || value == ".." {
        return Err(format!(
            "Failed to write dispatch packet: receipt.run_id `{value}` is not a valid filename segment"
        ));
    }
    Ok(value)
}

#[must_use]
pub fn runtime_packet_paths_equivalent(left: &str, right: &str) -> bool {
    match (
        canonical_runtime_packet_identity(left),
        canonical_runtime_packet_identity(right),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub fn validate_runtime_packet_receipt_identity(
    identity: RuntimePacketReceiptIdentity<'_>,
) -> Result<(), String> {
    let packet_run_id = identity
        .packet_run_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Persisted {} is missing run_id", identity.packet_label))?;
    if packet_run_id != identity.receipt_run_id {
        return Err(format!(
            "Persisted {} run_id `{packet_run_id}` does not match dispatch receipt run_id `{}`",
            identity.packet_label, identity.receipt_run_id
        ));
    }
    if let Some(expected_dispatch_packet_path) = identity
        .receipt_dispatch_packet_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        && !runtime_packet_paths_equivalent(expected_dispatch_packet_path, identity.packet_path)
    {
        let expected_downstream_packet_path = identity
            .receipt_downstream_dispatch_packet_path
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if !expected_downstream_packet_path
            .map(|path| runtime_packet_paths_equivalent(path, identity.packet_path))
            .unwrap_or(false)
        {
            return Err(format!(
                "Persisted dispatch receipt expects dispatch_packet_path `{expected_dispatch_packet_path}` but resolved `{}`",
                identity.packet_path
            ));
        }
    }
    Ok(())
}

pub fn canonical_runtime_packet_identity(path: &str) -> Result<PathBuf, String> {
    if packet_path_has_dot_segment(path, current_packet_path_platform()) {
        return Err(format!(
            "Runtime packet path `{}` contains dot-segment traversal and is not admissible.",
            path.trim()
        ));
    }
    let normalized = normalize_persisted_runtime_path(path);
    let canonical = std::fs::canonicalize(&normalized).map_err(|error| {
        format!(
            "Failed to canonicalize runtime packet path `{}`: {error}",
            normalized.display()
        )
    })?;
    let parent = canonical.parent().ok_or_else(|| {
        format!(
            "Runtime packet path `{}` has no canonical parent.",
            canonical.display()
        )
    })?;
    let parent_name = parent
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let grandparent_name = parent
        .parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let packet_dir_allowed = matches!(
        parent_name,
        "dispatch-packets" | "downstream-dispatch-packets"
    ) && grandparent_name == "runtime-consumption";
    if !packet_dir_allowed {
        return Err(format!(
            "Runtime packet path `{}` is outside VIDA runtime packet directories.",
            canonical.display()
        ));
    }
    Ok(canonical)
}

fn current_packet_path_platform() -> PacketPathPlatform {
    if cfg!(windows) {
        PacketPathPlatform::Windows
    } else {
        PacketPathPlatform::Posix
    }
}

fn packet_path_components_for_platform(path: &str, platform: PacketPathPlatform) -> Vec<&str> {
    let trimmed = path.trim();
    let stripped = match platform {
        PacketPathPlatform::Windows => trimmed
            .strip_prefix(r"\\?\")
            .or_else(|| trimmed.strip_prefix("//?/"))
            .unwrap_or(trimmed),
        PacketPathPlatform::Posix => trimmed,
    };
    match platform {
        PacketPathPlatform::Windows => stripped
            .split(['/', '\\'])
            .filter(|part| !part.is_empty())
            .collect(),
        PacketPathPlatform::Posix => stripped
            .split('/')
            .filter(|part| !part.is_empty())
            .collect(),
    }
}

fn packet_path_has_dot_segment(path: &str, platform: PacketPathPlatform) -> bool {
    packet_path_components_for_platform(path, platform)
        .iter()
        .any(|part| *part == "." || *part == "..")
}

#[cfg(test)]
mod tests {
    use super::{
        PacketPathPlatform, RuntimePacketReceiptIdentity, canonical_runtime_packet_identity,
        packet_path_components_for_platform, runtime_packet_paths_equivalent,
        validate_runtime_packet_receipt_identity, validate_runtime_packet_run_id_component,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_runtime_packet_root(name: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{name}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn runtime_packet_run_id_component_rejects_empty_separators_and_dot_segments() {
        assert_eq!(
            validate_runtime_packet_run_id_component("run-1").expect("valid run id"),
            "run-1"
        );
        let empty_error = validate_runtime_packet_run_id_component(" ")
            .expect_err("empty run id should fail closed");
        assert_eq!(
            empty_error,
            "Failed to write dispatch packet: receipt.run_id is empty"
        );
        assert!(validate_runtime_packet_run_id_component("run/1").is_err());
        assert!(validate_runtime_packet_run_id_component(r"run\1").is_err());
        assert!(validate_runtime_packet_run_id_component("..").is_err());
    }

    #[test]
    fn runtime_packet_receipt_identity_requires_matching_run_id_and_packet_path() {
        let root = unique_runtime_packet_root("taskflow-core-packet-identity");
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("packet.json");
        fs::write(&packet_path, "{}").expect("write packet file");
        let packet_path_string = packet_path.display().to_string();

        validate_runtime_packet_receipt_identity(RuntimePacketReceiptIdentity {
            receipt_run_id: "run-identity",
            receipt_dispatch_packet_path: Some(&packet_path_string),
            receipt_downstream_dispatch_packet_path: None,
            packet_run_id: Some("run-identity"),
            packet_path: &packet_path_string,
            packet_label: "dispatch packet",
        })
        .expect("matching packet identity should validate");

        let run_id_error = validate_runtime_packet_receipt_identity(RuntimePacketReceiptIdentity {
            receipt_run_id: "run-identity",
            receipt_dispatch_packet_path: Some(&packet_path_string),
            receipt_downstream_dispatch_packet_path: None,
            packet_run_id: Some("other-run"),
            packet_path: &packet_path_string,
            packet_label: "dispatch packet",
        })
        .expect_err("run_id mismatch should fail closed");
        assert!(run_id_error.contains("does not match dispatch receipt run_id"));

        let other_packet = packet_dir.join("other.json");
        fs::write(&other_packet, "{}").expect("write other packet");
        let other_packet_string = other_packet.display().to_string();
        let path_error = validate_runtime_packet_receipt_identity(RuntimePacketReceiptIdentity {
            receipt_run_id: "run-identity",
            receipt_dispatch_packet_path: Some(&packet_path_string),
            receipt_downstream_dispatch_packet_path: None,
            packet_run_id: Some("run-identity"),
            packet_path: &other_packet_string,
            packet_label: "dispatch packet",
        })
        .expect_err("packet path mismatch should fail closed");
        assert!(path_error.contains("expects dispatch_packet_path"));

        let downstream_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&downstream_dir).expect("create downstream packet dir");
        let downstream_packet = downstream_dir.join("downstream.json");
        fs::write(&downstream_packet, "{}").expect("write downstream packet");
        let downstream_packet_string = downstream_packet.display().to_string();
        validate_runtime_packet_receipt_identity(RuntimePacketReceiptIdentity {
            receipt_run_id: "run-identity",
            receipt_dispatch_packet_path: Some(&packet_path_string),
            receipt_downstream_dispatch_packet_path: Some(&downstream_packet_string),
            packet_run_id: Some("run-identity"),
            packet_path: &downstream_packet_string,
            packet_label: "dispatch packet",
        })
        .expect("matching downstream packet identity should validate");

        let outside_dir = root.join("other/dispatch-packets");
        fs::create_dir_all(&outside_dir).expect("create outside packet dir");
        let outside_packet = outside_dir.join("outside.json");
        fs::write(&outside_packet, "{}").expect("write outside packet");
        let outside_error = canonical_runtime_packet_identity(&outside_packet.display().to_string())
            .expect_err("packet outside runtime-consumption should fail closed");
        assert!(outside_error.contains("outside VIDA runtime packet directories"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_packet_identity_rejects_dot_segment_packet_path_escape() {
        let root = unique_runtime_packet_root("taskflow-core-dot-segment-packet");
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("packet.json");
        fs::write(&packet_path, "{}").expect("write packet file");
        let dot_segment_path = packet_dir.join("../dispatch-packets/packet.json");

        let error = canonical_runtime_packet_identity(&dot_segment_path.display().to_string())
            .expect_err("dot-segment path should fail closed");
        assert!(error.contains("dot-segment traversal"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_packet_identity_allows_downstream_packet_directory() {
        let root = unique_runtime_packet_root("taskflow-core-downstream-packet");
        let packet_dir = root.join("runtime-consumption/downstream-dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("packet.json");
        fs::write(&packet_path, "{}").expect("write packet file");

        assert!(
            canonical_runtime_packet_identity(&packet_path.display().to_string()).is_ok(),
            "downstream dispatch packet directory is part of the runtime packet identity policy"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn runtime_packet_paths_equivalent_accepts_mixed_windows_separators() {
        let root = unique_runtime_packet_root("taskflow-core-mixed-separator-packet");
        let packet_dir = root.join("runtime-consumption/dispatch-packets");
        fs::create_dir_all(&packet_dir).expect("create packet dir");
        let packet_path = packet_dir.join("packet.json");
        fs::write(&packet_path, "{}").expect("write packet file");
        let expected_path = packet_path.display().to_string().replace('/', "\\");
        let resolved_path = packet_path.display().to_string().replace('\\', "/");

        assert!(runtime_packet_paths_equivalent(
            &expected_path,
            &resolved_path
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn posix_backslash_path_collision_is_not_separator_equivalent() {
        let left =
            packet_path_components_for_platform("packets/a\\b.json", PacketPathPlatform::Posix);
        let right =
            packet_path_components_for_platform("packets/a/b.json", PacketPathPlatform::Posix);
        assert_ne!(left, right);
        assert!(!runtime_packet_paths_equivalent(
            "missing-packet.json",
            "missing-packet.json"
        ));
    }
}
