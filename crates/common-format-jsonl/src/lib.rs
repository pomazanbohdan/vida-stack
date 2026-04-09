use serde::{Serialize, de::DeserializeOwned};

pub fn encode_line<T: Serialize>(value: &T) -> serde_json::Result<String> {
    serde_json::to_string(value)
}

pub fn decode_line<T: DeserializeOwned>(line: &str) -> serde_json::Result<T> {
    serde_json::from_str(line)
}

#[cfg(test)]
mod tests {
    use super::{decode_line, encode_line};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Row {
        name: String,
        count: u32,
    }

    #[test]
    fn round_trip_jsonl_line() {
        let row = Row {
            name: "common".to_string(),
            count: 1,
        };
        let encoded = encode_line(&row).expect("encode");
        let decoded: Row = decode_line(&encoded).expect("decode");
        assert_eq!(decoded, row);
    }
}
