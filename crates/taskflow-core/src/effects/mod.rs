use serde::{Deserialize, Serialize};

pub const MODULE: &str = "effects";
pub const EFFECT_INTENT_SCHEMA_VERSION: u32 = 1;
pub const EFFECT_RECORD_SCHEMA_VERSION: u32 = 1;

pub trait Clock {
    fn now(&self) -> String;
}

pub trait IdGenerator {
    fn stable_id(&self, parts: &[&str]) -> String;
}

pub trait ArtifactHasher {
    fn artifact_hash(&self, bytes: &[u8]) -> String;
}

pub trait PolicyEvaluator {
    fn allows(&self, action: &str, resource: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum EffectIntentKind {
    ArtifactWrite {
        artifact_path: String,
        content_hash: String,
    },
    PacketMaterialization {
        packet_id: String,
    },
    HostDispatch {
        dispatch_target: String,
        packet_id: String,
    },
    ProjectionRebuild {
        projection_id: String,
    },
    Timer {
        timer_id: String,
        fire_at: String,
    },
    Notification {
        channel: String,
        message_key: String,
    },
    Cleanup {
        resource_id: String,
    },
}

impl EffectIntentKind {
    #[must_use]
    pub fn discriminator(&self) -> &'static str {
        match self {
            Self::ArtifactWrite { .. } => "artifact_write",
            Self::PacketMaterialization { .. } => "packet_materialization",
            Self::HostDispatch { .. } => "host_dispatch",
            Self::ProjectionRebuild { .. } => "projection_rebuild",
            Self::Timer { .. } => "timer",
            Self::Notification { .. } => "notification",
            Self::Cleanup { .. } => "cleanup",
        }
    }

    fn stable_parts(&self) -> Vec<&str> {
        match self {
            Self::ArtifactWrite {
                artifact_path,
                content_hash,
            } => vec![artifact_path.as_str(), content_hash.as_str()],
            Self::PacketMaterialization { packet_id } => vec![packet_id.as_str()],
            Self::HostDispatch {
                dispatch_target,
                packet_id,
            } => vec![dispatch_target.as_str(), packet_id.as_str()],
            Self::ProjectionRebuild { projection_id } => vec![projection_id.as_str()],
            Self::Timer { timer_id, fire_at } => vec![timer_id.as_str(), fire_at.as_str()],
            Self::Notification {
                channel,
                message_key,
            } => vec![channel.as_str(), message_key.as_str()],
            Self::Cleanup { resource_id } => vec![resource_id.as_str()],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectIntent {
    pub schema_version: u32,
    pub intent_id: String,
    pub bounded_unit_id: String,
    pub sequence: u64,
    pub idempotency_key: String,
    pub kind: EffectIntentKind,
}

impl EffectIntent {
    #[must_use]
    pub fn new(
        bounded_unit_id: impl Into<String>,
        sequence: u64,
        kind: EffectIntentKind,
        id_generator: &dyn IdGenerator,
    ) -> Self {
        let bounded_unit_id = bounded_unit_id.into();
        let sequence_text = sequence.to_string();
        let mut parts = vec![
            "effect_intent",
            bounded_unit_id.as_str(),
            sequence_text.as_str(),
            kind.discriminator(),
        ];
        let stable_parts = kind.stable_parts();
        parts.extend(stable_parts.iter().copied());
        let intent_id = id_generator.stable_id(&parts);
        let idempotency_key = id_generator.stable_id(&["effect_idempotency", intent_id.as_str()]);
        Self {
            schema_version: EFFECT_INTENT_SCHEMA_VERSION,
            intent_id,
            bounded_unit_id,
            sequence,
            idempotency_key,
            kind,
        }
    }
}

#[must_use]
pub fn intent_is_idempotent(left: &EffectIntent, right: &EffectIntent) -> bool {
    left.intent_id == right.intent_id && left.idempotency_key == right.idempotency_key
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectLifecycleStatus {
    Requested,
    OutboxPending,
    Leased,
    Completed,
    FailedRetryable,
    FailedTerminal,
    Compensated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectRecord {
    pub schema_version: u32,
    pub intent: EffectIntent,
    pub status: EffectLifecycleStatus,
    pub lease_owner: Option<String>,
    pub lease_expires_at: Option<String>,
    pub attempt_count: u32,
    pub receipt_ref: Option<String>,
    pub failure_code: Option<String>,
}

impl EffectRecord {
    #[must_use]
    pub fn requested(intent: EffectIntent) -> Self {
        Self {
            schema_version: EFFECT_RECORD_SCHEMA_VERSION,
            intent,
            status: EffectLifecycleStatus::Requested,
            lease_owner: None,
            lease_expires_at: None,
            attempt_count: 0,
            receipt_ref: None,
            failure_code: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectProcessingCommand {
    Enqueue {
        intent: EffectIntent,
    },
    Lease {
        owner: String,
        lease_expires_at: String,
    },
    Complete {
        receipt_ref: String,
    },
    FailRetryable {
        failure_code: String,
    },
    FailTerminal {
        failure_code: String,
    },
    Compensate {
        receipt_ref: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectProcessingEvent {
    Enqueued,
    LeaseGranted {
        owner: String,
        lease_expires_at: String,
    },
    Completed {
        receipt_ref: String,
    },
    FailedRetryable {
        failure_code: String,
    },
    FailedTerminal {
        failure_code: String,
    },
    Compensated {
        receipt_ref: String,
    },
    DuplicateIgnored {
        status: EffectLifecycleStatus,
    },
    Rejected {
        blocker_code: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectProcessingDecision {
    pub admitted: bool,
    pub record: EffectRecord,
    pub events: Vec<EffectProcessingEvent>,
    pub blocker_codes: Vec<String>,
}

impl EffectProcessingDecision {
    fn admitted(record: EffectRecord, events: Vec<EffectProcessingEvent>) -> Self {
        Self {
            admitted: true,
            record,
            events,
            blocker_codes: Vec::new(),
        }
    }

    fn rejected(record: EffectRecord, blocker_code: impl Into<String>) -> Self {
        let blocker_code = blocker_code.into();
        Self {
            admitted: false,
            record,
            events: vec![EffectProcessingEvent::Rejected {
                blocker_code: blocker_code.clone(),
            }],
            blocker_codes: vec![blocker_code],
        }
    }
}

#[must_use]
pub fn decide_effect_processing(
    current: Option<&EffectRecord>,
    command: EffectProcessingCommand,
) -> EffectProcessingDecision {
    match command {
        EffectProcessingCommand::Enqueue { intent } => match current {
            None => {
                let mut record = EffectRecord::requested(intent);
                record.status = EffectLifecycleStatus::OutboxPending;
                EffectProcessingDecision::admitted(record, vec![EffectProcessingEvent::Enqueued])
            }
            Some(record) if intent_is_idempotent(&record.intent, &intent) => {
                EffectProcessingDecision::admitted(
                    record.clone(),
                    vec![EffectProcessingEvent::DuplicateIgnored {
                        status: record.status,
                    }],
                )
            }
            Some(record) => {
                EffectProcessingDecision::rejected(record.clone(), "effect_idempotency_conflict")
            }
        },
        EffectProcessingCommand::Lease {
            owner,
            lease_expires_at,
        } => {
            let Some(current) = current else {
                return EffectProcessingDecision::rejected(
                    empty_rejected_effect_record(),
                    "effect_record_missing",
                );
            };
            if current.status != EffectLifecycleStatus::OutboxPending
                && current.status != EffectLifecycleStatus::FailedRetryable
            {
                return EffectProcessingDecision::rejected(current.clone(), "effect_not_leaseable");
            }
            let mut record = current.clone();
            record.status = EffectLifecycleStatus::Leased;
            record.lease_owner = Some(owner.clone());
            record.lease_expires_at = Some(lease_expires_at.clone());
            record.attempt_count = record.attempt_count.saturating_add(1);
            EffectProcessingDecision::admitted(
                record,
                vec![EffectProcessingEvent::LeaseGranted {
                    owner,
                    lease_expires_at,
                }],
            )
        }
        EffectProcessingCommand::Complete { receipt_ref } => {
            transition_leased_effect(current, EffectLifecycleStatus::Completed, |record| {
                record.receipt_ref = Some(receipt_ref.clone());
                EffectProcessingEvent::Completed { receipt_ref }
            })
        }
        EffectProcessingCommand::FailRetryable { failure_code } => {
            transition_leased_effect(current, EffectLifecycleStatus::FailedRetryable, |record| {
                record.failure_code = Some(failure_code.clone());
                EffectProcessingEvent::FailedRetryable { failure_code }
            })
        }
        EffectProcessingCommand::FailTerminal { failure_code } => {
            transition_leased_effect(current, EffectLifecycleStatus::FailedTerminal, |record| {
                record.failure_code = Some(failure_code.clone());
                EffectProcessingEvent::FailedTerminal { failure_code }
            })
        }
        EffectProcessingCommand::Compensate { receipt_ref } => {
            let Some(current) = current else {
                return EffectProcessingDecision::rejected(
                    empty_rejected_effect_record(),
                    "effect_record_missing",
                );
            };
            if current.status != EffectLifecycleStatus::Completed {
                return EffectProcessingDecision::rejected(
                    current.clone(),
                    "effect_not_compensatable",
                );
            }
            let mut record = current.clone();
            record.status = EffectLifecycleStatus::Compensated;
            record.receipt_ref = Some(receipt_ref.clone());
            EffectProcessingDecision::admitted(
                record,
                vec![EffectProcessingEvent::Compensated { receipt_ref }],
            )
        }
    }
}

fn transition_leased_effect(
    current: Option<&EffectRecord>,
    status: EffectLifecycleStatus,
    event: impl FnOnce(&mut EffectRecord) -> EffectProcessingEvent,
) -> EffectProcessingDecision {
    let Some(current) = current else {
        return EffectProcessingDecision::rejected(
            empty_rejected_effect_record(),
            "effect_record_missing",
        );
    };
    if current.status != EffectLifecycleStatus::Leased {
        return EffectProcessingDecision::rejected(current.clone(), "effect_not_leased");
    }
    let mut record = current.clone();
    record.status = status;
    let event = event(&mut record);
    EffectProcessingDecision::admitted(record, vec![event])
}

fn empty_rejected_effect_record() -> EffectRecord {
    EffectRecord::requested(EffectIntent {
        schema_version: EFFECT_INTENT_SCHEMA_VERSION,
        intent_id: String::new(),
        bounded_unit_id: String::new(),
        sequence: 0,
        idempotency_key: String::new(),
        kind: EffectIntentKind::Cleanup {
            resource_id: String::new(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{
        EFFECT_INTENT_SCHEMA_VERSION, EFFECT_RECORD_SCHEMA_VERSION, EffectIntent, EffectIntentKind,
        EffectLifecycleStatus, EffectProcessingCommand, EffectProcessingEvent, IdGenerator, MODULE,
        decide_effect_processing, intent_is_idempotent,
    };
    use proptest::prelude::*;

    struct StableGenerator;

    impl IdGenerator for StableGenerator {
        fn stable_id(&self, parts: &[&str]) -> String {
            parts.join(":")
        }
    }

    #[test]
    fn effect_intent_ids_are_deterministic_for_same_payload() {
        let generator = StableGenerator;
        let left = EffectIntent::new(
            "run-1",
            1,
            EffectIntentKind::HostDispatch {
                dispatch_target: "developer".to_string(),
                packet_id: "packet-1".to_string(),
            },
            &generator,
        );
        let right = EffectIntent::new(
            "run-1",
            1,
            EffectIntentKind::HostDispatch {
                dispatch_target: "developer".to_string(),
                packet_id: "packet-1".to_string(),
            },
            &generator,
        );

        assert_eq!(left.schema_version, EFFECT_INTENT_SCHEMA_VERSION);
        assert_eq!(left.intent_id, right.intent_id);
        assert!(intent_is_idempotent(&left, &right));
    }

    #[test]
    fn effect_intent_ids_change_when_payload_changes() {
        let generator = StableGenerator;
        let artifact = EffectIntent::new(
            "run-1",
            1,
            EffectIntentKind::ArtifactWrite {
                artifact_path: "packets/run-1.json".to_string(),
                content_hash: "hash-a".to_string(),
            },
            &generator,
        );
        let cleanup = EffectIntent::new(
            "run-1",
            1,
            EffectIntentKind::Cleanup {
                resource_id: "packets/run-1.json".to_string(),
            },
            &generator,
        );

        assert_ne!(artifact.intent_id, cleanup.intent_id);
    }

    #[test]
    fn effect_enqueue_is_idempotent_for_same_intent_and_rejects_conflicting_intent() {
        let generator = StableGenerator;
        let intent = host_dispatch_intent(&generator, "packet-1");
        let first = decide_effect_processing(
            None,
            EffectProcessingCommand::Enqueue {
                intent: intent.clone(),
            },
        );

        assert!(first.admitted);
        assert_eq!(first.record.schema_version, EFFECT_RECORD_SCHEMA_VERSION);
        assert_eq!(first.record.status, EffectLifecycleStatus::OutboxPending);
        assert_eq!(first.events, vec![EffectProcessingEvent::Enqueued]);

        let duplicate = decide_effect_processing(
            Some(&first.record),
            EffectProcessingCommand::Enqueue {
                intent: intent.clone(),
            },
        );
        assert!(duplicate.admitted);
        assert_eq!(
            duplicate.events,
            vec![EffectProcessingEvent::DuplicateIgnored {
                status: EffectLifecycleStatus::OutboxPending
            }]
        );

        let conflicting = decide_effect_processing(
            Some(&first.record),
            EffectProcessingCommand::Enqueue {
                intent: host_dispatch_intent(&generator, "packet-2"),
            },
        );
        assert!(!conflicting.admitted);
        assert_eq!(
            conflicting.blocker_codes,
            vec!["effect_idempotency_conflict"]
        );
    }

    #[test]
    fn effect_lifecycle_leases_completes_and_blocks_duplicate_completion() {
        let generator = StableGenerator;
        let enqueued = decide_effect_processing(
            None,
            EffectProcessingCommand::Enqueue {
                intent: host_dispatch_intent(&generator, "packet-1"),
            },
        );
        let leased = decide_effect_processing(
            Some(&enqueued.record),
            EffectProcessingCommand::Lease {
                owner: "worker-1".to_string(),
                lease_expires_at: "2026-06-22T00:05:00Z".to_string(),
            },
        );

        assert!(leased.admitted);
        assert_eq!(leased.record.status, EffectLifecycleStatus::Leased);
        assert_eq!(leased.record.attempt_count, 1);
        assert_eq!(
            leased.events,
            vec![EffectProcessingEvent::LeaseGranted {
                owner: "worker-1".to_string(),
                lease_expires_at: "2026-06-22T00:05:00Z".to_string(),
            }]
        );

        let completed = decide_effect_processing(
            Some(&leased.record),
            EffectProcessingCommand::Complete {
                receipt_ref: "receipts/effect-1.json".to_string(),
            },
        );
        assert!(completed.admitted);
        assert_eq!(completed.record.status, EffectLifecycleStatus::Completed);
        assert_eq!(
            completed.record.receipt_ref.as_deref(),
            Some("receipts/effect-1.json")
        );

        let duplicate_complete = decide_effect_processing(
            Some(&completed.record),
            EffectProcessingCommand::Complete {
                receipt_ref: "receipts/effect-1.json".to_string(),
            },
        );
        assert!(!duplicate_complete.admitted);
        assert_eq!(duplicate_complete.blocker_codes, vec!["effect_not_leased"]);
    }

    #[test]
    fn retryable_failure_can_be_leased_again_and_completed_effect_can_be_compensated() {
        let generator = StableGenerator;
        let enqueued = decide_effect_processing(
            None,
            EffectProcessingCommand::Enqueue {
                intent: host_dispatch_intent(&generator, "packet-1"),
            },
        );
        let leased = decide_effect_processing(
            Some(&enqueued.record),
            EffectProcessingCommand::Lease {
                owner: "worker-1".to_string(),
                lease_expires_at: "2026-06-22T00:05:00Z".to_string(),
            },
        );
        let retryable = decide_effect_processing(
            Some(&leased.record),
            EffectProcessingCommand::FailRetryable {
                failure_code: "transient_io".to_string(),
            },
        );
        assert_eq!(
            retryable.record.status,
            EffectLifecycleStatus::FailedRetryable
        );

        let leased_again = decide_effect_processing(
            Some(&retryable.record),
            EffectProcessingCommand::Lease {
                owner: "worker-2".to_string(),
                lease_expires_at: "2026-06-22T00:10:00Z".to_string(),
            },
        );
        assert!(leased_again.admitted);
        assert_eq!(leased_again.record.attempt_count, 2);

        let completed = decide_effect_processing(
            Some(&leased_again.record),
            EffectProcessingCommand::Complete {
                receipt_ref: "receipts/effect-1.json".to_string(),
            },
        );
        let compensated = decide_effect_processing(
            Some(&completed.record),
            EffectProcessingCommand::Compensate {
                receipt_ref: "receipts/compensate-1.json".to_string(),
            },
        );

        assert!(compensated.admitted);
        assert_eq!(
            compensated.record.status,
            EffectLifecycleStatus::Compensated
        );
    }

    #[test]
    fn effects_module_is_registered() {
        assert_eq!(MODULE, "effects");
    }

    #[test]
    fn effect_commands_fail_closed_without_a_record_or_lease() {
        let missing_lease = decide_effect_processing(
            None,
            EffectProcessingCommand::Lease {
                owner: "worker".to_string(),
                lease_expires_at: "2026-06-22T00:00:00Z".to_string(),
            },
        );
        assert!(!missing_lease.admitted);
        assert_eq!(missing_lease.blocker_codes, vec!["effect_record_missing"]);

        let generator = StableGenerator;
        let enqueued = decide_effect_processing(
            None,
            EffectProcessingCommand::Enqueue {
                intent: host_dispatch_intent(&generator, "packet-1"),
            },
        );
        let completion = decide_effect_processing(
            Some(&enqueued.record),
            EffectProcessingCommand::Complete {
                receipt_ref: "receipt".to_string(),
            },
        );
        assert!(!completion.admitted);
        assert_eq!(completion.blocker_codes, vec!["effect_not_leased"]);
    }

    fn host_dispatch_intent(generator: &StableGenerator, packet_id: &str) -> EffectIntent {
        EffectIntent::new(
            "run-1",
            1,
            EffectIntentKind::HostDispatch {
                dispatch_target: "developer".to_string(),
                packet_id: packet_id.to_string(),
            },
            generator,
        )
    }

    proptest! {
        #[test]
        fn effect_intent_ids_are_stable_for_repeated_construction(
            bounded_unit_id in "[a-z][a-z0-9-]{0,16}",
            sequence in 0_u64..10_000,
            dispatch_target in "[a-z][a-z0-9-]{0,16}",
            packet_id in "[a-z][a-z0-9-]{0,16}",
        ) {
            let generator = StableGenerator;
            let kind = EffectIntentKind::HostDispatch {
                dispatch_target,
                packet_id,
            };

            let left = EffectIntent::new(bounded_unit_id.clone(), sequence, kind.clone(), &generator);
            let right = EffectIntent::new(bounded_unit_id, sequence, kind, &generator);

            prop_assert_eq!(&left.intent_id, &right.intent_id);
            prop_assert_eq!(&left.idempotency_key, &right.idempotency_key);
            prop_assert!(intent_is_idempotent(&left, &right));
        }
    }
}
