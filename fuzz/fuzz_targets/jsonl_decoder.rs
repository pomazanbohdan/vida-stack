#![no_main]

use common_format_jsonl::decode_line as decode_common;
use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use taskflow_format_jsonl::decode_line as decode_taskflow;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _: Result<Value, _> = decode_common(&input);
    let _: Result<Value, _> = decode_taskflow(&input);
});
