.PHONY: vida-build vida-check vida-test vida-test-workspace vida-dev-script-check vida-dev-quick vida-dev-smoke vida-run-help

vida-build:
	cargo build --locked -p vida -p taskflow-cli -p docflow-cli -p vida-pi-agent

vida-check:
	cargo check --locked -p vida

vida-test:
	cargo nextest run --locked -p vida --profile default

vida-test-workspace:
	cargo nextest run --locked --workspace --profile ci

vida-dev-script-check:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode script-check -Json

vida-dev-quick:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode quick -Json

vida-dev-smoke:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode runtime-smoke -Json

vida-run-help:
	cargo run -- --help
