.PHONY: vida-build vida-test vida-run-help

vida-build:
	cargo build

vida-test:
	cargo test

vida-run-help:
	cargo run -- --help
