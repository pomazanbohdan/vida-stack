use super::*;

const LAUNCHER_ACTIVATION_SNAPSHOT_ID: &str = "launcher_live";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, SurrealValue)]
pub struct LauncherActivationSnapshot {
    pub source: String,
    pub source_config_path: String,
    pub source_config_digest: String,
    pub captured_at: String,
    pub compiled_bundle: serde_json::Value,
    pub pack_router_keywords: serde_json::Value,
}

impl StateStore {
    pub async fn write_launcher_activation_snapshot(
        &self,
        snapshot: &LauncherActivationSnapshot,
    ) -> Result<(), StateStoreError> {
        snapshot.validate()?;
        let _: Option<LauncherActivationSnapshot> = self
            .db
            .upsert((
                "launcher_activation_snapshot",
                LAUNCHER_ACTIVATION_SNAPSHOT_ID,
            ))
            .content(snapshot.clone())
            .await?;
        Ok(())
    }

    pub async fn read_launcher_activation_snapshot(
        &self,
    ) -> Result<LauncherActivationSnapshot, StateStoreError> {
        let row: Option<LauncherActivationSnapshot> = self
            .db
            .select((
                "launcher_activation_snapshot",
                LAUNCHER_ACTIVATION_SNAPSHOT_ID,
            ))
            .await?;
        let row = row.ok_or(StateStoreError::MissingLauncherActivationSnapshot)?;
        row.validate_shape()?;
        Ok(row)
    }
}

