.PHONY: vida-build vida-check vida-test vida-test-workspace vida-doc-test vida-release-package vida-dev-script-check vida-dev-quick vida-dev-smoke vida-run-help

vida-build:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode build-debug -Json

vida-check:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode quick -Json

vida-test:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode package-nextest -Json

vida-test-workspace:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode workspace-nextest -Json

vida-doc-test:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode doc-test -Json

vida-release-package:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode release-package -Json

vida-dev-script-check:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode script-check -Json

vida-dev-quick:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode quick -Json

vida-dev-smoke:
	pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/vida-dev-gate.ps1 -Mode runtime-smoke -Json

vida-run-help:
	cargo run -- --help
