use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

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

pub(crate) fn read_state_fresh_json_projection(
    state_dir: &Path,
    projection_name: &str,
) -> Option<String> {
    read_fresh_json_projection_with_dependency_marker(state_dir, projection_name, None)
}

pub(crate) fn read_fresh_json_projection_with_dependency_marker(
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

pub(crate) fn read_recent_json_projection(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
) -> Option<String> {
    read_recent_json_projection_with_dependency_marker(
        state_dir,
        projection_name,
        max_age,
        current_launcher_mutation_marker(),
    )
}

pub(crate) fn read_state_recent_json_projection(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
) -> Option<String> {
    read_recent_json_projection_with_dependency_marker(state_dir, projection_name, max_age, None)
}

pub(crate) fn read_state_stale_recent_json_projection(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
) -> Option<String> {
    read_recent_json_projection_allowing_state_marker(state_dir, projection_name, max_age, None)
}

pub(crate) fn read_recent_json_projection_with_dependency_marker(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
    dependency_modified: Option<SystemTime>,
) -> Option<String> {
    let path = projection_path(state_dir, projection_name);
    if path_is_symlink(&path) {
        return None;
    }
    let cache_modified = std::fs::metadata(&path).ok()?.modified().ok()?;
    // Security invariant: "recent" cached projections must still be invalidated when
    // authoritative TaskFlow/state data mutates. Age-bounded cache reuse is allowed
    // only for projections that are newer than the latest state mutation marker.
    let state_modified = latest_state_mutation_marker(state_dir).ok()?;
    if cache_modified < state_modified {
        return None;
    }
    if dependency_modified.is_some_and(|modified| cache_modified < modified) {
        return None;
    }
    let cache_age = SystemTime::now().duration_since(cache_modified).ok()?;
    if cache_age > max_age {
        return None;
    }
    let body = read_json_without_following_symlinks(&path).ok()?;
    annotate_recent_projection(&body, projection_name, cache_age, max_age).or(Some(body))
}

fn read_recent_json_projection_allowing_state_marker(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
    dependency_modified: Option<SystemTime>,
) -> Option<String> {
    let path = projection_path(state_dir, projection_name);
    if path_is_symlink(&path) {
        return None;
    }
    let cache_modified = std::fs::metadata(&path).ok()?.modified().ok()?;
    if dependency_modified.is_some_and(|modified| cache_modified < modified) {
        return None;
    }
    let cache_age = SystemTime::now().duration_since(cache_modified).ok()?;
    if cache_age > max_age {
        return None;
    }
    let state_modified = latest_state_mutation_marker(state_dir).ok();
    let state_marker_newer = state_modified.is_some_and(|modified| cache_modified <= modified);
    let body = read_json_without_following_symlinks(&path).ok()?;
    annotate_recent_projection_with_status(
        &body,
        projection_name,
        cache_age,
        max_age,
        if state_marker_newer {
            "state_marker_stale_recent_projection"
        } else {
            "recent_projection"
        },
        if state_marker_newer {
            "bounded_state_marker_stale_ok_for_doctor_summary_read_only_operator_query"
        } else {
            "recent_bounded_stale_ok_for_read_only_operator_query"
        },
    )
    .or(Some(body))
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
    if path_is_symlink(&path) {
        return;
    }
    let body = format!(
        "{}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    );
    let _ = write_bytes_without_following_symlinks(&path, body.as_bytes());
}

fn projection_path(state_dir: &Path, projection_name: &str) -> PathBuf {
    state_dir
        .join("operator-projections")
        .join(format!("{projection_name}.json"))
}

fn annotate_recent_projection(
    body: &str,
    projection_name: &str,
    cache_age: Duration,
    max_age: Duration,
) -> Option<String> {
    annotate_recent_projection_with_status(
        body,
        projection_name,
        cache_age,
        max_age,
        "recent_projection",
        "recent_bounded_stale_ok_for_read_only_operator_query",
    )
}

fn annotate_recent_projection_with_status(
    body: &str,
    projection_name: &str,
    cache_age: Duration,
    max_age: Duration,
    status: &str,
    freshness_contract: &str,
) -> Option<String> {
    let mut payload = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let serde_json::Value::Object(object) = &mut payload else {
        return None;
    };
    object.insert(
        "projection_cache".to_string(),
        serde_json::json!({
            "status": status,
            "projection_name": projection_name,
            "age_millis": cache_age.as_millis(),
            "max_age_millis": max_age.as_millis(),
            "freshness_contract": freshness_contract,
        }),
    );
    serde_json::to_string_pretty(&payload).ok()
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

pub(crate) fn current_launcher_mutation_marker() -> Option<SystemTime> {
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
    write_bytes_without_following_symlinks(path, body.as_bytes())
}

#[cfg(unix)]
fn write_bytes_without_following_symlinks(path: &Path, body: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(O_NOFOLLOW_FLAG)
        .open(path)?;
    file.write_all(body)
}

#[cfg(not(unix))]
fn write_json_without_following_symlinks(path: &Path, body: &str) -> std::io::Result<()> {
    write_bytes_without_following_symlinks(path, body.as_bytes())
}

#[cfg(not(unix))]
fn write_bytes_without_following_symlinks(path: &Path, body: &[u8]) -> std::io::Result<()> {
    std::fs::write(path, body)
}

#[cfg(test)]
mod tests {
    use super::{
        read_fresh_json_projection, read_fresh_json_projection_with_dependency_marker,
        read_recent_json_projection, read_recent_json_projection_with_dependency_marker,
        read_state_fresh_json_projection, read_state_recent_json_projection,
        read_state_stale_recent_json_projection, touch_state_mutation_marker,
        write_json_projection,
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
    fn state_bound_json_projection_ignores_launcher_dependency_marker() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-state-bound-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        fs::write(root.join("manifest"), "stable").expect("marker should be writable");
        let payload = serde_json::json!({"surface": "vida doctor", "status": "pass"});
        write_json_projection(&root, "doctor-summary-latest", &payload);

        std::thread::sleep(Duration::from_millis(10));
        let dependency_modified = std::time::SystemTime::now();
        assert!(read_fresh_json_projection_with_dependency_marker(
            &root,
            "doctor-summary-latest",
            Some(dependency_modified),
        )
        .is_none());
        assert!(read_state_fresh_json_projection(&root, "doctor-summary-latest").is_some());
        assert!(read_state_recent_json_projection(
            &root,
            "doctor-summary-latest",
            Duration::from_secs(60)
        )
        .is_some());
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

    #[test]
    fn recent_json_projection_invalidates_when_state_marker_is_touched() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-recent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let payload = serde_json::json!({"surface": "vida status", "status": "pass"});
        write_json_projection(&root, "status-full-latest", &payload);

        std::thread::sleep(Duration::from_millis(10));
        touch_state_mutation_marker(&root);
        assert!(read_fresh_json_projection(&root, "status-full-latest").is_none());
        assert!(
            read_recent_json_projection(&root, "status-full-latest", Duration::from_secs(60))
                .is_none()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_recent_json_projection_reports_state_marker_staleness() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-stale-recent-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let payload = serde_json::json!({"surface": "vida doctor", "status": "pass"});
        write_json_projection(&root, "doctor-summary-latest", &payload);

        std::thread::sleep(Duration::from_millis(10));
        touch_state_mutation_marker(&root);
        assert!(read_state_recent_json_projection(
            &root,
            "doctor-summary-latest",
            Duration::from_secs(60)
        )
        .is_none());
        let stale = read_state_stale_recent_json_projection(
            &root,
            "doctor-summary-latest",
            Duration::from_secs(60),
        )
        .expect("stale recent projection should be available inside max age");
        let stale_json: serde_json::Value =
            serde_json::from_str(&stale).expect("stale projection should remain json");
        assert_eq!(
            stale_json["projection_cache"]["status"],
            "state_marker_stale_recent_projection"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recent_json_projection_still_respects_launcher_dependency_marker() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-recent-dep-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let payload = serde_json::json!({"surface": "vida doctor", "status": "pass"});
        write_json_projection(&root, "doctor-full-latest", &payload);

        std::thread::sleep(Duration::from_millis(10));
        let dependency_modified = std::time::SystemTime::now();
        assert!(read_recent_json_projection_with_dependency_marker(
            &root,
            "doctor-full-latest",
            Duration::from_secs(60),
            Some(dependency_modified),
        )
        .is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn state_mutation_marker_touch_rejects_symlink() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-marker-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");

        let outside = root.with_extension("outside-marker-target");
        fs::write(&outside, "outside-original").expect("outside file should be writable");

        let marker_link = root.join(".operator-projection-cache-state-marker");
        symlink(&outside, &marker_link).expect("marker symlink should be creatable");

        touch_state_mutation_marker(&root);

        assert_eq!(
            fs::read_to_string(&outside).expect("outside file should remain readable"),
            "outside-original"
        );
        assert!(std::fs::symlink_metadata(&marker_link)
            .expect("marker symlink should remain readable")
            .file_type()
            .is_symlink());

        let _ = fs::remove_file(outside);
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
