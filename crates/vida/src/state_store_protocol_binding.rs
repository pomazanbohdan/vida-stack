use super::*;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
struct ProtocolBindingStateRow {
    protocol_id: String,
    source_path: String,
    activation_class: String,
    runtime_owner: String,
    enforcement_type: String,
    proof_surface: String,
    primary_state_authority: String,
    binding_status: String,
    active: bool,
    blockers: Vec<String>,
    scenario: String,
    synced_at: String,
}

impl ProtocolBindingStateRow {
    fn from_state(
        scenario: &str,
        primary_state_authority: &str,
        synced_at: String,
        state: ProtocolBindingState,
    ) -> Self {
        Self {
            protocol_id: state.protocol_id,
            source_path: state.source_path,
            activation_class: state.activation_class,
            runtime_owner: state.runtime_owner,
            enforcement_type: state.enforcement_type,
            proof_surface: state.proof_surface,
            primary_state_authority: if state.primary_state_authority.trim().is_empty() {
                primary_state_authority.to_string()
            } else {
                state.primary_state_authority
            },
            binding_status: state.binding_status,
            active: state.active,
            blockers: state.blockers,
            scenario: scenario.to_string(),
            synced_at,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProtocolBindingState {
    pub protocol_id: String,
    pub source_path: String,
    pub activation_class: String,
    pub runtime_owner: String,
    pub enforcement_type: String,
    pub proof_surface: String,
    pub primary_state_authority: String,
    pub binding_status: String,
    pub active: bool,
    pub blockers: Vec<String>,
    pub scenario: String,
    pub synced_at: String,
}

impl ProtocolBindingState {
    fn from_row(row: ProtocolBindingStateRow) -> Self {
        Self {
            protocol_id: row.protocol_id,
            source_path: row.source_path,
            activation_class: row.activation_class,
            runtime_owner: row.runtime_owner,
            enforcement_type: row.enforcement_type,
            proof_surface: row.proof_surface,
            primary_state_authority: row.primary_state_authority,
            binding_status: row.binding_status,
            active: row.active,
            blockers: row.blockers,
            scenario: row.scenario,
            synced_at: row.synced_at,
        }
    }

    pub fn as_display(&self) -> String {
        let blockers = if self.blockers.is_empty() {
            "none".to_string()
        } else {
            self.blockers.join(",")
        };
        format!(
            "{} status={} active={} activation={} enforcement={} owner={} blockers={}",
            self.protocol_id,
            self.binding_status,
            self.active,
            self.activation_class,
            self.enforcement_type,
            self.runtime_owner,
            blockers
        )
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, SurrealValue)]
pub struct ProtocolBindingReceipt {
    pub receipt_id: String,
    pub scenario: String,
    pub total_bindings: usize,
    pub active_bindings: usize,
    pub script_bound_count: usize,
    pub rust_bound_count: usize,
    pub fully_runtime_bound_count: usize,
    pub unbound_count: usize,
    pub blocking_issue_count: usize,
    pub primary_state_authority: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ProtocolBindingSummary {
    pub total_receipts: usize,
    pub total_bindings: usize,
    pub active_bindings: usize,
    pub script_bound_count: usize,
    pub rust_bound_count: usize,
    pub fully_runtime_bound_count: usize,
    pub unbound_count: usize,
    pub blocking_issue_count: usize,
    pub latest_receipt_id: Option<String>,
    pub latest_scenario: Option<String>,
    pub latest_recorded_at: Option<String>,
    pub primary_state_authority: Option<String>,
}

impl ProtocolBindingSummary {
    pub fn as_display(&self) -> String {
        if self.total_receipts == 0 {
            return "idle (no protocol-binding receipts)".to_string();
        }
        format!(
            "receipts={} bindings={} active={} script_bound={} rust_bound={} fully_runtime_bound={} unbound={} blocking_issues={} latest_scenario={} authority={}",
            self.total_receipts,
            self.total_bindings,
            self.active_bindings,
            self.script_bound_count,
            self.rust_bound_count,
            self.fully_runtime_bound_count,
            self.unbound_count,
            self.blocking_issue_count,
            self.latest_scenario.as_deref().unwrap_or("none"),
            self.primary_state_authority.as_deref().unwrap_or("none"),
        )
    }
}

impl StateStore {
    pub async fn record_protocol_binding_snapshot(
        &self,
        scenario: &str,
        primary_state_authority: &str,
        bindings: &[ProtocolBindingState],
    ) -> Result<ProtocolBindingReceipt, StateStoreError> {
        if scenario.trim().is_empty() {
            return Err(StateStoreError::InvalidProtocolBinding {
                reason: "scenario is required".to_string(),
            });
        }
        if primary_state_authority.trim().is_empty() {
            return Err(StateStoreError::InvalidProtocolBinding {
                reason: "primary_state_authority is required".to_string(),
            });
        }
        if bindings.is_empty() {
            return Err(StateStoreError::InvalidProtocolBinding {
                reason: "at least one protocol binding row is required".to_string(),
            });
        }

        let recorded_at = unix_timestamp_nanos().to_string();
        let receipt_id = format!("protocol-binding-{recorded_at}");
        let scenario_literal = escape_surql_literal(scenario);
        self.db
            .query(format!(
                "DELETE protocol_binding_state WHERE scenario = '{scenario_literal}';"
            ))
            .await?;

        let mut active_bindings = 0usize;
        let mut script_bound_count = 0usize;
        let mut rust_bound_count = 0usize;
        let mut fully_runtime_bound_count = 0usize;
        let mut unbound_count = 0usize;
        let mut blocking_issue_count = 0usize;

        for binding in bindings {
            let record = ProtocolBindingStateRow::from_state(
                scenario,
                primary_state_authority,
                recorded_at.clone(),
                binding.clone(),
            );
            if record.active {
                active_bindings += 1;
            }
            match record.binding_status.as_str() {
                "script-bound" => script_bound_count += 1,
                "rust-bound" => rust_bound_count += 1,
                "fully-runtime-bound" => fully_runtime_bound_count += 1,
                _ => unbound_count += 1,
            }
            blocking_issue_count += record.blockers.len();
            let row_id = format!(
                "{}--{}",
                sanitize_record_id(scenario),
                sanitize_record_id(&record.protocol_id)
            );
            let _: Option<ProtocolBindingStateRow> = self
                .db
                .upsert(("protocol_binding_state", row_id.as_str()))
                .content(record)
                .await?;
        }

        let receipt = ProtocolBindingReceipt {
            receipt_id,
            scenario: scenario.to_string(),
            total_bindings: bindings.len(),
            active_bindings,
            script_bound_count,
            rust_bound_count,
            fully_runtime_bound_count,
            unbound_count,
            blocking_issue_count,
            primary_state_authority: primary_state_authority.to_string(),
            recorded_at,
        };
        let _: Option<ProtocolBindingReceipt> = self
            .db
            .upsert(("protocol_binding_receipt", receipt.receipt_id.as_str()))
            .content(receipt.clone())
            .await?;
        Ok(receipt)
    }

    pub async fn latest_protocol_binding_receipt(
        &self,
    ) -> Result<Option<ProtocolBindingReceipt>, StateStoreError> {
        let mut query = self
            .db
            .query(
                "SELECT receipt_id, scenario, total_bindings, active_bindings, script_bound_count, rust_bound_count, fully_runtime_bound_count, unbound_count, blocking_issue_count, primary_state_authority, recorded_at FROM protocol_binding_receipt ORDER BY recorded_at DESC LIMIT 1;",
            )
            .await?;
        let rows: Vec<ProtocolBindingReceipt> = query.take(0)?;
        Ok(rows.into_iter().next())
    }

    pub async fn latest_protocol_binding_cache_token(
        &self,
    ) -> Result<Option<String>, StateStoreError> {
        let Some(receipt) = self.latest_protocol_binding_receipt().await? else {
            return Ok(None);
        };
        if receipt.receipt_id.trim().is_empty()
            || receipt.recorded_at.trim().is_empty()
            || receipt.primary_state_authority.trim().is_empty()
        {
            return Ok(None);
        }
        Ok(Some(format!(
            "{}::{}::{}",
            receipt.primary_state_authority, receipt.receipt_id, receipt.recorded_at
        )))
    }

    pub async fn latest_protocol_binding_rows(
        &self,
    ) -> Result<Vec<ProtocolBindingState>, StateStoreError> {
        let Some(receipt) = self.latest_protocol_binding_receipt().await? else {
            return Ok(Vec::new());
        };
        let mut query = self
            .db
            .query(format!(
                "SELECT protocol_id, source_path, activation_class, runtime_owner, enforcement_type, proof_surface, primary_state_authority, binding_status, active, blockers, scenario, synced_at FROM protocol_binding_state WHERE scenario = '{}' ORDER BY protocol_id ASC;",
                escape_surql_literal(&receipt.scenario)
            ))
            .await?;
        let rows: Vec<ProtocolBindingStateRow> = query.take(0)?;
        Ok(rows
            .into_iter()
            .map(ProtocolBindingState::from_row)
            .collect())
    }

    pub async fn protocol_binding_summary(
        &self,
    ) -> Result<ProtocolBindingSummary, StateStoreError> {
        let latest_receipt = self.latest_protocol_binding_receipt().await?;
        Ok(match latest_receipt {
            Some(receipt) => ProtocolBindingSummary {
                total_receipts: self.count_table_rows("protocol_binding_receipt").await?,
                total_bindings: receipt.total_bindings,
                active_bindings: receipt.active_bindings,
                script_bound_count: receipt.script_bound_count,
                rust_bound_count: receipt.rust_bound_count,
                fully_runtime_bound_count: receipt.fully_runtime_bound_count,
                unbound_count: receipt.unbound_count,
                blocking_issue_count: receipt.blocking_issue_count,
                latest_receipt_id: Some(receipt.receipt_id),
                latest_scenario: Some(receipt.scenario),
                latest_recorded_at: Some(receipt.recorded_at),
                primary_state_authority: Some(receipt.primary_state_authority),
            },
            None => ProtocolBindingSummary {
                total_receipts: 0,
                total_bindings: 0,
                active_bindings: 0,
                script_bound_count: 0,
                rust_bound_count: 0,
                fully_runtime_bound_count: 0,
                unbound_count: 0,
                blocking_issue_count: 0,
                latest_receipt_id: None,
                latest_scenario: None,
                latest_recorded_at: None,
                primary_state_authority: None,
            },
        })
    }
}
