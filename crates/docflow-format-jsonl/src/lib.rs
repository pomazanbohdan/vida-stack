use serde::{Serialize, de::DeserializeOwned};

pub fn encode_line<T: Serialize>(value: &T) -> serde_json::Result<String> {
    common_format_jsonl::encode_line(value)
}

pub fn decode_line<T: DeserializeOwned>(line: &str) -> serde_json::Result<T> {
    common_format_jsonl::decode_line(line)
}

#[cfg(test)]
mod tests {
    use super::{decode_line, encode_line};
    use docflow_contracts::RegistryRow;
    use docflow_core::ArtifactPath;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Row {
        artifact: String,
        verdict: String,
    }

    #[test]
    fn round_trip_jsonl_line() {
        let row = Row {
            artifact: "product/spec/foo".to_string(),
            verdict: "ok".to_string(),
        };
        let encoded = encode_line(&row).expect("encode");
        let decoded: Row = decode_line(&encoded).expect("decode");
        assert_eq!(decoded, row);
    }

    #[test]
    fn registry_row_matches_golden_fixture() {
        let row = RegistryRow {
            artifact_path: ArtifactPath(
                "docs/product/spec/taskflow-v1-runtime-modernization-plan.md".into(),
            ),
            artifact_type: "product_spec".into(),
        };
        let encoded = encode_line(&row).expect("encode");
        let expected = include_str!("../../../tests/golden/docflow/registry_row.jsonl").trim();
        assert_eq!(encoded, expected);
    }
}
