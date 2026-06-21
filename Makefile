.PHONY: vida-build vida-check vida-test vida-test-workspace vida-doc-test vida-release-package vida-dev-script-check vida-dev-quick vida-dev-smoke vida-run-help

vida-build:
	scripts/vida-dev-gate.cmd -Mode build-debug -Json

vida-check:
	scripts/vida-dev-gate.cmd -Mode quick -Json

vida-test:
	scripts/vida-dev-gate.cmd -Mode package-nextest -Json

vida-test-workspace:
	scripts/vida-dev-gate.cmd -Mode workspace-nextest -Json

vida-doc-test:
	scripts/vida-dev-gate.cmd -Mode doc-test -Json

vida-release-package:
	scripts/vida-dev-gate.cmd -Mode release-package -Json

vida-dev-script-check:
	scripts/vida-dev-gate.cmd -Mode script-check -Json

vida-dev-quick:
	scripts/vida-dev-gate.cmd -Mode quick -Json

vida-dev-smoke:
	scripts/vida-dev-gate.cmd -Mode runtime-smoke -Json

vida-run-help:
	cargo run -- --help
