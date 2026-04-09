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
    use serde::{Deserialize, Serialize};
    use taskflow_contracts::DependencyEdge;
    use taskflow_core::TaskId;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Row {
        name: String,
        count: u32,
    }

    #[test]
    fn round_trip_jsonl_line() {
        let row = Row {
            name: "taskflow".to_string(),
            count: 1,
        };
        let encoded = encode_line(&row).expect("encode");
        let decoded: Row = decode_line(&encoded).expect("decode");
        assert_eq!(decoded, row);
    }

    #[test]
    fn dependency_edge_matches_golden_fixture() {
        let edge = DependencyEdge {
            issue_id: TaskId::new("vida-rf1-taskflow-core"),
            depends_on_id: TaskId::new("vida-rf1-taskflow-tests"),
            dependency_type: "blocks".to_string(),
        };
        let encoded = encode_line(&edge).expect("encode");
        let expected = include_str!("../../../tests/golden/taskflow/dependency_edge.jsonl").trim();
        assert_eq!(encoded, expected);
    }
}
