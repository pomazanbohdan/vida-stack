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
        row.validate()?;
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn launcher_activation_snapshot_write_accepts_empty_source_config_path_as_provenance_only()
     {
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
}
