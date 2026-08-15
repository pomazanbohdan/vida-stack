Describe 'vida-dev-gate quality modes' {
    It 'declares deterministic quality modes and risk thresholds' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match 'quality-cycle'
        $script | Should -Match 'quality-pack'
        $script | Should -Match 'critical = 98\.0'
        $script | Should -Match 'runtime = 95\.0'
        $script | Should -Match 'standard = 93\.0'
    }

    It 'keeps typed blockers separate from quality status' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match 'typed_blocker'
        $script | Should -Match 'metric_blocker'
    }

    It 'writes quality artifacts through the reparse-safe state writer' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $qualityWriter = [regex]::Match(
            $script,
            '(?s)function New-QualityGateReport \{(?<body>.*?)\r?\n\}\r?\n\r?\nfunction Invoke-QualitySequence',
            [System.Text.RegularExpressions.RegexOptions]::CultureInvariant
        ).Groups['body'].Value

        $qualityWriter | Should -Not -BeNullOrEmpty
        $qualityWriter | Should -Match '\| Write-SafeStateFile -Path \$planPath'
        $qualityWriter | Should -Match '\| Write-SafeStateFile -Path \$reportPath'
        $qualityWriter | Should -Not -Match 'Set-Content'
    }

    It 'orchestrates one-pass quality proof sequence' {
        $script = Get-Content (Join-Path $PSScriptRoot '..\vida-dev-gate.ps1') -Raw
        $script | Should -Match 'quality-script-check'
        $script | Should -Match 'quality-package-nextest'
        $script | Should -Match 'quality-workspace-nextest'
        $script | Should -Match 'quality-doc-test'
        $script | Should -Match 'quality-focused-nextest'
        $script | Should -Match 'quality-mutation-audit'
        $script | Should -Match 'RunMutationAudit'
        $script | Should -Match 'PlanOnly'
    }
}
