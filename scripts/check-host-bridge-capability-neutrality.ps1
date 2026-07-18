param(
    [switch]$Json
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
        if (-not $inBridge) {
            continue
        }
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

function Get-ProductionLines {
    param([string]$Path)

    $lines = @(Get-Content -LiteralPath $Path)
    $testStart = $lines.IndexOf('#[cfg(test)]')
    if ($testStart -ge 0) {
        return @($lines[0..($testStart - 1)])
    }
    return $lines
}

$configPaths = @(
    "vida.config.yaml",
    "docs/framework/templates/vida.config.yaml.template"
) | Where-Object { Test-Path -LiteralPath $_ }
$ownedPaths = @(
    "crates/taskflow-host-bridge/src/request.rs",
    "crates/taskflow-host-bridge/src/adapter_payload.rs",
    "crates/taskflow-host-bridge/src/adapter_contract.rs",
    "crates/vida/src/agent_dispatch_surface.rs",
    "crates/vida/src/runtime_dispatch_execution.rs",
    "crates/vida/src/project_activator_surface.rs"
) | Where-Object { Test-Path -LiteralPath $_ }

$configuredValues = @($configPaths | ForEach-Object { Get-ConfiguredAdapterValues $_ } | Sort-Object -Unique)
$violations = New-Object System.Collections.Generic.List[object]
foreach ($path in $ownedPaths) {
    $lineNumber = 0
    foreach ($line in (Get-ProductionLines $path)) {
        $lineNumber++
        foreach ($value in $configuredValues) {
            if ($value.Length -gt 2 -and $line.Contains($value)) {
                $violations.Add([pscustomobject]@{
                    path = $path
                    line = $lineNumber
                    configured_value = $value
                    text = $line.Trim()
                })
            }
        }
    }
}

$status = if ($violations.Count -eq 0) { "pass" } else { "blocked" }
$violationArray = @($violations.ToArray())
$result = [pscustomobject]@{
    surface = "scripts/check-host-bridge-capability-neutrality.ps1"
    status = $status
    config_paths = $configPaths
    scanned_paths = $ownedPaths
    configured_values = $configuredValues
    violations = $violationArray
}

if ($Json) {
    $result | ConvertTo-Json -Depth 8
} else {
    Write-Host "host bridge capability neutrality: $status"
    Write-Host "- configured adapter values: $($configuredValues.Count)"
    Write-Host "- production violations: $($violations.Count)"
    foreach ($violation in $violations) {
        Write-Host "  $($violation.path):$($violation.line): $($violation.text) [configured=$($violation.configured_value)]"
    }
}

if ($status -ne "pass") {
    exit 1
}
