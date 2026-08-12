#![no_main]

use common_format_toon::{render_toon_value_block, sanitize_toon_scalar};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _ = sanitize_toon_scalar(&input);
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let _ = render_toon_value_block("fuzz", &value);
    }
});
