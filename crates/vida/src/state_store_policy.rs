use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use vida_policy_rhai::PolicyBundle;

#[path = "policy_runtime.rs"]
pub mod runtime;

pub const POLICY_LIFECYCLE_SCHEMA_VERSION: u16 = 1;
const MAX_POLICY_RECEIPT_ID_BYTES: usize = 128;
const MAX_POLICY_RECEIPT_CODE_BYTES: usize = 128;

fn valid_policy_receipt_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_POLICY_RECEIPT_ID_BYTES
        && !value.to_ascii_lowercase().contains("context")
        && !value.to_ascii_lowercase().contains("source")
        && !value.to_ascii_lowercase().contains("secret")
}

fn valid_policy_receipt_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_policy_receipt_code(value: Option<&str>) -> bool {
    value
        .map(|value| {
            !value.trim().is_empty()
                && value.len() <= MAX_POLICY_RECEIPT_CODE_BYTES
                && !value.to_ascii_lowercase().contains("context")
                && !value.to_ascii_lowercase().contains("source")
                && !value.to_ascii_lowercase().contains("secret")
        })
        .unwrap_or(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyMode {
    Off,
    Shadow,
    Active,
}

impl Default for PolicyMode {
    fn default() -> Self {
        Self::Off
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyLifecycle {
    Candidate,
    Active,
    Retired,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyBundleRecord {
    pub bundle_id: String,
    pub policy_id: String,
    pub version: u32,
    pub engine_abi: String,
    pub source: String,
    pub content_digest: String,
    pub lifecycle: PolicyLifecycle,
}

impl PolicyBundleRecord {
    pub fn from_bundle(bundle: &PolicyBundle, content_digest: impl Into<String>) -> Self {
        Self {
            bundle_id: format!("{}@{}", bundle.policy_id, bundle.version),
            policy_id: bundle.policy_id.clone(),
            version: bundle.version,
            engine_abi: bundle.engine_abi.clone(),
            source: bundle.source.clone(),
            content_digest: content_digest.into(),
            lifecycle: PolicyLifecycle::Candidate,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyTestReceipt {
    pub bundle_id: String,
    pub test_id: String,
    pub content_digest: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyEvaluationReceipt {
    pub bundle_id: String,
    pub evaluation_id: String,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyShadowDiff {
    pub bundle_id: String,
    pub expected_digest: String,
    pub observed_digest: String,
    pub diverged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyModeRecord {
    pub bundle_id: String,
    pub mode: PolicyMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRunPin {
    pub run_id: String,
    pub bundle_id: String,
    pub policy_id: String,
    pub version: u32,
    pub content_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyShadowReceipt {
    pub receipt_id: String,
    pub run_id: String,
    pub bundle_id: String,
    pub policy_id: String,
    pub version: u32,
    pub content_digest: String,
    pub input_digest: String,
    pub output_digest: Option<String>,
    pub duration_ms: u64,
    pub agreed: Option<bool>,
    pub diff_code: Option<String>,
    pub error_code: Option<String>,
    pub fallback_code: Option<String>,
}

impl PolicyShadowReceipt {
    fn validate(&self) -> bool {
        valid_policy_receipt_id(&self.receipt_id)
            && valid_policy_receipt_id(&self.run_id)
            && valid_policy_receipt_id(&self.bundle_id)
            && valid_policy_receipt_id(&self.policy_id)
            && valid_policy_receipt_digest(&self.content_digest)
            && valid_policy_receipt_digest(&self.input_digest)
            && self
                .output_digest
                .as_deref()
                .map(valid_policy_receipt_digest)
                .unwrap_or(true)
            && valid_policy_receipt_code(self.diff_code.as_deref())
            && valid_policy_receipt_code(self.error_code.as_deref())
            && valid_policy_receipt_code(self.fallback_code.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyLifecycleStoreSnapshot {
    pub schema_version: u16,
    pub bundles: Vec<PolicyBundleRecord>,
    pub test_receipts: Vec<PolicyTestReceipt>,
    pub evaluation_receipts: Vec<PolicyEvaluationReceipt>,
    pub shadow_diffs: Vec<PolicyShadowDiff>,
    pub active_pointer: Option<String>,
    pub last_known_good: Option<String>,
    #[serde(default)]
    pub modes: Vec<PolicyModeRecord>,
    #[serde(default)]
    pub run_pins: Vec<PolicyRunPin>,
    #[serde(default)]
    pub shadow_receipts: Vec<PolicyShadowReceipt>,
}

#[derive(Debug, Default)]
pub struct PolicyLifecycleStore {
    bundles: BTreeMap<String, PolicyBundleRecord>,
    test_receipts: BTreeMap<String, PolicyTestReceipt>,
    evaluation_receipts: BTreeMap<String, PolicyEvaluationReceipt>,
    shadow_diffs: BTreeMap<String, PolicyShadowDiff>,
    active_pointer: Option<String>,
    last_known_good: Option<String>,
    modes: BTreeMap<String, PolicyMode>,
    run_pins: BTreeMap<String, PolicyRunPin>,
    shadow_receipts: BTreeMap<String, PolicyShadowReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyLifecycleStoreError {
    InvalidSnapshotSchema(u16),
    DuplicateBundleImport {
        bundle_id: String,
    },
    BundleNotFound {
        bundle_id: String,
    },
    MissingTestReceipt {
        bundle_id: String,
    },
    TestReceiptFailed {
        bundle_id: String,
    },
    TestReceiptDigestMismatch {
        bundle_id: String,
    },
    ActivePointerMissing {
        bundle_id: String,
    },
    RollbackRequiresActive {
        bundle_id: String,
    },
    MissingLastKnownGood,
    InvalidModeTransition {
        bundle_id: String,
        from: PolicyMode,
        to: PolicyMode,
    },
    RunPinMissing {
        run_id: String,
    },
    RunPinConflict {
        run_id: String,
    },
    RunPinDigestMismatch {
        run_id: String,
    },
    ShadowReceiptInvalid {
        receipt_id: String,
    },
}

impl PolicyLifecycleStoreError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidSnapshotSchema(_) => "policy_snapshot_schema_invalid",
            Self::DuplicateBundleImport { .. } => "policy_bundle_duplicate",
            Self::BundleNotFound { .. } => "policy_bundle_not_found",
            Self::MissingTestReceipt { .. } => "policy_test_receipt_missing",
            Self::TestReceiptFailed { .. } => "policy_test_receipt_failed",
            Self::TestReceiptDigestMismatch { .. } => "policy_test_receipt_digest_mismatch",
            Self::ActivePointerMissing { .. } => "policy_active_pointer_missing",
            Self::RollbackRequiresActive { .. } => "policy_rollback_requires_active",
            Self::MissingLastKnownGood => "policy_last_known_good_missing",
            Self::InvalidModeTransition { .. } => "policy_mode_transition_invalid",
            Self::RunPinMissing { .. } => "policy_run_pin_missing",
            Self::RunPinConflict { .. } => "policy_run_pin_conflict",
            Self::RunPinDigestMismatch { .. } => "policy_run_pin_digest_mismatch",
            Self::ShadowReceiptInvalid { .. } => "policy_shadow_receipt_invalid",
        }
    }
}

impl fmt::Display for PolicyLifecycleStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSnapshotSchema(version) => {
                write!(formatter, "unsupported policy snapshot schema {version}")
            }
            Self::DuplicateBundleImport { bundle_id } => {
                write!(formatter, "policy bundle {bundle_id} was already imported")
            }
            Self::BundleNotFound { bundle_id } => {
                write!(formatter, "policy bundle {bundle_id} not found")
            }
            Self::MissingTestReceipt { bundle_id } => {
                write!(
                    formatter,
                    "passing test receipt missing for policy bundle {bundle_id}"
                )
            }
            Self::TestReceiptFailed { bundle_id } => {
                write!(
                    formatter,
                    "test receipt failed for policy bundle {bundle_id}"
                )
            }
            Self::TestReceiptDigestMismatch { bundle_id } => {
                write!(
                    formatter,
                    "test receipt digest does not match policy bundle {bundle_id}"
                )
            }
            Self::ActivePointerMissing { bundle_id } => {
                write!(
                    formatter,
                    "active policy pointer references missing bundle {bundle_id}"
                )
            }
            Self::RollbackRequiresActive { bundle_id } => {
                write!(
                    formatter,
                    "rollback requires active policy bundle {bundle_id}"
                )
            }
            Self::MissingLastKnownGood => {
                write!(formatter, "last-known-good policy pointer is missing")
            }
            Self::InvalidModeTransition {
                bundle_id,
                from,
                to,
            } => {
                write!(
                    formatter,
                    "invalid policy mode transition for {bundle_id}: {from:?} -> {to:?}"
                )
            }
            Self::RunPinMissing { run_id } => {
                write!(formatter, "policy run pin missing for {run_id}")
            }
            Self::RunPinConflict { run_id } => {
                write!(formatter, "policy run pin is immutable for {run_id}")
            }
            Self::RunPinDigestMismatch { run_id } => {
                write!(formatter, "policy run pin digest mismatch for {run_id}")
            }
            Self::ShadowReceiptInvalid { receipt_id } => {
                write!(formatter, "invalid redacted shadow receipt {receipt_id}")
            }
        }
    }
}

impl std::error::Error for PolicyLifecycleStoreError {}

impl PolicyLifecycleStore {
    pub fn import_bundle(
        &mut self,
        bundle: PolicyBundleRecord,
    ) -> Result<(), PolicyLifecycleStoreError> {
        if self.bundles.contains_key(&bundle.bundle_id) {
            return Err(PolicyLifecycleStoreError::DuplicateBundleImport {
                bundle_id: bundle.bundle_id,
            });
        }
        self.bundles.insert(bundle.bundle_id.clone(), bundle);
        Ok(())
    }

    pub fn record_test_receipt(
        &mut self,
        receipt: PolicyTestReceipt,
    ) -> Result<(), PolicyLifecycleStoreError> {
        if !self.bundles.contains_key(&receipt.bundle_id) {
            return Err(PolicyLifecycleStoreError::BundleNotFound {
                bundle_id: receipt.bundle_id,
            });
        }
        self.test_receipts
            .insert(receipt.bundle_id.clone(), receipt);
        Ok(())
    }

    pub fn record_evaluation_receipt(
        &mut self,
        receipt: PolicyEvaluationReceipt,
    ) -> Result<(), PolicyLifecycleStoreError> {
        if !self.bundles.contains_key(&receipt.bundle_id) {
            return Err(PolicyLifecycleStoreError::BundleNotFound {
                bundle_id: receipt.bundle_id,
            });
        }
        self.evaluation_receipts
            .insert(receipt.evaluation_id.clone(), receipt);
        Ok(())
    }

    pub fn record_shadow_diff(
        &mut self,
        diff: PolicyShadowDiff,
    ) -> Result<(), PolicyLifecycleStoreError> {
        if !self.bundles.contains_key(&diff.bundle_id) {
            return Err(PolicyLifecycleStoreError::BundleNotFound {
                bundle_id: diff.bundle_id,
            });
        }
        self.shadow_diffs.insert(diff.bundle_id.clone(), diff);
        Ok(())
    }

    pub fn mode(&self, bundle_id: &str) -> Result<PolicyMode, PolicyLifecycleStoreError> {
        if !self.bundles.contains_key(bundle_id) {
            return Err(PolicyLifecycleStoreError::BundleNotFound {
                bundle_id: bundle_id.to_string(),
            });
        }
        Ok(self.modes.get(bundle_id).copied().unwrap_or_default())
    }

    pub fn set_mode(
        &mut self,
        bundle_id: &str,
        mode: PolicyMode,
    ) -> Result<(), PolicyLifecycleStoreError> {
        let current = self.mode(bundle_id)?;
        if current != mode
            && !matches!(
                (current, mode),
                (PolicyMode::Off, PolicyMode::Shadow) | (PolicyMode::Shadow, PolicyMode::Active)
            )
        {
            return Err(PolicyLifecycleStoreError::InvalidModeTransition {
                bundle_id: bundle_id.to_string(),
                from: current,
                to: mode,
            });
        }
        self.modes.insert(bundle_id.to_string(), mode);
        Ok(())
    }

    pub fn record_run_pin(&mut self, pin: PolicyRunPin) -> Result<(), PolicyLifecycleStoreError> {
        let bundle = self.bundles.get(&pin.bundle_id).ok_or_else(|| {
            PolicyLifecycleStoreError::BundleNotFound {
                bundle_id: pin.bundle_id.clone(),
            }
        })?;
        if bundle.policy_id != pin.policy_id
            || bundle.version != pin.version
            || bundle.content_digest != pin.content_digest
        {
            return Err(PolicyLifecycleStoreError::RunPinDigestMismatch { run_id: pin.run_id });
        }
        if let Some(existing) = self.run_pins.get(&pin.run_id) {
            if existing != &pin {
                return Err(PolicyLifecycleStoreError::RunPinConflict { run_id: pin.run_id });
            }
            return Ok(());
        }
        self.run_pins.insert(pin.run_id.clone(), pin);
        Ok(())
    }

    pub fn run_pin(&self, run_id: &str) -> Result<&PolicyRunPin, PolicyLifecycleStoreError> {
        self.run_pins
            .get(run_id)
            .ok_or_else(|| PolicyLifecycleStoreError::RunPinMissing {
                run_id: run_id.to_string(),
            })
    }

    pub fn record_shadow_receipt(
        &mut self,
        receipt: PolicyShadowReceipt,
    ) -> Result<(), PolicyLifecycleStoreError> {
        if !receipt.validate() {
            return Err(PolicyLifecycleStoreError::ShadowReceiptInvalid {
                receipt_id: receipt.receipt_id,
            });
        }
        let bundle = self.bundles.get(&receipt.bundle_id).ok_or_else(|| {
            PolicyLifecycleStoreError::BundleNotFound {
                bundle_id: receipt.bundle_id.clone(),
            }
        })?;
        if bundle.policy_id != receipt.policy_id
            || bundle.version != receipt.version
            || bundle.content_digest != receipt.content_digest
        {
            return Err(PolicyLifecycleStoreError::ShadowReceiptInvalid {
                receipt_id: receipt.receipt_id,
            });
        }
        let receipt_id = receipt.receipt_id.clone();
        if self.shadow_receipts.contains_key(&receipt_id) {
            return Err(PolicyLifecycleStoreError::ShadowReceiptInvalid { receipt_id });
        }
        self.shadow_receipts.insert(receipt_id, receipt);
        Ok(())
    }

    pub fn activate(&mut self, bundle_id: &str) -> Result<(), PolicyLifecycleStoreError> {
        let bundle = self
            .bundles
            .get(bundle_id)
            .ok_or_else(|| PolicyLifecycleStoreError::BundleNotFound {
                bundle_id: bundle_id.to_string(),
            })?
            .clone();
        let receipt = self.test_receipts.get(bundle_id).ok_or_else(|| {
            PolicyLifecycleStoreError::MissingTestReceipt {
                bundle_id: bundle_id.to_string(),
            }
        })?;
        if !receipt.passed {
            return Err(PolicyLifecycleStoreError::TestReceiptFailed {
                bundle_id: bundle_id.to_string(),
            });
        }
        if receipt.content_digest != bundle.content_digest {
            return Err(PolicyLifecycleStoreError::TestReceiptDigestMismatch {
                bundle_id: bundle_id.to_string(),
            });
        }

        let previous_active = self.active_pointer.clone();
        if let Some(previous_id) = previous_active.as_deref() {
            if !self.bundles.contains_key(previous_id) {
                return Err(PolicyLifecycleStoreError::ActivePointerMissing {
                    bundle_id: previous_id.to_string(),
                });
            }
        }

        if let Some(previous_id) = previous_active.as_deref() {
            if previous_id != bundle_id {
                self.bundles
                    .get_mut(previous_id)
                    .expect("validated active pointer")
                    .lifecycle = PolicyLifecycle::Retired;
                self.last_known_good = Some(previous_id.to_string());
            }
        }
        self.bundles
            .get_mut(bundle_id)
            .expect("validated bundle")
            .lifecycle = PolicyLifecycle::Active;
        self.active_pointer = Some(bundle_id.to_string());
        self.modes.insert(bundle_id.to_string(), PolicyMode::Active);
        Ok(())
    }

    pub fn rollback(&mut self, failed_bundle_id: &str) -> Result<(), PolicyLifecycleStoreError> {
        if self.active_pointer.as_deref() != Some(failed_bundle_id) {
            return Err(PolicyLifecycleStoreError::RollbackRequiresActive {
                bundle_id: failed_bundle_id.to_string(),
            });
        }
        let last_known_good = self
            .last_known_good
            .clone()
            .ok_or(PolicyLifecycleStoreError::MissingLastKnownGood)?;
        if !self.bundles.contains_key(&last_known_good) {
            return Err(PolicyLifecycleStoreError::ActivePointerMissing {
                bundle_id: last_known_good,
            });
        }

        self.bundles
            .get_mut(failed_bundle_id)
            .expect("validated active pointer")
            .lifecycle = PolicyLifecycle::Quarantined;
        self.bundles
            .get_mut(&last_known_good)
            .expect("validated last-known-good pointer")
            .lifecycle = PolicyLifecycle::Active;
        self.active_pointer = Some(last_known_good);
        self.modes
            .insert(failed_bundle_id.to_string(), PolicyMode::Off);
        Ok(())
    }

    pub fn active_pointer(&self) -> Option<&str> {
        self.active_pointer.as_deref()
    }

    pub fn last_known_good(&self) -> Option<&str> {
        self.last_known_good.as_deref()
    }

    pub fn bundle(&self, bundle_id: &str) -> Option<&PolicyBundleRecord> {
        self.bundles.get(bundle_id)
    }

    pub fn snapshot(&self) -> PolicyLifecycleStoreSnapshot {
        PolicyLifecycleStoreSnapshot {
            schema_version: POLICY_LIFECYCLE_SCHEMA_VERSION,
            bundles: self.bundles.values().cloned().collect(),
            test_receipts: self.test_receipts.values().cloned().collect(),
            evaluation_receipts: self.evaluation_receipts.values().cloned().collect(),
            shadow_diffs: self.shadow_diffs.values().cloned().collect(),
            active_pointer: self.active_pointer.clone(),
            last_known_good: self.last_known_good.clone(),
            modes: self
                .modes
                .iter()
                .map(|(bundle_id, mode)| PolicyModeRecord {
                    bundle_id: bundle_id.clone(),
                    mode: *mode,
                })
                .collect(),
            run_pins: self.run_pins.values().cloned().collect(),
            shadow_receipts: self.shadow_receipts.values().cloned().collect(),
        }
    }

    pub fn from_snapshot(
        snapshot: PolicyLifecycleStoreSnapshot,
    ) -> Result<Self, PolicyLifecycleStoreError> {
        if snapshot.schema_version != POLICY_LIFECYCLE_SCHEMA_VERSION {
            return Err(PolicyLifecycleStoreError::InvalidSnapshotSchema(
                snapshot.schema_version,
            ));
        }
        let PolicyLifecycleStoreSnapshot {
            bundles,
            test_receipts,
            evaluation_receipts,
            shadow_diffs,
            active_pointer,
            last_known_good,
            modes,
            run_pins,
            shadow_receipts,
            ..
        } = snapshot;
        let mut store = Self {
            bundles: bundles
                .into_iter()
                .map(|bundle| (bundle.bundle_id.clone(), bundle))
                .collect(),
            test_receipts: test_receipts
                .into_iter()
                .map(|receipt| (receipt.bundle_id.clone(), receipt))
                .collect(),
            evaluation_receipts: evaluation_receipts
                .into_iter()
                .map(|receipt| (receipt.evaluation_id.clone(), receipt))
                .collect(),
            shadow_diffs: shadow_diffs
                .into_iter()
                .map(|diff| (diff.bundle_id.clone(), diff))
                .collect(),
            active_pointer,
            last_known_good,
            modes: modes
                .into_iter()
                .map(|record| (record.bundle_id, record.mode))
                .collect(),
            run_pins: run_pins
                .into_iter()
                .map(|pin| (pin.run_id.clone(), pin))
                .collect(),
            shadow_receipts: BTreeMap::new(),
        };
        for receipt in shadow_receipts {
            store.record_shadow_receipt(receipt)?;
        }
        Ok(store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(id: &str) -> String {
        blake3::hash(id.as_bytes()).to_hex().to_string()
    }

    fn bundle(id: &str) -> PolicyBundleRecord {
        PolicyBundleRecord {
            bundle_id: id.to_string(),
            policy_id: "policy".to_string(),
            version: 1,
            engine_abi: "rhai-policy-engine-v1".to_string(),
            source: "1".to_string(),
            content_digest: digest(id),
            lifecycle: PolicyLifecycle::Candidate,
        }
    }

    fn receipt(id: &str) -> PolicyTestReceipt {
        PolicyTestReceipt {
            bundle_id: id.to_string(),
            test_id: format!("test-{id}"),
            content_digest: digest(id),
            passed: true,
        }
    }

    fn shadow_receipt(bundle_id: &str, receipt_id: &str) -> PolicyShadowReceipt {
        PolicyShadowReceipt {
            receipt_id: receipt_id.to_string(),
            run_id: "run-one".to_string(),
            bundle_id: bundle_id.to_string(),
            policy_id: "policy".to_string(),
            version: 1,
            content_digest: digest(bundle_id),
            input_digest: digest("input"),
            output_digest: Some(digest("output")),
            duration_ms: 4,
            agreed: Some(true),
            diff_code: None,
            error_code: None,
            fallback_code: None,
        }
    }

    #[test]
    fn import_activation_and_rollback_are_receipt_gated_and_restart_safe() {
        let mut store = PolicyLifecycleStore::default();
        store.import_bundle(bundle("one")).unwrap();
        store.import_bundle(bundle("two")).unwrap();
        store.record_test_receipt(receipt("one")).unwrap();
        store.record_test_receipt(receipt("two")).unwrap();
        assert_eq!(store.activate("one"), Ok(()));
        assert_eq!(store.activate("two"), Ok(()));
        let snapshot = store.snapshot();
        let mut restarted = PolicyLifecycleStore::from_snapshot(snapshot).unwrap();
        assert_eq!(restarted.active_pointer(), Some("two"));
        assert_eq!(restarted.last_known_good(), Some("one"));
        restarted.rollback("two").unwrap();
        assert_eq!(restarted.active_pointer(), Some("one"));
        assert_eq!(
            restarted.bundle("two").unwrap().lifecycle,
            PolicyLifecycle::Quarantined
        );
    }

    #[test]
    fn duplicate_and_nonpassing_receipts_fail_closed() {
        let mut store = PolicyLifecycleStore::default();
        store.import_bundle(bundle("one")).unwrap();
        assert!(matches!(
            store.import_bundle(bundle("one")),
            Err(PolicyLifecycleStoreError::DuplicateBundleImport { .. })
        ));
        store
            .record_test_receipt(PolicyTestReceipt {
                passed: false,
                ..receipt("one")
            })
            .unwrap();
        assert!(matches!(
            store.activate("one"),
            Err(PolicyLifecycleStoreError::TestReceiptFailed { .. })
        ));
    }

    #[test]
    fn mode_rollout_pins_and_redacted_shadow_receipts_survive_restart() {
        let mut store = PolicyLifecycleStore::default();
        store.import_bundle(bundle("one")).unwrap();
        assert_eq!(store.mode("one"), Ok(PolicyMode::Off));
        store.set_mode("one", PolicyMode::Shadow).unwrap();
        assert_eq!(
            store.set_mode("one", PolicyMode::Off),
            Err(PolicyLifecycleStoreError::InvalidModeTransition {
                bundle_id: "one".to_string(),
                from: PolicyMode::Shadow,
                to: PolicyMode::Off,
            })
        );
        let pin = PolicyRunPin {
            run_id: "run-one".to_string(),
            bundle_id: "one".to_string(),
            policy_id: "policy".to_string(),
            version: 1,
            content_digest: digest("one"),
        };
        store.record_run_pin(pin.clone()).unwrap();
        assert_eq!(store.record_run_pin(pin.clone()), Ok(()));
        assert!(matches!(
            store.record_run_pin(PolicyRunPin {
                content_digest: "other".to_string(),
                ..pin
            }),
            Err(PolicyLifecycleStoreError::RunPinDigestMismatch { .. })
        ));
        store
            .record_shadow_receipt(PolicyShadowReceipt {
                receipt_id: "receipt-one".to_string(),
                run_id: "run-one".to_string(),
                bundle_id: "one".to_string(),
                policy_id: "policy".to_string(),
                version: 1,
                content_digest: digest("one"),
                input_digest: digest("input"),
                output_digest: Some(digest("output")),
                duration_ms: 4,
                agreed: Some(true),
                diff_code: None,
                error_code: None,
                fallback_code: None,
            })
            .unwrap();
        let restarted = PolicyLifecycleStore::from_snapshot(store.snapshot()).unwrap();
        assert_eq!(restarted.mode("one"), Ok(PolicyMode::Shadow));
        assert_eq!(restarted.run_pin("run-one").unwrap().bundle_id, "one");
        assert_eq!(restarted.snapshot().shadow_receipts.len(), 1);
    }

    #[test]
    fn shadow_receipt_round_trip_rejects_unknown_fields_and_preserves_restart_state() {
        let mut store = PolicyLifecycleStore::default();
        store.import_bundle(bundle("one")).unwrap();
        let receipt = shadow_receipt("one", "receipt-round-trip");
        store.record_shadow_receipt(receipt.clone()).unwrap();
        let encoded = serde_json::to_string(&receipt).unwrap();
        let decoded: PolicyShadowReceipt = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, receipt);
        let restarted = PolicyLifecycleStore::from_snapshot(store.snapshot()).unwrap();
        assert_eq!(restarted.snapshot().shadow_receipts, vec![receipt]);

        let mut unknown = serde_json::to_value(decoded).unwrap();
        unknown["raw_context"] = serde_json::json!({"secret": "never"});
        assert!(serde_json::from_value::<PolicyShadowReceipt>(unknown).is_err());
    }

    #[test]
    fn shadow_receipt_digest_identity_and_duplicate_validation_fail_closed() {
        let mut store = PolicyLifecycleStore::default();
        store.import_bundle(bundle("one")).unwrap();
        let receipt = shadow_receipt("one", "receipt-duplicate");
        store.record_shadow_receipt(receipt.clone()).unwrap();
        assert!(matches!(
            store.record_shadow_receipt(receipt.clone()),
            Err(PolicyLifecycleStoreError::ShadowReceiptInvalid { .. })
        ));
        assert_eq!(store.snapshot().shadow_receipts.len(), 1);

        let mut mismatch = shadow_receipt("one", "receipt-mismatch");
        mismatch.content_digest = digest("different");
        assert!(matches!(
            store.record_shadow_receipt(mismatch),
            Err(PolicyLifecycleStoreError::ShadowReceiptInvalid { .. })
        ));

        let mut duplicate_snapshot = store.snapshot();
        duplicate_snapshot
            .shadow_receipts
            .push(duplicate_snapshot.shadow_receipts[0].clone());
        assert!(matches!(
            PolicyLifecycleStore::from_snapshot(duplicate_snapshot),
            Err(PolicyLifecycleStoreError::ShadowReceiptInvalid { .. })
        ));
    }

    #[test]
    fn shadow_receipt_digest_shape_and_raw_source_fields_are_rejected() {
        let mut store = PolicyLifecycleStore::default();
        store.import_bundle(bundle("one")).unwrap();
        let mut malformed = shadow_receipt("one", "receipt-malformed");
        malformed.input_digest = "not-a-digest".to_string();
        assert!(matches!(
            store.record_shadow_receipt(malformed),
            Err(PolicyLifecycleStoreError::ShadowReceiptInvalid { .. })
        ));
        let mut raw_code = shadow_receipt("one", "receipt-raw");
        raw_code.error_code = Some("raw source context".to_string());
        assert!(matches!(
            store.record_shadow_receipt(raw_code),
            Err(PolicyLifecycleStoreError::ShadowReceiptInvalid { .. })
        ));
    }
}
