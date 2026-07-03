use std::path::{Path, PathBuf};

pub(crate) const EXCEPTION_TAKEOVER_METADATA_DIR: &str = "lane-exception-path-metadata";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ExceptionTakeoverMetadata {
    #[serde(default)]
    pub(crate) run_id: Option<String>,
    #[serde(default)]
    pub(crate) dispatch_target: Option<String>,
    #[serde(default)]
    pub(crate) dispatch_packet_path: Option<String>,
    #[serde(default)]
    pub(crate) source_exception_path_receipt_id: Option<String>,
    #[serde(default)]
    pub(crate) reason_class: String,
    #[serde(default)]
    pub(crate) active_bounded_unit: String,
    #[serde(default)]
    pub(crate) owned_write_scope: Vec<String>,
    #[serde(default)]
    pub(crate) why_delegated_or_rerouted_path_is_not_currently_lawful: String,
    #[serde(default)]
    pub(crate) why_local_write_is_the_smallest_safe_bounded_workaround: String,
    #[serde(default)]
    pub(crate) return_to_normal_posture_condition: String,
    #[serde(default)]
    pub(crate) verification_plan: Vec<String>,
    #[serde(default)]
    pub(crate) recorded_at: String,
}

impl ExceptionTakeoverMetadata {
    pub(crate) fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("reason_class", self.reason_class.trim()),
            ("active_bounded_unit", self.active_bounded_unit.trim()),
            (
                "why_delegated_or_rerouted_path_is_not_currently_lawful",
                self.why_delegated_or_rerouted_path_is_not_currently_lawful
                    .trim(),
            ),
            (
                "why_local_write_is_the_smallest_safe_bounded_workaround",
                self.why_local_write_is_the_smallest_safe_bounded_workaround
                    .trim(),
            ),
            (
                "return_to_normal_posture_condition",
                self.return_to_normal_posture_condition.trim(),
            ),
            ("recorded_at", self.recorded_at.trim()),
        ] {
            if value.is_empty() {
                return Err(format!(
                    "exception takeover metadata field `{field}` must be non-empty"
                ));
            }
        }
        if self.owned_write_scope.is_empty()
            || self
                .owned_write_scope
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(
                "exception takeover metadata requires at least one non-empty `owned_write_scope` entry"
                    .to_string(),
            );
        }
        if self.verification_plan.is_empty()
            || self
                .verification_plan
                .iter()
                .any(|value| value.trim().is_empty())
        {
            return Err(
                "exception takeover metadata requires at least one non-empty `verification_plan` entry"
                    .to_string(),
            );
        }
        Ok(())
    }

    pub(crate) fn bind_to_receipt(
        &mut self,
        receipt: &crate::state_store::RunGraphDispatchReceipt,
    ) {
        self.run_id = Some(receipt.run_id.clone());
        self.dispatch_target = Some(receipt.dispatch_target.clone());
        self.dispatch_packet_path = receipt.dispatch_packet_path.clone();
        self.source_exception_path_receipt_id = receipt.exception_path_receipt_id.clone();
    }

    pub(crate) fn validate_for_receipt(
        &self,
        receipt: &crate::state_store::RunGraphDispatchReceipt,
    ) -> Result<(), String> {
        let run_id = self
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "exception takeover metadata is missing receipt-bound `run_id`; record a fresh exception takeover for the current lane before superseding".to_string()
            })?;
        if run_id != receipt.run_id {
            return Err(format!(
                "exception takeover metadata run_id `{run_id}` does not match current lane `{}`",
                receipt.run_id
            ));
        }

        let dispatch_target = self
            .dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "exception takeover metadata is missing receipt-bound `dispatch_target`; record a fresh exception takeover for the current lane before superseding".to_string()
            })?;
        if dispatch_target != receipt.dispatch_target {
            return Err(format!(
                "exception takeover metadata dispatch_target `{dispatch_target}` does not match current lane target `{}`",
                receipt.dispatch_target
            ));
        }

        let source_receipt_id = self
            .source_exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "exception takeover metadata is missing receipt-bound `source_exception_path_receipt_id`; record a fresh exception takeover for the current lane before superseding".to_string()
            })?;
        let current_exception_receipt = receipt
            .exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                "current lane receipt is missing exception_path_receipt_id; record exception takeover before superseding".to_string()
            })?;
        if source_receipt_id != current_exception_receipt {
            return Err(format!(
                "exception takeover metadata source receipt `{source_receipt_id}` does not match current exception receipt `{current_exception_receipt}`"
            ));
        }

        Ok(())
    }

    pub(crate) fn matches_summary(
        &self,
        summary: &crate::state_store::RunGraphDispatchReceiptSummary,
    ) -> bool {
        let run_id_matches = self
            .run_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value == summary.run_id);
        let target_matches = self
            .dispatch_target
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| value == summary.dispatch_target);
        let source_receipt_matches = self
            .source_exception_path_receipt_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some_and(|value| {
                summary
                    .exception_path_receipt_id
                    .as_deref()
                    .is_some_and(|summary_value| value == summary_value)
            });
        run_id_matches && target_matches && source_receipt_matches
    }

    pub(crate) fn owned_write_scope(&self) -> Vec<String> {
        self.owned_write_scope
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect()
    }

    pub(crate) fn owned_write_scope_slice(&self) -> &[String] {
        self.owned_write_scope.as_slice()
    }
}

