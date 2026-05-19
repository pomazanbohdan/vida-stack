use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(unix)]
const O_NOFOLLOW_FLAG: i32 = libc::O_NOFOLLOW;

pub(crate) fn read_fresh_json_projection(
    state_dir: &Path,
    projection_name: &str,
) -> Option<String> {
    read_fresh_json_projection_with_dependency_marker(
        state_dir,
        projection_name,
        current_launcher_mutation_marker(),
    )
}

fn read_fresh_json_projection_with_dependency_marker(
    state_dir: &Path,
    projection_name: &str,
    dependency_modified: Option<SystemTime>,
) -> Option<String> {
    let path = projection_path(state_dir, projection_name);
    if path_is_symlink(&path) {
        return None;
    }
    let cache_modified = std::fs::metadata(&path).ok()?.modified().ok()?;
    let state_modified = latest_state_mutation_marker(state_dir).ok()?;
    if cache_modified < state_modified {
        return None;
    }
    if dependency_modified.is_some_and(|modified| cache_modified < modified) {
        return None;
    }
    read_json_without_following_symlinks(&path).ok()
}

pub(crate) fn write_json_projection(
    state_dir: &Path,
    projection_name: &str,
    payload: &serde_json::Value,
) {
    let path = projection_path(state_dir, projection_name);
    if let Some(parent) = path.parent() {
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
    }
    let Ok(body) = serde_json::to_string_pretty(payload) else {
        return;
    };
    if path_is_symlink(&path) {
        return;
    }
    let _ = write_json_without_following_symlinks(&path, &body);
}

pub(crate) fn touch_state_mutation_marker(state_dir: &Path) {
    let path = state_dir.join(".operator-projection-cache-state-marker");
    let _ = std::fs::write(
        path,
        format!(
            "{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ),
    );
}

fn projection_path(state_dir: &Path, projection_name: &str) -> PathBuf {
    state_dir
        .join("operator-projections")
        .join(format!("{projection_name}.json"))
}

fn latest_state_mutation_marker(state_dir: &Path) -> std::io::Result<SystemTime> {
    let mut latest = SystemTime::UNIX_EPOCH;
    for entry in std::fs::read_dir(state_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                matches!(
                    name,
                    "operator-projections" | "LOCK" | ".vida-authoritative-open.guard" | "wal"
                )
            })
        {
            continue;
        }
        if let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) {
            if modified > latest {
                latest = modified;
            }
        }
    }
    Ok(latest)
}

fn current_launcher_mutation_marker() -> Option<SystemTime> {
    std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
}

fn path_is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
}

#[cfg(unix)]
fn read_json_without_following_symlinks(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW_FLAG)
        .open(path)?;
    let mut body = String::new();
    file.read_to_string(&mut body)?;
    Ok(body)
}

#[cfg(not(unix))]
fn read_json_without_following_symlinks(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

#[cfg(unix)]
fn write_json_without_following_symlinks(path: &Path, body: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(O_NOFOLLOW_FLAG)
        .open(path)?;
    file.write_all(body.as_bytes())
}

#[cfg(not(unix))]
fn write_json_without_following_symlinks(path: &Path, body: &str) -> std::io::Result<()> {
    std::fs::write(path, body)
}

#[cfg(test)]
mod tests {
    use super::{
        read_fresh_json_projection, read_fresh_json_projection_with_dependency_marker,
        touch_state_mutation_marker, write_json_projection,
    };
    use std::{fs, time::Duration};

    #[test]
    fn json_projection_cache_invalidates_when_state_marker_is_newer() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker = root.join("manifest");
        fs::write(&marker, "old").expect("marker should be writable");
        let payload = serde_json::json!({"status": "pass", "cached": true});
        write_json_projection(&root, "status-summary-latest", &payload);
        assert!(read_fresh_json_projection(&root, "status-summary-latest").is_some());

        std::thread::sleep(Duration::from_millis(10));
        fs::write(&marker, "new").expect("marker should be updateable");
        assert!(read_fresh_json_projection(&root, "status-summary-latest").is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn json_projection_cache_invalidates_when_launcher_dependency_is_newer() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-launcher-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        fs::write(root.join("manifest"), "stable").expect("marker should be writable");
        let payload = serde_json::json!({"status": "pass", "cached": true});
        write_json_projection(&root, "doctor-latest", &payload);
        assert!(read_fresh_json_projection_with_dependency_marker(
            &root,
            "doctor-latest",
            Some(std::time::SystemTime::UNIX_EPOCH)
        )
        .is_some());

        std::thread::sleep(Duration::from_millis(10));
        let dependency_modified = std::time::SystemTime::now();
        assert!(read_fresh_json_projection_with_dependency_marker(
            &root,
            "doctor-latest",
            Some(dependency_modified)
        )
        .is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn json_projection_cache_invalidates_when_state_marker_is_touched() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-marker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let payload = serde_json::json!({"status": "blocked", "cached": true});
        write_json_projection(&root, "taskflow-graph-summary-latest", &payload);
        assert!(read_fresh_json_projection(&root, "taskflow-graph-summary-latest").is_some());

        std::thread::sleep(Duration::from_millis(10));
        touch_state_mutation_marker(&root);
        assert!(read_fresh_json_projection(&root, "taskflow-graph-summary-latest").is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn json_projection_cache_rejects_symlink_read_and_write() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("operator-projections"))
            .expect("projection directory should be writable");
        fs::write(root.join("manifest"), "stable").expect("marker should be writable");

        let outside = root.with_extension("outside-target");
        fs::write(&outside, "outside-original").expect("outside file should be writable");
        let link = root
            .join("operator-projections")
            .join("taskflow-graph-summary-latest.json");
        symlink(&outside, &link).expect("symlink should be creatable");

        let payload = serde_json::json!({"status": "should-not-overwrite"});
        write_json_projection(&root, "taskflow-graph-summary-latest", &payload);
        assert_eq!(
            fs::read_to_string(&outside).expect("outside file should remain readable"),
            "outside-original"
        );
        assert!(read_fresh_json_projection(&root, "taskflow-graph-summary-latest").is_none());

        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(root);
    }
}
