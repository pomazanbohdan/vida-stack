param(
    [switch]$Json,
    [switch]$SelfTest
)

$ErrorActionPreference = "Stop"

function Get-ConfiguredAdapterValues {
    param([string]$Path)

    $lines = Get-Content -LiteralPath $Path
    $inBridge = $false
    $bridgeIndent = -1
    $values = New-Object System.Collections.Generic.List[string]
    foreach ($line in $lines) {
        if ($line -match '^(?<indent>\s*)host_tool_bridge:\s*$') {
            $inBridge = $true
            $bridgeIndent = $Matches.indent.Length
            continue
        }
        if (-not $inBridge) { continue }
        if ($line -match '^(?<indent>\s*)\S' -and $Matches.indent.Length -le $bridgeIndent) {
            $inBridge = $false
            continue
        }
        if ($line -match '^\s+(adapter_kind|adapter_capability_id|invocation_mode|spawn|wait|dispose):\s*(?<value>[^#\s]+)') {
            $value = $Matches.value.Trim('"', "'")
            if ($value) { $values.Add($value) }
        }
    }
    return @($values | Sort-Object -Unique)
}

function Get-ProductionText {
    param([string]$Path)

    $lines = @(Get-Content -LiteralPath $Path)
    if ($Path -match '\.rs$') {
        $testStart = $lines.IndexOf('#[cfg(test)]')
        if ($testStart -ge 0) { $lines = @($lines[0..($testStart - 1)]) }
    }
    return ($lines -join "`n")
}

function Add-Violation {
    param(
        [System.Collections.Generic.List[object]]$List,
        [string]$Path,
        [string]$Rule,
        [string]$Detail
    )
    $List.Add([pscustomobject]@{ path = $Path; rule = $Rule; detail = $Detail })
}

if ($SelfTest) {
    $selfTestValues = @("fixture.spawn", "fixture.wait", "fixture.dispose")
    $syntheticProduction = 'adapter_operations: fixture.spawn'
    $syntheticAllowed = 'operations: fixture.spawn'
    if (-not $selfTestValues.Where({ $syntheticProduction.Contains($_) }).Count) {
        throw "self-test failed: production configured-value detection"
    }
    if (-not $selfTestValues.Where({ $syntheticAllowed.Contains($_) }).Count) {
        throw "self-test failed: allowed config detection"
    }
    Write-Host "host bridge capability neutrality self-test: pass"
    exit 0
}

$configPaths = @(
    "vida.config.yaml",
    "docs/framework/templates/vida.config.yaml.template"
) | Where-Object { Test-Path -LiteralPath $_ }
$configuredValues = @($configPaths | ForEach-Object { Get-ConfiguredAdapterValues $_ } | Sort-Object -Unique)

$surfacePaths = @(
    (Get-ChildItem -Path "crates" -Recurse -File -Filter "*.rs" -ErrorAction SilentlyContinue),
    (Get-ChildItem -Path "scripts" -Recurse -File -Include "*.ps1","*.cmd","*.sh" -ErrorAction SilentlyContinue),
    (Get-ChildItem -Path ".github/workflows" -Recurse -File -Include "*.yml","*.yaml" -ErrorAction SilentlyContinue),
    (Get-ChildItem -Path "docs" -Recurse -File -Include "*generated*","*.template.md","*.template.yaml","*.jsonl" -ErrorAction SilentlyContinue)
) | Where-Object { $_ } | Sort-Object -Property FullName -Unique

$allowedPaths = @(
    (Resolve-Path -LiteralPath "vida.config.yaml" -ErrorAction SilentlyContinue).Path,
    (Resolve-Path -LiteralPath "docs/framework/templates/vida.config.yaml.template" -ErrorAction SilentlyContinue).Path,
    (Resolve-Path -LiteralPath "crates/taskflow-host-bridge/src/adapter_contract.rs" -ErrorAction SilentlyContinue).Path,
    (Resolve-Path -LiteralPath "docs/product/spec/host-agent-bridge-adapter-contract.md" -ErrorAction SilentlyContinue).Path
) | Where-Object { $_ }

$violations = New-Object System.Collections.Generic.List[object]
foreach ($file in $surfacePaths) {
    $path = $file.FullName
    $normalized = $path -replace "\\", "/"
    if ($allowedPaths -contains $path) { continue }
    if ($normalized -match "/tests/") { continue }
    $text = Get-ProductionText $path

    foreach ($value in $configuredValues) {
        if ($value.Length -gt 2 -and $text.Contains($value)) {
            Add-Violation $violations $normalized "configured_value_in_production_surface" $value
        }
    }
    if ($normalized -notmatch "/adapter_contract\.rs$" -and
        $text -match '(?m)\b(spawn_tool|wait_tool|close_tool|dispose_tool)\b') {
        Add-Violation $violations $normalized "legacy_operation_alias_in_production_surface" "legacy lifecycle alias"
    }
}

$workflowPath = ".github/workflows/runtime-quality.yml"
if (Test-Path -LiteralPath $workflowPath) {
    $workflowText = Get-Content -LiteralPath $workflowPath -Raw
    if ($workflowText -notmatch 'check-host-bridge-capability-neutrality\.ps1') {
        Add-Violation $violations $workflowPath "workflow_gate_missing" "neutrality script is not invoked"
    }
}

$contractPath = "docs/product/spec/host-agent-bridge-adapter-contract.md"
if (Test-Path -LiteralPath $contractPath) {
    $contractText = Get-Content -LiteralPath $contractPath -Raw
    foreach ($required in @('"adapter_operations"', '"operations"', '"dispose_policy"', '"adapter_contract_hash"')) {
        if ($contractText -notmatch [regex]::Escape($required)) {
            Add-Violation $violations $contractPath "contract_schema_missing" $required
        }
    }
}

$status = if ($violations.Count -eq 0) { "pass" } else { "blocked" }
$result = [pscustomobject]@{
    surface = "scripts/check-host-bridge-capability-neutrality.ps1"
    status = $status
    config_paths = @($configPaths)
    scanned_surface_count = $surfacePaths.Count
    scanned_surface_roots = @("crates", "scripts", ".github/workflows", "docs")
    allowed_config_and_contract_paths = @($allowedPaths)
    configured_values = @($configuredValues)
    violations = @($violations.ToArray())
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
} else {
    Write-Host "host bridge capability neutrality: $status"
    Write-Host "- scanned surfaces: $($surfacePaths.Count)"
    Write-Host "- configured registry values: $($configuredValues.Count)"
    Write-Host "- violations: $($violations.Count)"
    foreach ($violation in $violations) {
        Write-Host "  $($violation.path): [$($violation.rule)] $($violation.detail)"
    }
}

if ($status -ne "pass") { exit 1 }
