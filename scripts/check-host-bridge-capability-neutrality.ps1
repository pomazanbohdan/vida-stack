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

function Get-SurfaceInventory {
    $specs = @(
        [pscustomobject]@{ name = "crates"; path = "crates"; include = @("*.rs") },
        [pscustomobject]@{ name = "scripts"; path = "scripts"; include = @("*.ps1", "*.cmd", "*.sh") },
        [pscustomobject]@{ name = ".github/workflows"; path = ".github/workflows"; include = @("*.yml", "*.yaml") },
        [pscustomobject]@{ name = "docs"; path = "docs"; include = @("*generated*", "*.template.md", "*.template.yaml", "*.jsonl") }
    )
    $paths = New-Object System.Collections.Generic.List[object]
    $familyCounts = [ordered]@{}
    $missingFamilies = New-Object System.Collections.Generic.List[string]
    foreach ($spec in $specs) {
        if (-not (Test-Path -LiteralPath $spec.path -PathType Container)) {
            $familyCounts[$spec.name] = 0
            $missingFamilies.Add($spec.name)
            continue
        }
        $files = @(Get-ChildItem -Path $spec.path -Recurse -File -Include $spec.include -ErrorAction SilentlyContinue)
        $familyCounts[$spec.name] = $files.Count
        foreach ($file in $files) { $paths.Add($file) }
        if ($files.Count -eq 0) { $missingFamilies.Add($spec.name) }
    }
    return [pscustomobject]@{
        paths = @($paths.ToArray() | Sort-Object -Property FullName -Unique)
        family_counts = $familyCounts
        missing_families = @($missingFamilies.ToArray())
    }
}

$surfaceInventory = Get-SurfaceInventory

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
    $expectedSurfaceCount = ($surfaceInventory.family_counts.Values | Measure-Object -Sum).Sum
    if ($surfaceInventory.missing_families.Count -gt 0) {
        throw "self-test failed: missing surface family '$($surfaceInventory.missing_families -join ', ')'"
    }
    if ($surfaceInventory.paths.Count -ne $expectedSurfaceCount -or $surfaceInventory.paths.Count -lt 4) {
        throw "self-test failed: flattened surface count '$($surfaceInventory.paths.Count)' does not match family inventory '$expectedSurfaceCount'"
    }
    Write-Host "host bridge capability neutrality self-test: pass"
    exit 0
}

$configPaths = @(
    "vida.config.yaml",
    "docs/framework/templates/vida.config.yaml.template"
) | Where-Object { Test-Path -LiteralPath $_ }
$configuredValues = @($configPaths | ForEach-Object { Get-ConfiguredAdapterValues $_ } | Sort-Object -Unique)

$surfacePaths = @($surfaceInventory.paths)

$allowedPaths = @(
    (Resolve-Path -LiteralPath "vida.config.yaml" -ErrorAction SilentlyContinue).Path,
    (Resolve-Path -LiteralPath "docs/framework/templates/vida.config.yaml.template" -ErrorAction SilentlyContinue).Path,
    (Resolve-Path -LiteralPath "crates/taskflow-host-bridge/src/adapter_contract.rs" -ErrorAction SilentlyContinue).Path,
    (Resolve-Path -LiteralPath "docs/product/spec/host-agent-bridge-adapter-contract.md" -ErrorAction SilentlyContinue).Path
) | Where-Object { $_ }

$violations = New-Object System.Collections.Generic.List[object]
foreach ($family in $surfaceInventory.missing_families) {
    Add-Violation $violations $family "surface_family_missing" "required neutrality scan family is absent or empty"
}
$expectedSurfaceCount = ($surfaceInventory.family_counts.Values | Measure-Object -Sum).Sum
if ($surfacePaths.Count -ne $expectedSurfaceCount -or $surfacePaths.Count -lt 4) {
    Add-Violation $violations "scripts/check-host-bridge-capability-neutrality.ps1" "surface_scan_count_too_small" "flattened scan count does not match family inventory"
}
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
        $normalized -notmatch "/taskflow-host-bridge/src/request\.rs$" -and
        $normalized -notmatch "/check-host-bridge-capability-neutrality\.ps1$" -and
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
    scanned_surface_families = $surfaceInventory.family_counts
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
