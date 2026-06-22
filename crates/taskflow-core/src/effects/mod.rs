use serde::{Deserialize, Serialize};

pub const MODULE: &str = "effects";
pub const EFFECT_INTENT_SCHEMA_VERSION: u32 = 1;

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

#[cfg(test)]
mod tests {
    use super::{
        EFFECT_INTENT_SCHEMA_VERSION, EffectIntent, EffectIntentKind, IdGenerator, MODULE,
        intent_is_idempotent,
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
    fn effects_module_is_registered() {
        assert_eq!(MODULE, "effects");
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
