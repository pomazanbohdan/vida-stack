use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const RUNTIME_CONTINUATION_BINDING_OVERLAY_PROJECTION_NAME: &str =
    "runtime-continuation-binding-latest";

#[cfg(unix)]
const O_NOFOLLOW_FLAG: i32 = libc::O_NOFOLLOW;

pub(crate) fn sanitize_projection_component(value: &str, fallback: &str, max_len: usize) -> String {
    let mut safe = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    safe.truncate(max_len);
    if safe.is_empty() {
        fallback.to_string()
    } else {
        safe
    }
}

pub(crate) fn read_fresh_json_projection(
    state_dir: &Path,
    projection_name: &str,
) -> Option<String> {
    read_fresh_json_projection_with_dependency_marker(
        state_dir,
        projection_name,
        current_operator_dependency_mutation_marker(state_dir),
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
    if cache_modified <= state_modified {
        return None;
    }
    if dependency_modified.is_some_and(|modified| cache_modified <= modified) {
        return None;
    }
    let body = read_json_without_following_symlinks(&path).ok()?;
    if !projection_task_snapshot_marker_matches(state_dir, &body) {
        return None;
    }
    Some(body)
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
        current_operator_dependency_mutation_marker(state_dir),
    )
}

pub(crate) fn read_state_recent_json_projection(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
) -> Option<String> {
    read_recent_json_projection_with_dependency_marker(state_dir, projection_name, max_age, None)
}

pub(crate) fn read_launcher_stale_state_fresh_recent_json_projection(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
) -> Option<String> {
    read_recent_json_projection_with_state_freshness_status(
        state_dir,
        projection_name,
        max_age,
        "launcher_stale_recent_projection",
        "bounded_launcher_marker_stale_ok_for_read_only_operator_query",
    )
}

pub(crate) fn read_state_fresh_json_projection_for_read_only_operator(
    state_dir: &Path,
    projection_name: &str,
) -> Option<String> {
    let path = projection_path(state_dir, projection_name);
    if path_is_symlink(&path) {
        return None;
    }
    let cache_modified = std::fs::metadata(&path).ok()?.modified().ok()?;
    let state_modified = latest_state_mutation_marker(state_dir).ok()?;
    if cache_modified <= state_modified {
        return None;
    }
    let cache_age = SystemTime::now().duration_since(cache_modified).ok()?;
    let body = read_json_without_following_symlinks(&path).ok()?;
    if !projection_task_snapshot_marker_matches(state_dir, &body) {
        return None;
    }
    annotate_projection_cache_with_status(
        &body,
        projection_name,
        cache_age,
        "state_fresh_structural_projection",
        "state_marker_fresh_structural_cache_ok_for_read_only_operator_query",
    )
    .or(Some(body))
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
    if cache_modified <= state_modified {
        return None;
    }
    if dependency_modified.is_some_and(|modified| cache_modified <= modified) {
        return None;
    }
    let cache_age = SystemTime::now().duration_since(cache_modified).ok()?;
    if cache_age > max_age {
        return None;
    }
    let body = read_json_without_following_symlinks(&path).ok()?;
    if !projection_task_snapshot_marker_matches(state_dir, &body) {
        return None;
    }
    annotate_recent_projection(&body, projection_name, cache_age, max_age).or(Some(body))
}

