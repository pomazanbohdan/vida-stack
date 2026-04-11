use super::*;

const LAUNCHER_ACTIVATION_SNAPSHOT_ID: &str = "launcher_live";

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
