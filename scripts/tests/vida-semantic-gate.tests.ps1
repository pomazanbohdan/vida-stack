Describe 'VIDA semantic local gates' {
    It 'declares P0/P1 and manual P2-P4 modes' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match 'semantic-focused'
        $script | Should -Match 'semantic-fuzz'
        $script | Should -Match 'semantic-loom'
        $script | Should -Match 'semantic-kani'
        $script | Should -Match 'semantic-miri'
    }

    It 'records explicit semantic blocked or not-applicable statuses' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match 'not_applicable'
        $script | Should -Match 'cargo-fuzz is not installed'
        $script | Should -Match 'Rust nightly toolchain is not installed or unavailable'
        $script | Should -Match 'nightly-2026-08-11'
        $script | Should -Match 'cargo-kani is not installed'
        $script | Should -Match 'Kani profile is Linux-only by project policy'
        $script | Should -Match 'cargo-miri is not installed'
    }

    It 'writes semantic artifacts below the ignored temporary root' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match '\.vida\\tmp\\semantic-testing'
        $script | Should -Match 'summary\.json'
    }

    It 'keeps all fuzz targets bounded and tied to the semantic artifact run' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match 'foreach \(\$target in @\("config_json", "jsonl_decoder", "cli_parser", "workflow_payload", "toon_render"\)\)'
        ([regex]::Matches($script, 'Invoke-Timed \("semantic-fuzz-run')).Count | Should -Be 1
        ([regex]::Matches($script, 'cargo-fuzz", "run", \$target')).Count | Should -Be 1
        $script | Should -Match 'runs=64'
        $script | Should -Match 'artifact_prefix='
        $script | Should -Match 'Initialize-SemanticFuzzCorpus'
    }

    It 'keeps Kani MSRV compatibility explicit and fail-closed' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match '--ignore-rust-version'
        $script | Should -Match 'CARGO_UNSTABLE_IGNORE_RUST_VERSION'
        $script | Should -Match 'installed cargo-kani lacks --ignore-rust-version'
    }

    It 'keeps the pre-push hook filename-independent' {
        $config = Get-Content (Join-Path $PSScriptRoot '..\..\.pre-commit-config.yaml') -Raw
        $config | Should -Match 'vida-semantic-prepush'
        $config | Should -Match 'stages: \[pre-push\]'
        $config | Should -Match 'pass_filenames: false'
    }
}