pub(crate) fn metadata_dir(state_root: &Path) -> PathBuf {
    state_root.join(EXCEPTION_TAKEOVER_METADATA_DIR)
}

pub(crate) fn metadata_filename(run_id: &str) -> Result<String, String> {
    if run_id.is_empty() {
        return Err("Run id cannot be empty for exception takeover metadata.".to_string());
    }
    if !run_id
        .chars()
        .all(|value| value.is_ascii_alphanumeric() || value == '-' || value == '_')
    {
        return Err(format!(
            "Run id `{run_id}` contains unsupported characters for exception takeover metadata filename."
        ));
    }
    Ok(format!("{run_id}.json"))
}

pub(crate) fn metadata_path(state_root: &Path, run_id: &str) -> Result<PathBuf, String> {
    let file_name = metadata_filename(run_id)?;
    Ok(metadata_dir(state_root).join(file_name))
}

pub(crate) fn read_exception_takeover_metadata(
    state_root: &Path,
    run_id: &str,
) -> Result<Option<ExceptionTakeoverMetadata>, String> {
    let path = metadata_path(state_root, run_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "Failed to read persisted exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    let metadata: ExceptionTakeoverMetadata = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "Failed to decode persisted exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    Ok(Some(metadata))
}

pub(crate) fn read_validated_exception_takeover_metadata(
    state_root: &Path,
    run_id: &str,
) -> Result<Option<ExceptionTakeoverMetadata>, String> {
    let metadata = read_exception_takeover_metadata(state_root, run_id)?;
    if let Some(metadata) = metadata.as_ref() {
        metadata.validate()?;
    }
    Ok(metadata)
}

pub(crate) fn write_exception_takeover_metadata(
    state_root: &Path,
    run_id: &str,
    metadata: &ExceptionTakeoverMetadata,
) -> Result<String, String> {
    metadata.validate()?;
    let dir = metadata_dir(state_root);
    std::fs::create_dir_all(&dir).map_err(|error| {
        format!(
            "Failed to create exception takeover metadata directory `{}`: {error}",
            dir.display()
        )
    })?;
    let path = metadata_path(state_root, run_id)?;
    let encoded = serde_json::to_string_pretty(metadata).map_err(|error| {
        format!(
            "Failed to encode exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    std::fs::write(&path, encoded).map_err(|error| {
        format!(
            "Failed to persist exception takeover metadata `{}`: {error}",
            path.display()
        )
    })?;
    Ok(path.display().to_string())
}

pub(crate) fn owned_write_scope_for_summary(
    state_root: &Path,
    summary: &crate::state_store::RunGraphDispatchReceiptSummary,
) -> Vec<String> {
    read_validated_exception_takeover_metadata(state_root, &summary.run_id)
        .ok()
        .flatten()
        .filter(|metadata| metadata.matches_summary(summary))
        .map(|metadata| metadata.owned_write_scope())
        .unwrap_or_default()
}

pub(crate) fn owned_write_scope_for_latest_receipt(
    state_root: &Path,
    latest_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
) -> Vec<String> {
    latest_receipt
        .map(|receipt| owned_write_scope_for_summary(state_root, receipt))
        .unwrap_or_default()
}

pub(crate) fn metadata_matches_taskflow_active_work(
    state_root: &Path,
    latest_receipt: Option<&crate::state_store::RunGraphDispatchReceiptSummary>,
    taskflow_active_candidates: &[serde_json::Value],
) -> bool {
    let Some(receipt) = latest_receipt else {
        return false;
    };
    if receipt
        .supersedes_receipt_id
        .as_deref()
        .map(str::trim)
        .is_none_or(str::is_empty)
    {
        return false;
    }
    let [candidate] = taskflow_active_candidates else {
        return false;
    };
    let Some(candidate_task_id) = candidate
        .get("task_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let metadata_matches = read_validated_exception_takeover_metadata(state_root, &receipt.run_id)
        .ok()
        .flatten()
        .is_some_and(|metadata| metadata.matches_summary(receipt));
    if !metadata_matches {
        return false;
    }

    let receipt_run_id = receipt.run_id.trim();
    !receipt_run_id.is_empty()
        && (candidate_task_id == receipt_run_id
            || candidate
                .get("parent_task_ids")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .any(|parent_id| parent_id == receipt_run_id))
}
