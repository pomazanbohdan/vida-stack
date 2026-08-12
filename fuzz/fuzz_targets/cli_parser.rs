#![no_main]

use clap::Parser;
use docflow_cli::Cli;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut args = vec!["docflow".to_string()];
    for token in data.split(|byte| *byte == 0).take(32) {
        let token = String::from_utf8_lossy(token);
        if !token.is_empty() {
            args.push(token.into_owned());
        }
    }
    let _ = Cli::try_parse_from(args);
});