impl LauncherActivationSnapshot {
    fn validate(&self) -> Result<(), StateStoreError> {
        if self.source != "state_store" {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: format!("unsupported source `{}`", self.source),
            });
        }
        if self.source_config_digest.trim().is_empty() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "source_config_digest is empty".to_string(),
            });
        }
        self.validate_shape()
    }

    fn validate_shape(&self) -> Result<(), StateStoreError> {
        if !self.compiled_bundle.is_object() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "compiled_bundle must be an object".to_string(),
            });
        }
        if !self.pack_router_keywords.is_object() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "pack_router_keywords must be an object".to_string(),
            });
        }
        let fallback_role = self.compiled_bundle["role_selection"]["fallback_role"]
            .as_str()
            .unwrap_or_default();
        if fallback_role.is_empty() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "role_selection.fallback_role is empty".to_string(),
            });
        }
        let selection_mode = self.compiled_bundle["role_selection"]["mode"]
            .as_str()
            .unwrap_or_default();
        if selection_mode.is_empty() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "role_selection.mode is empty".to_string(),
            });
        }
        if !self.compiled_bundle["agent_system"].is_object() {
            return Err(StateStoreError::InvalidLauncherActivationSnapshot {
                reason: "compiled_bundle.agent_system must be an object".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug)]
    struct JsonMismatch {
        path: String,
        before_type: &'static str,
        before_value: String,
        after_type: &'static str,
        after_value: String,
    }

    fn json_value_type(value: Option<&serde_json::Value>) -> &'static str {
        match value {
            None => "missing",
            Some(serde_json::Value::Null) => "null",
            Some(serde_json::Value::Bool(_)) => "boolean",
            Some(serde_json::Value::Number(_)) => "number",
            Some(serde_json::Value::String(_)) => "string",
            Some(serde_json::Value::Array(_)) => "array",
            Some(serde_json::Value::Object(_)) => "object",
        }
    }

    fn json_value_text(value: Option<&serde_json::Value>) -> String {
        value
            .map(serde_json::Value::to_string)
            .unwrap_or_else(|| "<missing>".to_string())
    }

    fn json_pointer_segment(value: &str) -> String {
        value.replace('~', "~0").replace('/', "~1")
    }

    fn first_json_mismatch(
        before: Option<&serde_json::Value>,
        after: Option<&serde_json::Value>,
        path: &str,
    ) -> Option<JsonMismatch> {
        if before == after {
            return None;
        }
        match (before, after) {
            (Some(serde_json::Value::Object(before)), Some(serde_json::Value::Object(after))) => {
                let keys = before.keys().chain(after.keys()).collect::<BTreeSet<_>>();
                for key in keys {
                    let child_path = format!("{path}/{}", json_pointer_segment(key));
                    if let Some(mismatch) =
                        first_json_mismatch(before.get(key), after.get(key), &child_path)
                    {
                        return Some(mismatch);
                    }
                }
                None
            }
            (Some(serde_json::Value::Array(before)), Some(serde_json::Value::Array(after))) => {
                for index in 0..before.len().max(after.len()) {
                    let child_path = format!("{path}/{index}");
                    if let Some(mismatch) =
                        first_json_mismatch(before.get(index), after.get(index), &child_path)
                    {
                        return Some(mismatch);
                    }
                }
                None
            }
            _ => Some(JsonMismatch {
                path: path.to_string(),
                before_type: json_value_type(before),
                before_value: json_value_text(before),
                after_type: json_value_type(after),
                after_value: json_value_text(after),
            }),
        }
    }

    #[tokio::test]
    async fn launcher_activation_snapshot_write_accepts_empty_source_config_path_as_provenance_only(
    ) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-launcher-activation-provenance-{}-{}",
            std::process::id(),
            nanos
        ));
        let store = StateStore::open(root.clone()).await.expect("open store");
        let snapshot = LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: String::new(),
            source_config_digest: "digest-123".to_string(),
            captured_at: "2026-03-08T00:00:00Z".to_string(),
            compiled_bundle: serde_json::json!({
                "role_selection": {
                    "fallback_role": "worker",
                    "mode": "native"
                },
                "agent_system": {}
            }),
            pack_router_keywords: serde_json::json!({}),
        };

        store
            .write_launcher_activation_snapshot(&snapshot)
            .await
            .expect("write launcher activation snapshot");

        let read_back = store
            .read_launcher_activation_snapshot()
            .await
            .expect("read launcher activation snapshot");
        assert_eq!(read_back.source, "state_store");
        assert_eq!(read_back.source_config_path, "");
        assert_eq!(read_back.source_config_digest, "digest-123");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn launcher_activation_snapshot_survives_state_store_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-launcher-activation-reopen-{}-{}",
            std::process::id(),
            nanos
        ));
        let snapshot = LauncherActivationSnapshot {
            source: "state_store".to_string(),
            source_config_path: String::new(),
            source_config_digest: "digest-123".to_string(),
            captured_at: "2026-03-08T00:00:00Z".to_string(),
            compiled_bundle: serde_json::json!({
                "role_selection": {
                    "fallback_role": "worker",
                    "mode": "native"
                },
                "agent_system": {}
            }),
            pack_router_keywords: serde_json::json!({}),
        };

        {
            let store = StateStore::open(root.clone()).await.expect("open store");
            store
                .write_launcher_activation_snapshot(&snapshot)
                .await
                .expect("write launcher activation snapshot");
        }

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let persisted = store
            .read_launcher_activation_snapshot()
            .await
            .expect("read persisted launcher activation snapshot");
        assert_eq!(persisted.source_config_digest, "digest-123");

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn captured_launcher_activation_snapshot_survives_state_store_reopen() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-launcher-activation-captured-reopen-{}-{}",
            std::process::id(),
            nanos
        ));
        let snapshot = crate::launcher_activation_snapshot::capture_launcher_activation_snapshot()
            .expect("capture snapshot");

        {
            let store = StateStore::open(root.clone()).await.expect("open store");
            store
                .write_launcher_activation_snapshot(&snapshot)
                .await
                .expect("write captured launcher activation snapshot");
        }

        let store = StateStore::open_existing(root.clone())
            .await
            .expect("reopen store");
        let persisted = store
            .read_launcher_activation_snapshot()
            .await
            .expect("read persisted captured launcher activation snapshot");
        assert_eq!(
            persisted.source_config_digest,
            snapshot.source_config_digest
        );
        let mismatch = first_json_mismatch(
            Some(&snapshot.compiled_bundle),
            Some(&persisted.compiled_bundle),
            "$",
        );
        assert!(
            persisted.compiled_bundle == snapshot.compiled_bundle,
            "compiled_bundle changed across state-store reopen at {}: before_type={}; before_value={}; after_type={}; after_value={}",
            mismatch.as_ref().map(|mismatch| mismatch.path.as_str()).unwrap_or("$"),
            mismatch
                .as_ref()
                .map(|mismatch| mismatch.before_type)
                .unwrap_or("unknown"),
            mismatch
                .as_ref()
                .map(|mismatch| mismatch.before_value.as_str())
                .unwrap_or("<unknown>"),
            mismatch
                .as_ref()
                .map(|mismatch| mismatch.after_type)
                .unwrap_or("unknown"),
            mismatch
                .as_ref()
                .map(|mismatch| mismatch.after_value.as_str())
                .unwrap_or("<unknown>"),
        );
        crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &persisted.compiled_bundle,
            None,
            None,
        )
        .unwrap_or_else(|blocker| {
            panic!(
                "persisted compiled_bundle must retain executable TeamFlow authority: code={}; requested={}; candidates={:?}",
                blocker.code, blocker.requested, blocker.candidates
            )
        });

        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn matching_digest_invalid_team_flow_snapshot_refreshes_from_canonical() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!(
            "vida-launcher-activation-semantic-refresh-{}-{}",
            std::process::id(),
            nanos
        ));
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("vida crate should live under the workspace root")
            .to_path_buf();
        let canonical =
            crate::launcher_activation_snapshot::capture_launcher_activation_snapshot_for_root(
                &workspace_root,
            )
            .expect("canonical snapshot should capture");
        let mut stale = canonical.clone();
        stale.compiled_bundle["team_flow_authority"]["resolved_all_flow_payload"]["flows"][0]
            ["lanes"][0]
            .as_object_mut()
            .expect("canonical lane should be an object")
            .remove("runtime_role");

        let store = StateStore::open(root.clone()).await.expect("open store");
        store
            .write_launcher_activation_snapshot(&stale)
            .await
            .expect("shape-valid stale snapshot should persist");
        let refreshed =
            crate::launcher_activation_snapshot::read_or_sync_launcher_activation_snapshot(&store)
                .await
                .expect("semantic mismatch should trigger canonical refresh");
        assert_eq!(
            refreshed.source_config_digest,
            canonical.source_config_digest
        );
        assert_eq!(refreshed.source_config_path, canonical.source_config_path);
        assert_eq!(
            refreshed.compiled_bundle["team_flow_authority"],
            canonical.compiled_bundle["team_flow_authority"],
            "canonical refresh must restore the stable TeamFlow authority contract"
        );
        crate::team_flow_authority_adapter::require_team_flow_execution_authority(
            &refreshed.compiled_bundle,
            None,
            None,
        )
        .expect("canonical refresh must restore executable TeamFlow authority");

        let _ = fs::remove_dir_all(&root);
    }
}