fn read_recent_json_projection_with_state_freshness_status(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
    status: &str,
    admissibility: &str,
) -> Option<String> {
    let path = projection_path(state_dir, projection_name);
    if path_is_symlink(&path) {
        return None;
    }
    let cache_modified = std::fs::metadata(&path).ok()?.modified().ok()?;
    let state_modified = latest_state_mutation_marker(state_dir).ok()?;
    if cache_modified <= state_modified {
        return None;
    }
    let cache_age = SystemTime::now().duration_since(cache_modified).ok()?;
    if cache_age > max_age {
        return None;
    }
    let body = read_json_without_following_symlinks(&path).ok()?;
    if !projection_task_snapshot_marker_matches(state_dir, &body) {
        return None;
    }
    annotate_recent_projection_with_status(
        &body,
        projection_name,
        cache_age,
        max_age,
        status,
        admissibility,
    )
    .or(Some(body))
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
    if !projection_task_snapshot_marker_matches(state_dir, &body) {
        return None;
    }
    if state_marker_newer
        && pass_projection_requires_recompute_without_operator_evidence(projection_name, &body)
    {
        return None;
    }
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

fn pass_projection_requires_recompute_without_operator_evidence(
    projection_name: &str,
    body: &str,
) -> bool {
    if !matches!(
        projection_name,
        "status-full-latest" | "taskflow-graph-summary-latest"
    ) {
        return false;
    }
    let Ok(payload) = serde_json::from_str::<serde_json::Value>(body) else {
        return true;
    };
    if payload.get("status").and_then(serde_json::Value::as_str) != Some("pass") {
        return false;
    }
    let Some(dependencies) = payload
        .get("projection_cache_dependencies")
        .and_then(serde_json::Value::as_object)
    else {
        return true;
    };
    !dependencies.keys().any(|key| key != "task_snapshot_marker")
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
    let mut payload = payload.clone();
    let source_event_cursor = task_snapshot_marker_value(state_dir);
    if let serde_json::Value::Object(object) = &mut payload {
        object.insert(
            "projection_cache_dependencies".to_string(),
            serde_json::json!({
                "task_snapshot_marker": source_event_cursor
            }),
        );
    }
    let content_hash = serde_json::to_string(&payload)
        .map(|body| stable_hash_hex(&body))
        .unwrap_or_default();
    if let serde_json::Value::Object(object) = &mut payload {
        object.insert(
            "projection_metadata".to_string(),
            serde_json::json!({
                "authority": "redb_operational_journal",
                "source_event_cursor": source_event_cursor,
                "content_hash": content_hash,
            }),
        );
    }
    let Ok(body) = serde_json::to_string_pretty(&payload) else {
        return;
    };
    if path_is_symlink(&path) {
        return;
    }
    let _ = write_json_without_following_symlinks(&path, &body);
}

pub(crate) fn projection_cache_control_payload(
    state_dir: &Path,
    projection_name: &str,
    mode: &str,
    hit: bool,
    freshness: &str,
    max_age: Duration,
    recompute_reason: Option<&str>,
) -> serde_json::Value {
    let mode = match mode {
        "auto" | "refresh" | "off" => mode,
        _ => "auto",
    };
    let path = projection_path(state_dir, projection_name);
    let (age_millis, cached_invalidation_tuple) = std::fs::metadata(&path)
        .ok()
        .and_then(|metadata| {
            let age_millis = metadata
                .modified()
                .ok()
                .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                .map(|age| age.as_millis());
            let cached_invalidation_tuple = read_json_without_following_symlinks(&path)
                .ok()
                .and_then(|body| {
                    serde_json::from_str::<serde_json::Value>(&body)
                        .ok()
                        .and_then(|payload| {
                            payload.get("projection_cache_dependencies").cloned()
                        })
                })
                .unwrap_or(serde_json::Value::Null);
            Some((age_millis, cached_invalidation_tuple))
        })
        .unwrap_or((None, serde_json::Value::Null));
    serde_json::json!({
        "status": if hit { "hit" } else if mode == "off" { "disabled" } else { "miss" },
        "projection_name": projection_name,
        "mode": mode,
        "hit": hit,
        "freshness": freshness,
        "age_millis": age_millis,
        "max_age_millis": max_age.as_millis(),
        "invalidation_tuple": {
            "task_snapshot_marker": task_snapshot_marker_value(state_dir),
        },
        "cached_invalidation_tuple": cached_invalidation_tuple,
        "recompute_reason": recompute_reason
            .map(serde_json::Value::from)
            .unwrap_or(serde_json::Value::Null),
        "read_allowed": mode == "auto",
        "write_allowed": mode != "off",
    })
}

pub(crate) fn annotate_projection_cache_control(
    state_dir: &Path,
    projection_name: &str,
    body: &str,
    mode: &str,
    hit: bool,
    freshness: &str,
    max_age: Duration,
    recompute_reason: Option<&str>,
) -> Option<String> {
    let mut payload = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let object = payload.as_object_mut()?;
    object.insert(
        "projection_cache".to_string(),
        projection_cache_control_payload(
            state_dir,
            projection_name,
            mode,
            hit,
            freshness,
            max_age,
            recompute_reason,
        ),
    );
    serde_json::to_string_pretty(&payload).ok()
}

pub(crate) fn projection_cache_recompute_reason(
    state_dir: &Path,
    projection_name: &str,
    max_age: Duration,
) -> String {
    let path = projection_path(state_dir, projection_name);
    if path_is_symlink(&path) {
        return "cache_symlink_rejected".to_string();
    }
    let Ok(metadata) = std::fs::metadata(&path) else {
        return "cache_missing".to_string();
    };
    let Ok(cache_modified) = metadata.modified() else {
        return "cache_mtime_unreadable".to_string();
    };
    if SystemTime::now()
        .duration_since(cache_modified)
        .ok()
        .is_some_and(|age| age > max_age)
    {
        return "max_age_exceeded".to_string();
    }
    if latest_state_mutation_marker(state_dir)
        .ok()
        .is_some_and(|modified| cache_modified <= modified)
    {
        return "state_marker_newer".to_string();
    }
    if current_operator_dependency_mutation_marker(state_dir)
        .is_some_and(|modified| cache_modified <= modified)
    {
        return "dependency_marker_newer".to_string();
    }
    let Ok(body) = read_json_without_following_symlinks(&path) else {
        return "cache_unreadable".to_string();
    };
    if serde_json::from_str::<serde_json::Value>(&body).is_err() {
        return "cache_invalid_json".to_string();
    }
    if !projection_task_snapshot_marker_matches(state_dir, &body) {
        return "invalidation_tuple_mismatch".to_string();
    }
    "cache_read_rejected".to_string()
}

pub(crate) fn write_runtime_continuation_binding_overlay(
    state_dir: &Path,
    binding: &crate::state_store::RunGraphContinuationBinding,
) {
    let payload = serde_json::json!({
        "schema_version": "runtime-continuation-binding-overlay-v1",
        "task_snapshot_marker": task_snapshot_marker_value(state_dir),
        "binding": binding,
        "continuation_binding": {
            "status": binding.status,
            "continuation_allowed": binding.status == "bound",
            "continuation_required_now": false,
            "active_bounded_unit": binding.active_bounded_unit,
            "binding_source": binding.binding_source,
            "why_this_unit": binding.why_this_unit,
            "primary_path": binding.primary_path,
            "sequential_vs_parallel_posture": binding.sequential_vs_parallel_posture,
            "pause_boundary_gate": "allowed_if_no_further_bound_work_is_evidenced",
            "ambiguity_reason": serde_json::Value::Null,
            "next_actions": []
        }
    });
    write_json_projection(
        state_dir,
        RUNTIME_CONTINUATION_BINDING_OVERLAY_PROJECTION_NAME,
        &payload,
    );
}

pub(crate) fn read_runtime_continuation_binding_overlay(
    state_dir: &Path,
) -> Option<serde_json::Value> {
    read_runtime_continuation_binding_overlay_after(state_dir, None)
}

pub(crate) fn read_runtime_continuation_binding_overlay_newer_than_projection(
    state_dir: &Path,
    projection_name: &str,
) -> Option<serde_json::Value> {
    let projection_modified = std::fs::metadata(projection_path(state_dir, projection_name))
        .ok()?
        .modified()
        .ok()?;
    read_runtime_continuation_binding_overlay_after(state_dir, Some(projection_modified))
}

fn read_runtime_continuation_binding_overlay_after(
    state_dir: &Path,
    minimum_modified: Option<SystemTime>,
) -> Option<serde_json::Value> {
    let path = projection_path(
        state_dir,
        RUNTIME_CONTINUATION_BINDING_OVERLAY_PROJECTION_NAME,
    );
    if path_is_symlink(&path) {
        return None;
    }
    let overlay_modified = std::fs::metadata(&path).ok()?.modified().ok()?;
    if latest_state_mutation_marker(state_dir)
        .ok()
        .is_some_and(|modified| overlay_modified <= modified)
    {
        return None;
    }
    if current_operator_dependency_mutation_marker(state_dir)
        .is_some_and(|modified| overlay_modified <= modified)
    {
        return None;
    }
    if minimum_modified.is_some_and(|modified| overlay_modified <= modified) {
        return None;
    }
    let payload = serde_json::from_str::<serde_json::Value>(
        &read_json_without_following_symlinks(&path).ok()?,
    )
    .ok()?;
    if payload
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        != Some("runtime-continuation-binding-overlay-v1")
    {
        return None;
    }
    if payload
        .get("task_snapshot_marker")
        .cloned()
        .unwrap_or_default()
        != task_snapshot_marker_value(state_dir)
    {
        return None;
    }
    Some(payload)
}

pub(crate) fn apply_runtime_continuation_binding_overlay_to_payload(
    state_dir: &Path,
    cached: &str,
    overlay: &serde_json::Value,
) -> Option<String> {
    apply_runtime_continuation_binding_overlay_to_payload_with_cache_status(
        state_dir,
        cached,
        overlay,
        "state_marker_stale_recent_projection_with_runtime_continuation_overlay",
        "cached_structural_projection_with_validated_continuation_binding_overlay",
    )
}

pub(crate) fn apply_runtime_continuation_binding_overlay_to_fresh_payload(
    state_dir: &Path,
    cached: &str,
    overlay: &serde_json::Value,
) -> Option<String> {
    apply_runtime_continuation_binding_overlay_to_payload_with_cache_status(
        state_dir,
        cached,
        overlay,
        "state_marker_fresh_projection_with_runtime_continuation_overlay",
        "fresh_cached_structural_projection_with_validated_continuation_binding_overlay",
    )
}

fn apply_runtime_continuation_binding_overlay_to_payload_with_cache_status(
    state_dir: &Path,
    cached: &str,
    overlay: &serde_json::Value,
    cache_status: &str,
    freshness_contract: &str,
) -> Option<String> {
    let mut payload = serde_json::from_str::<serde_json::Value>(cached).ok()?;
    if payload
        .get("projection_cache_dependencies")
        .and_then(|dependencies| dependencies.get("task_snapshot_marker"))
        .cloned()
        != Some(task_snapshot_marker_value(state_dir))
    {
        return None;
    }
    let continuation_binding = overlay.get("continuation_binding")?.clone();
    let binding = overlay.get("binding").cloned();
    let object = payload.as_object_mut()?;
    clear_stale_root_session_write_guard_after_continuation_overlay(object, &continuation_binding);

    object.insert(
        "projection_cache".to_string(),
        serde_json::json!({
            "status": cache_status,
            "projection_name": object
                .get("surface")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("operator_projection"),
            "freshness_contract": freshness_contract
        }),
    );

    if let Some(init) = object
        .get_mut("init")
        .and_then(serde_json::Value::as_object_mut)
    {
        init.insert(
            "continuation_binding".to_string(),
            continuation_binding.clone(),
        );
    }

    object.insert(
        "continuation_binding".to_string(),
        continuation_binding.clone(),
    );
    let active_bounded_unit = continuation_binding
        .get("active_bounded_unit")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    object.insert(
        "active_bounded_unit".to_string(),
        active_bounded_unit.clone(),
    );
    object.insert(
        "active_step".to_string(),
        active_bounded_unit
            .get("active_step")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "active_parent_task".to_string(),
        active_bounded_unit
            .get("active_parent_task")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "active_epic".to_string(),
        active_bounded_unit
            .get("active_epic")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "why_this_unit".to_string(),
        continuation_binding
            .get("why_this_unit")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    object.insert(
        "sequential_vs_parallel_posture".to_string(),
        continuation_binding
            .get("sequential_vs_parallel_posture")
            .cloned()
            .unwrap_or(serde_json::Value::Null),
    );
    if let Some(binding) = binding {
        object.insert("explicit_continuation_binding".to_string(), binding);
    }
    serde_json::to_string_pretty(&payload).ok()
}

fn clear_stale_root_session_write_guard_after_continuation_overlay(
    object: &mut serde_json::Map<String, serde_json::Value>,
    continuation_binding: &serde_json::Value,
) {
    let Some(active_unit) = continuation_binding
        .get("active_bounded_unit")
        .and_then(serde_json::Value::as_object)
    else {
        return;
    };
    let active_run_id = active_unit
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let active_task_id = active_unit
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let cached_unit = object
        .get("active_bounded_unit")
        .or_else(|| {
            object
                .get("continuation_binding")
                .and_then(|binding| binding.get("active_bounded_unit"))
        })
        .and_then(serde_json::Value::as_object);
    let cached_run_id = cached_unit
        .and_then(|unit| unit.get("run_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let cached_task_id = cached_unit
        .and_then(|unit| unit.get("task_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let cached_run_graph_task_id = object
        .get("latest_run_graph_status")
        .and_then(|status| status.get("task_id"))
        .or_else(|| {
            object
                .get("latest_run_graph_gate")
                .and_then(|gate| gate.get("task_id"))
        })
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let Some(guard) = object
        .get_mut("root_session_write_guard")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };

    let guard_run_id = guard
        .get("run_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let guard_task_id = guard
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let guard_marks_stale_run_graph_task = guard
        .get("latest_run_graph_task_stale")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let stale_guard = guard_run_id
        .is_some_and(|run_id| active_run_id.is_some_and(|active| run_id != active))
        || guard_task_id
            .is_some_and(|task_id| active_task_id.is_some_and(|active| task_id != active))
        || cached_run_id
            .as_deref()
            .is_some_and(|run_id| active_run_id.is_some_and(|active| run_id != active))
        || cached_task_id
            .as_deref()
            .is_some_and(|task_id| active_task_id.is_some_and(|active| task_id != active))
        || (guard_marks_stale_run_graph_task
            && cached_run_graph_task_id
                .as_deref()
                .is_some_and(|task_id| active_task_id.is_some_and(|active| task_id != active)));
    if !stale_guard {
        return;
    }

    guard.insert(
        "status".to_string(),
        serde_json::Value::String("blocked_by_default".to_string()),
    );
    guard.insert(
        "reason".to_string(),
        serde_json::Value::String("stale_continuation_overlay_write_guard".to_string()),
    );
    guard.insert(
        "root_local_write_allowed".to_string(),
        serde_json::Value::Bool(false),
    );
    guard.insert(
        "root_local_write_allowed_for_only_these_paths".to_string(),
        serde_json::json!([]),
    );
    guard.insert("run_id".to_string(), serde_json::Value::Null);
    guard.insert("task_id".to_string(), serde_json::Value::Null);
    guard.insert(
        "latest_lane_status".to_string(),
        serde_json::Value::String("stale_continuation_overlay_blocked".to_string()),
    );
    guard.insert(
        "local_exception_takeover_state".to_string(),
        serde_json::Value::String("stale_task_blocked".to_string()),
    );
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

fn annotate_projection_cache_with_status(
    body: &str,
    projection_name: &str,
    cache_age: Duration,
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
            "max_age_millis": null,
            "freshness_contract": freshness_contract,
        }),
    );
    serde_json::to_string_pretty(&payload).ok()
}

fn stable_hash_hex(input: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn latest_state_mutation_marker(state_dir: &Path) -> std::io::Result<SystemTime> {
    let mut latest = SystemTime::UNIX_EPOCH;
    for entry in std::fs::read_dir(state_dir)? {
        let entry = entry?;
        if state_root_entry_is_projection_cache_noise(&entry) {
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

fn state_root_entry_is_projection_cache_noise(entry: &std::fs::DirEntry) -> bool {
    let name = entry.file_name();
    let Some(name) = name.to_str() else {
        return false;
    };
    if matches!(
        name,
        "operator-projections" | "LOCK" | ".vida-authoritative-open.guard"
    ) {
        return true;
    }
    if !matches!(name, "manifest" | "sstables" | "vlog" | "wal") {
        return false;
    }
    entry
        .file_type()
        .map(|file_type| file_type.is_dir())
        .unwrap_or(false)
}

pub(crate) fn current_launcher_mutation_marker() -> Option<SystemTime> {
    std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
}

fn current_operator_dependency_mutation_marker(state_dir: &Path) -> Option<SystemTime> {
    [
        current_launcher_mutation_marker(),
        project_config_mutation_marker(state_dir),
    ]
    .into_iter()
    .flatten()
    .max()
}

fn project_config_mutation_marker(state_dir: &Path) -> Option<SystemTime> {
    state_dir
        .ancestors()
        .map(|ancestor| ancestor.join("vida.config.yaml"))
        .find(|path| path.is_file())
        .and_then(|path| std::fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
}

fn task_snapshot_marker_value(state_dir: &Path) -> serde_json::Value {
    std::fs::read_to_string(
        crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(
            state_dir,
        ),
    )
    .ok()
    .map(|value| serde_json::Value::String(value.trim().to_string()))
    .unwrap_or(serde_json::Value::Null)
}

fn projection_task_snapshot_marker_matches(state_dir: &Path, body: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|payload| {
            payload
                .get("projection_cache_dependencies")
                .and_then(|dependencies| dependencies.get("task_snapshot_marker"))
                .cloned()
        })
        == Some(task_snapshot_marker_value(state_dir))
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
        annotate_projection_cache_control, projection_cache_control_payload,
        projection_cache_recompute_reason,
        projection_path, read_fresh_json_projection,
        read_fresh_json_projection_with_dependency_marker,
        read_launcher_stale_state_fresh_recent_json_projection, read_recent_json_projection,
        read_recent_json_projection_with_dependency_marker,
        read_runtime_continuation_binding_overlay,
        read_runtime_continuation_binding_overlay_newer_than_projection,
        read_state_fresh_json_projection, read_state_fresh_json_projection_for_read_only_operator,
        read_state_recent_json_projection, read_state_stale_recent_json_projection,
        sanitize_projection_component, touch_state_mutation_marker, write_json_projection,
        write_runtime_continuation_binding_overlay,
    };
    use std::{fs, time::Duration};

    #[test]
    fn projection_component_sanitizer_preserves_projection_name_semantics() {
        assert_eq!(
            sanitize_projection_component("run:1/impl review", "none", 120),
            "run-1-impl-review"
        );
        assert_eq!(sanitize_projection_component("", "unknown", 160), "unknown");
        assert_eq!(sanitize_projection_component("!!!", "unknown", 160), "---");
        assert_eq!(
            sanitize_projection_component(&"a".repeat(121), "none", 120).len(),
            120
        );
    }

    #[test]
    fn projection_cache_control_reports_policy_age_and_invalidation_tuple() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-control-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker = crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(&root);
        fs::write(&marker, "cache-marker-1").expect("task marker should write");
        write_json_projection(
            &root,
            "agent-dispatch-next-control",
            &serde_json::json!({"status": "pass"}),
        );

        let report = projection_cache_control_payload(
            &root,
            "agent-dispatch-next-control",
            "auto",
            true,
            "fresh",
            Duration::from_secs(300),
            None,
        );
        assert_eq!(report["mode"], "auto");
        assert_eq!(report["hit"], true);
        assert_eq!(report["freshness"], "fresh");
        assert!(report["age_millis"].is_number());
        assert_eq!(report["max_age_millis"], 300_000);
        assert_eq!(
            report["invalidation_tuple"]["task_snapshot_marker"],
            "cache-marker-1"
        );
        assert_eq!(report["read_allowed"], true);
        assert_eq!(report["write_allowed"], true);

        let body = fs::read_to_string(projection_path(&root, "agent-dispatch-next-control"))
            .expect("projection should be readable");
        let annotated = annotate_projection_cache_control(
            &root,
            "agent-dispatch-next-control",
            &body,
            "refresh",
            false,
            "refreshed",
            Duration::from_secs(42),
            Some("refresh_requested"),
        )
        .expect("cache control should annotate valid json");
        let annotated: serde_json::Value =
            serde_json::from_str(&annotated).expect("annotated projection should remain json");
        assert_eq!(annotated["projection_cache"]["mode"], "refresh");
        assert_eq!(annotated["projection_cache"]["recompute_reason"], "refresh_requested");
        assert_eq!(annotated["projection_cache"]["max_age_millis"], 42_000);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn projection_cache_recompute_reason_is_explicit_for_missing_and_age_bound_cache() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-reason-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        assert_eq!(
            projection_cache_recompute_reason(
                &root,
                "agent-dispatch-next-missing",
                Duration::from_secs(300)
            ),
            "cache_missing"
        );
        write_json_projection(
            &root,
            "agent-dispatch-next-old",
            &serde_json::json!({"status": "pass"}),
        );
        assert_eq!(
            projection_cache_recompute_reason(
                &root,
                "agent-dispatch-next-old",
                Duration::ZERO
            ),
            "max_age_exceeded"
        );
        let _ = fs::remove_dir_all(root);
    }

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
        std::thread::sleep(Duration::from_millis(10));
        let payload = serde_json::json!({"status": "pass", "cached": true});
        write_json_projection(&root, "status-summary-latest", &payload);
        let cached = read_fresh_json_projection(&root, "status-summary-latest")
            .expect("fresh projection should read");
        let cached: serde_json::Value =
            serde_json::from_str(&cached).expect("projection should remain json");
        assert!(
            cached
                .get("projection_cache_dependencies")
                .and_then(|dependencies| dependencies.get("task_snapshot_marker"))
                .is_some()
        );

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
        std::thread::sleep(Duration::from_millis(10));
        let payload = serde_json::json!({"status": "pass", "cached": true});
        write_json_projection(&root, "doctor-latest", &payload);
        assert!(
            read_fresh_json_projection_with_dependency_marker(
                &root,
                "doctor-latest",
                Some(std::time::SystemTime::UNIX_EPOCH)
            )
            .is_some()
        );

        std::thread::sleep(Duration::from_millis(10));
        let dependency_modified = std::time::SystemTime::now();
        assert!(
            read_fresh_json_projection_with_dependency_marker(
                &root,
                "doctor-latest",
                Some(dependency_modified)
            )
            .is_none()
        );
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
        std::thread::sleep(Duration::from_millis(10));
        let payload = serde_json::json!({"surface": "vida doctor", "status": "pass"});
        write_json_projection(&root, "doctor-summary-latest", &payload);

        std::thread::sleep(Duration::from_millis(10));
        let dependency_modified = std::time::SystemTime::now();
        assert!(
            read_fresh_json_projection_with_dependency_marker(
                &root,
                "doctor-summary-latest",
                Some(dependency_modified),
            )
            .is_none()
        );
        assert!(read_state_fresh_json_projection(&root, "doctor-summary-latest").is_some());
        assert!(
            read_state_recent_json_projection(
                &root,
                "doctor-summary-latest",
                Duration::from_secs(60)
            )
            .is_some()
        );
        let launcher_stale = read_launcher_stale_state_fresh_recent_json_projection(
            &root,
            "doctor-summary-latest",
            Duration::from_secs(60),
        )
        .expect("launcher-stale state-fresh projection should be admissible");
        let launcher_stale: serde_json::Value =
            serde_json::from_str(&launcher_stale).expect("annotated cache should parse");
        assert_eq!(
            launcher_stale["projection_cache"]["status"],
            "launcher_stale_recent_projection"
        );
        assert_eq!(
            launcher_stale["projection_cache"]["freshness_contract"],
            "bounded_launcher_marker_stale_ok_for_read_only_operator_query"
        );
        let state_fresh =
            read_state_fresh_json_projection_for_read_only_operator(&root, "doctor-summary-latest")
                .expect(
                    "state-fresh projection should remain admissible without wall-clock expiry",
                );
        let state_fresh: serde_json::Value =
            serde_json::from_str(&state_fresh).expect("state-fresh cache should parse");
        assert_eq!(
            state_fresh["projection_cache"]["status"],
            "state_fresh_structural_projection"
        );
        assert_eq!(
            state_fresh["projection_cache"]["freshness_contract"],
            "state_marker_fresh_structural_cache_ok_for_read_only_operator_query"
        );
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
    fn json_projection_cache_invalidates_when_project_config_changes() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-config-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let payload = serde_json::json!({"status": "pass", "cached": true});
        write_json_projection(&root, "status-full-latest", &payload);
        assert!(read_fresh_json_projection(&root, "status-full-latest").is_some());

        std::thread::sleep(Duration::from_millis(10));
        fs::write(
            root.join("vida.config.yaml"),
            "agent_system:\n  subagents: {}\n",
        )
        .expect("project config should write");
        assert!(read_fresh_json_projection(&root, "status-full-latest").is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn json_projection_cache_ignores_storage_engine_mtime_noise() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-storage-noise-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("sstables")).expect("sstables dir should be writable");
        fs::create_dir_all(root.join("vlog")).expect("vlog dir should be writable");
        fs::create_dir_all(root.join("manifest")).expect("manifest dir should be writable");
        fs::create_dir_all(root.join("wal")).expect("wal dir should be writable");
        let payload = serde_json::json!({"status": "pass", "cached": true});
        write_json_projection(&root, "doctor-full-latest", &payload);

        std::thread::sleep(Duration::from_millis(10));
        fs::write(root.join("sstables").join("read-noise"), "engine")
            .expect("storage engine mtime noise should write");
        fs::write(root.join("vlog").join("read-noise"), "engine")
            .expect("storage engine mtime noise should write");
        fs::write(root.join("manifest").join("read-noise"), "engine")
            .expect("manifest directory mtime noise should write");
        fs::write(root.join("wal").join("read-noise"), "engine")
            .expect("wal directory mtime noise should write");

        assert!(read_fresh_json_projection(&root, "doctor-full-latest").is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn json_projection_cache_invalidates_when_state_marker_mtime_ties_cache() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-marker-tie-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker = root.join("manifest");
        let projection = projection_path(&root, "taskflow-graph-summary-latest");
        fs::write(&marker, "state").expect("marker should be writable");
        let payload = serde_json::json!({"status": "blocked", "cached": true});
        write_json_projection(&root, "taskflow-graph-summary-latest", &payload);
        let marker_modified = fs::metadata(&marker)
            .and_then(|metadata| metadata.modified())
            .expect("marker mtime should load");
        let projection_modified = fs::metadata(&projection)
            .and_then(|metadata| metadata.modified())
            .expect("projection mtime should load");
        if projection_modified != marker_modified {
            fs::write(&marker, "state-touched").expect("marker should be updateable");
        }

        assert!(
            read_fresh_json_projection(&root, "taskflow-graph-summary-latest").is_none(),
            "equal or newer state marker must invalidate cached projections"
        );
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
        assert!(
            read_state_recent_json_projection(
                &root,
                "doctor-summary-latest",
                Duration::from_secs(60)
            )
            .is_none()
        );
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
    fn state_stale_recent_status_pass_cache_with_only_task_marker_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-status-pass-state-stale-{}-{}",
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
        assert!(
            read_state_stale_recent_json_projection(
                &root,
                "status-full-latest",
                Duration::from_secs(60)
            )
            .is_none(),
            "state-stale status pass cache with only task marker must recompute"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn state_stale_recent_graph_summary_pass_cache_with_only_task_marker_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-graph-pass-state-stale-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let payload =
            serde_json::json!({"surface": "vida taskflow graph-summary", "status": "pass"});
        write_json_projection(&root, "taskflow-graph-summary-latest", &payload);

        std::thread::sleep(Duration::from_millis(10));
        touch_state_mutation_marker(&root);
        assert!(
            read_state_stale_recent_json_projection(
                &root,
                "taskflow-graph-summary-latest",
                Duration::from_secs(60)
            )
            .is_none(),
            "state-stale graph-summary pass cache with only task marker must recompute"
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
        assert!(
            read_recent_json_projection_with_dependency_marker(
                &root,
                "doctor-full-latest",
                Duration::from_secs(60),
                Some(dependency_modified),
            )
            .is_none()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_continuation_overlay_rejects_state_mutation_newer_than_overlay() {
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-continuation-overlay-state-marker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker =
            crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(
                &root,
            );
        fs::write(&marker, "task-marker-1").expect("task marker should write");
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "run-overlay".to_string(),
            task_id: "task-overlay".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "run_id": "run-overlay",
                "task_id": "task-overlay",
                "task_status": "open"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "test overlay".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("task-overlay".to_string()),
            recorded_at: "2026-05-25T00:00:00Z".to_string(),
        };

        std::thread::sleep(Duration::from_millis(10));
        write_runtime_continuation_binding_overlay(&root, &binding);
        assert!(
            read_runtime_continuation_binding_overlay(&root).is_some(),
            "fresh overlay should be admitted before later state mutation"
        );

        std::thread::sleep(Duration::from_millis(10));
        touch_state_mutation_marker(&root);
        assert!(
            read_runtime_continuation_binding_overlay(&root).is_none(),
            "state mutation newer than the overlay must fail closed"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_continuation_overlay_for_fresh_projection_requires_newer_overlay() {
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-continuation-overlay-projection-marker-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker =
            crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(
                &root,
            );
        fs::write(&marker, "task-marker-1").expect("task marker should write");
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "run-overlay".to_string(),
            task_id: "task-overlay".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "run_id": "run-overlay",
                "task_id": "task-overlay",
                "task_status": "open"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "test overlay".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("task-overlay".to_string()),
            recorded_at: "2026-05-25T00:00:00Z".to_string(),
        };

        std::thread::sleep(Duration::from_millis(10));
        write_runtime_continuation_binding_overlay(&root, &binding);
        std::thread::sleep(Duration::from_millis(10));
        write_json_projection(
            &root,
            "orchestrator-init-summary-latest",
            &serde_json::json!({
                "surface": "vida orchestrator-init",
                "status": "ready_enough_for_normal_work"
            }),
        );
        assert!(
            read_runtime_continuation_binding_overlay_newer_than_projection(
                &root,
                "orchestrator-init-summary-latest"
            )
            .is_none(),
            "fresh projection newer than the overlay must not be overwritten"
        );

        std::thread::sleep(Duration::from_millis(10));
        write_runtime_continuation_binding_overlay(&root, &binding);
        assert!(
            read_runtime_continuation_binding_overlay_newer_than_projection(
                &root,
                "orchestrator-init-summary-latest"
            )
            .is_some(),
            "overlay newer than the fresh projection may be applied"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_continuation_overlay_validates_task_snapshot_marker() {
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-continuation-overlay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker =
            crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(
                &root,
            );
        fs::write(&marker, "task-marker-1").expect("task marker should write");
        std::thread::sleep(Duration::from_millis(10));
        let binding = crate::state_store::RunGraphContinuationBinding {
            run_id: "run-overlay".to_string(),
            task_id: "task-overlay".to_string(),
            status: "bound".to_string(),
            active_bounded_unit: serde_json::json!({
                "kind": "task_graph_task",
                "run_id": "run-overlay",
                "task_id": "task-overlay",
                "task_status": "open"
            }),
            binding_source: "explicit_continuation_bind_task".to_string(),
            why_this_unit: "test overlay".to_string(),
            primary_path: "normal_delivery_path".to_string(),
            sequential_vs_parallel_posture: "sequential_only_explicit_task_bound".to_string(),
            request_text: Some("task-overlay".to_string()),
            recorded_at: "2026-05-25T00:00:00Z".to_string(),
        };

        write_runtime_continuation_binding_overlay(&root, &binding);
        let overlay = read_runtime_continuation_binding_overlay(&root)
            .expect("matching task marker should admit overlay");
        assert_eq!(
            overlay["continuation_binding"]["active_bounded_unit"]["task_id"],
            "task-overlay"
        );

        fs::write(&marker, "task-marker-2").expect("task marker should update");
        assert!(
            read_runtime_continuation_binding_overlay(&root).is_none(),
            "task snapshot marker drift must fail closed"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn json_projection_cache_invalidates_when_task_snapshot_marker_changes() {
        let root = std::env::temp_dir().join(format!(
            "vida-operator-projection-cache-task-snapshot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker =
            crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(
                &root,
            );
        fs::write(&marker, "task-marker-1").expect("task marker should write");
        std::thread::sleep(Duration::from_millis(10));
        let payload = serde_json::json!({"status": "blocked", "cached": true});
        write_json_projection(&root, "agent-dispatch-next-latest", &payload);
        write_json_projection(&root, "status-full-latest", &payload);
        assert!(read_fresh_json_projection(&root, "agent-dispatch-next-latest").is_some());
        assert!(read_fresh_json_projection(&root, "status-full-latest").is_some());
        let cached = serde_json::from_str::<serde_json::Value>(
            &fs::read_to_string(
                root.join("operator-projections")
                    .join("status-full-latest.json"),
            )
            .expect("projection should be readable"),
        )
        .expect("projection should be json");
        assert_eq!(
            cached
                .get("projection_metadata")
                .and_then(|metadata| metadata.get("authority"))
                .and_then(serde_json::Value::as_str),
            Some("redb_operational_journal")
        );
        assert_eq!(
            cached
                .get("projection_metadata")
                .and_then(|metadata| metadata.get("source_event_cursor"))
                .and_then(serde_json::Value::as_str),
            Some("task-marker-1")
        );
        assert!(
            cached
                .get("projection_metadata")
                .and_then(|metadata| metadata.get("content_hash"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|hash| !hash.is_empty())
        );

        fs::write(&marker, "task-marker-2").expect("task marker should update");
        assert!(
            read_fresh_json_projection(&root, "agent-dispatch-next-latest").is_none(),
            "task snapshot marker drift must invalidate structural operator projections"
        );
        assert!(
            read_fresh_json_projection(&root, "status-full-latest").is_none(),
            "task snapshot marker drift must invalidate status projections"
        );
        assert!(
            read_state_stale_recent_json_projection(
                &root,
                "status-full-latest",
                Duration::from_secs(300)
            )
            .is_none(),
            "state-stale status cache reuse must still reject task snapshot drift"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_continuation_overlay_updates_operator_payload() {
        let overlay = serde_json::json!({
            "binding": {
                "run_id": "run-overlay",
                "task_id": "task-overlay",
                "status": "bound",
                "active_bounded_unit": {
                    "kind": "task_graph_task",
                    "run_id": "run-overlay",
                    "task_id": "task-overlay",
                    "active_step": {
                        "task_id": "step-overlay"
                    },
                    "active_parent_task": {
                        "task_id": "task-overlay"
                    },
                    "active_epic": {
                        "task_id": "epic-overlay"
                    }
                },
                "binding_source": "explicit_continuation_bind_task",
                "why_this_unit": "test overlay",
                "primary_path": "normal_delivery_path",
                "sequential_vs_parallel_posture": "sequential_only_explicit_task_bound",
                "request_text": "task-overlay",
                "recorded_at": "2026-05-25T00:00:00Z"
            },
            "continuation_binding": {
                "status": "bound",
                "continuation_allowed": true,
                "continuation_required_now": false,
                "active_bounded_unit": {
                    "kind": "task_graph_task",
                    "run_id": "run-overlay",
                    "task_id": "task-overlay",
                    "active_step": {
                        "task_id": "step-overlay"
                    },
                    "active_parent_task": {
                        "task_id": "task-overlay"
                    },
                    "active_epic": {
                        "task_id": "epic-overlay"
                    }
                },
                "binding_source": "explicit_continuation_bind_task",
                "why_this_unit": "test overlay",
                "primary_path": "normal_delivery_path",
                "sequential_vs_parallel_posture": "sequential_only_explicit_task_bound",
                "pause_boundary_gate": "allowed_if_no_further_bound_work_is_evidenced",
                "ambiguity_reason": null,
                "next_actions": []
            }
        });
        let cached = serde_json::json!({
            "surface": "vida orchestrator-init",
            "status": "ready_enough_for_normal_work",
            "init": {
                "continuation_binding": {
                    "status": "ambiguous",
                    "active_bounded_unit": null
                }
            }
        })
        .to_string();

        let root = std::env::temp_dir().join(format!(
            "vida-runtime-continuation-payload-overlay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker =
            crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(
                &root,
            );
        fs::write(&marker, "task-marker-1").expect("task marker should write");
        write_json_projection(
            &root,
            "orchestrator-init-summary-latest",
            &serde_json::from_str::<serde_json::Value>(&cached).expect("cached payload"),
        );
        let cached = read_state_stale_recent_json_projection(
            &root,
            "orchestrator-init-summary-latest",
            Duration::from_secs(60),
        )
        .expect("projection should be readable");

        let rendered =
            super::apply_runtime_continuation_binding_overlay_to_payload(&root, &cached, &overlay)
                .expect("overlay should update payload");
        let rendered: serde_json::Value =
            serde_json::from_str(&rendered).expect("rendered overlay should parse");

        assert_eq!(
            rendered["init"]["continuation_binding"]["active_bounded_unit"]["task_id"],
            "task-overlay"
        );
        assert_eq!(rendered["active_bounded_unit"]["task_id"], "task-overlay");
        assert_eq!(rendered["active_step"]["task_id"], "step-overlay");
        assert_eq!(rendered["active_parent_task"]["task_id"], "task-overlay");
        assert_eq!(rendered["active_epic"]["task_id"], "epic-overlay");
        assert_eq!(
            rendered["projection_cache"]["status"],
            "state_marker_stale_recent_projection_with_runtime_continuation_overlay"
        );

        fs::write(&marker, "task-marker-2").expect("task marker should update");
        assert!(
            super::apply_runtime_continuation_binding_overlay_to_payload(&root, &cached, &overlay)
                .is_none(),
            "structural projection with stale task marker must fail closed"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn runtime_continuation_overlay_updates_fresh_operator_payload() {
        let overlay = serde_json::json!({
            "binding": {
                "run_id": "run-overlay",
                "task_id": "task-overlay",
                "status": "bound",
                "active_bounded_unit": {
                    "kind": "task_graph_task",
                    "run_id": "run-overlay",
                    "task_id": "task-overlay"
                },
                "binding_source": "explicit_continuation_bind_task",
                "why_this_unit": "test overlay",
                "primary_path": "normal_delivery_path",
                "sequential_vs_parallel_posture": "sequential_only_explicit_task_bound",
                "request_text": "task-overlay",
                "recorded_at": "2026-05-25T00:00:00Z"
            },
            "continuation_binding": {
                "status": "bound",
                "continuation_allowed": true,
                "continuation_required_now": false,
                "active_bounded_unit": {
                    "kind": "task_graph_task",
                    "run_id": "run-overlay",
                    "task_id": "task-overlay"
                },
                "binding_source": "explicit_continuation_bind_task",
                "why_this_unit": "test overlay",
                "primary_path": "normal_delivery_path",
                "sequential_vs_parallel_posture": "sequential_only_explicit_task_bound",
                "pause_boundary_gate": "allowed_if_no_further_bound_work_is_evidenced",
                "ambiguity_reason": null,
                "next_actions": []
            }
        });
        let root = std::env::temp_dir().join(format!(
            "vida-runtime-continuation-fresh-payload-overlay-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("test clock should support unique ids")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("state root should be writable");
        let marker =
            crate::state_store::StateStore::canonical_task_snapshot_marker_path_for_state_root(
                &root,
            );
        fs::write(&marker, "task-marker-1").expect("task marker should write");
        std::thread::sleep(Duration::from_millis(10));
        write_json_projection(
            &root,
            "orchestrator-init-summary-latest",
            &serde_json::json!({
                "surface": "vida orchestrator-init",
                "status": "ready_enough_for_normal_work",
                "init": {
                    "continuation_binding": {
                        "status": "ambiguous",
                        "active_bounded_unit": null
                    }
                }
            }),
        );
        let cached = read_fresh_json_projection(&root, "orchestrator-init-summary-latest")
            .expect("fresh projection should be readable");

        let rendered = super::apply_runtime_continuation_binding_overlay_to_fresh_payload(
            &root, &cached, &overlay,
        )
        .expect("fresh overlay should update payload");
        let rendered: serde_json::Value =
            serde_json::from_str(&rendered).expect("rendered overlay should parse");

        assert_eq!(
            rendered["init"]["continuation_binding"]["active_bounded_unit"]["task_id"],
            "task-overlay"
        );
        assert_eq!(rendered["active_bounded_unit"]["task_id"], "task-overlay");
        assert_eq!(
            rendered["projection_cache"]["status"],
            "state_marker_fresh_projection_with_runtime_continuation_overlay"
        );
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
        assert!(
            std::fs::symlink_metadata(&marker_link)
                .expect("marker symlink should remain readable")
                .file_type()
                .is_symlink()
        );

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
