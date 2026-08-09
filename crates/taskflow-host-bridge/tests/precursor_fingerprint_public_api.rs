use taskflow_host_bridge::{
    BLOCKER_PRECURSOR_FINGERPRINT_MISSING, HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION,
    HOST_BRIDGE_PRECURSOR_RECEIPT_FIELDS, HostBridgePrecursorFingerprintV1,
};

#[test]
fn precursor_fingerprint_v1_contract_is_public() {
    assert_eq!(
        HOST_BRIDGE_PRECURSOR_FINGERPRINT_SCHEMA_VERSION,
        "host-bridge-precursor-fingerprint-v1"
    );
    assert_eq!(HOST_BRIDGE_PRECURSOR_RECEIPT_FIELDS.len(), 29);
    assert!(HOST_BRIDGE_PRECURSOR_RECEIPT_FIELDS.contains(&"policy_bundle_ref"));
    assert_eq!(
        HostBridgePrecursorFingerprintV1::from_value(None)
            .expect_err("missing fingerprint must fail closed"),
        BLOCKER_PRECURSOR_FINGERPRINT_MISSING
    );
}
