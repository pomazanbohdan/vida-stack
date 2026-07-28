use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use vida_policy_rhai::PolicyBundle;

pub const POLICY_LIFECYCLE_SCHEMA_VERSION: u16 = 1;

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
pub struct PolicyLifecycleStoreSnapshot {
    pub schema_version: u16,
    pub bundles: Vec<PolicyBundleRecord>,
    pub test_receipts: Vec<PolicyTestReceipt>,
    pub evaluation_receipts: Vec<PolicyEvaluationReceipt>,
    pub shadow_diffs: Vec<PolicyShadowDiff>,
    pub active_pointer: Option<String>,
    pub last_known_good: Option<String>,
}

#[derive(Debug, Default)]
pub struct PolicyLifecycleStore {
    bundles: BTreeMap<String, PolicyBundleRecord>,
    test_receipts: BTreeMap<String, PolicyTestReceipt>,
    evaluation_receipts: BTreeMap<String, PolicyEvaluationReceipt>,
    shadow_diffs: BTreeMap<String, PolicyShadowDiff>,
    active_pointer: Option<String>,
    last_known_good: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyLifecycleStoreError {
    InvalidSnapshotSchema(u16),
    DuplicateBundleImport { bundle_id: String },
    BundleNotFound { bundle_id: String },
    MissingTestReceipt { bundle_id: String },
    TestReceiptFailed { bundle_id: String },
    TestReceiptDigestMismatch { bundle_id: String },
    ActivePointerMissing { bundle_id: String },
    RollbackRequiresActive { bundle_id: String },
    MissingLastKnownGood,
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
        Ok(Self {
            bundles: snapshot
                .bundles
                .into_iter()
                .map(|bundle| (bundle.bundle_id.clone(), bundle))
                .collect(),
            test_receipts: snapshot
                .test_receipts
                .into_iter()
                .map(|receipt| (receipt.bundle_id.clone(), receipt))
                .collect(),
            evaluation_receipts: snapshot
                .evaluation_receipts
                .into_iter()
                .map(|receipt| (receipt.evaluation_id.clone(), receipt))
                .collect(),
            shadow_diffs: snapshot
                .shadow_diffs
                .into_iter()
                .map(|diff| (diff.bundle_id.clone(), diff))
                .collect(),
            active_pointer: snapshot.active_pointer,
            last_known_good: snapshot.last_known_good,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundle(id: &str) -> PolicyBundleRecord {
        PolicyBundleRecord {
            bundle_id: id.to_string(),
            policy_id: "policy".to_string(),
            version: 1,
            engine_abi: "rhai-policy-engine-v1".to_string(),
            source: "1".to_string(),
            content_digest: format!("digest-{id}"),
            lifecycle: PolicyLifecycle::Candidate,
        }
    }

    fn receipt(id: &str) -> PolicyTestReceipt {
        PolicyTestReceipt {
            bundle_id: id.to_string(),
            test_id: format!("test-{id}"),
            content_digest: format!("digest-{id}"),
            passed: true,
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
}
