#![no_main]

use docflow_config::load_from_json_str;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    let _ = load_from_json_str(&input);
});
