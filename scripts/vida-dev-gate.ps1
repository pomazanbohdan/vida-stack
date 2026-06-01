param(
    [ValidateSet("quick", "runtime-smoke", "release-install")]
    [string]$Mode = "quick",
    [string]$TestFilter = "",
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
$Records = New-Object System.Collections.Generic.List[object]

function Invoke-Timed {
    param(
        [string]$OperationId,
        [string[]]$Command
    )

    $started = Get-Date
    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $exitCode = 0
    $artifactRefs = @()
    $exe = $Command[0]
    $args = @()
    if ($Command.Length -gt 1) {
        $args = $Command[1..($Command.Length - 1)]
    }
    try {
        if ($Json) {
            $logDir = Join-Path $RootDir ".vida\data\state\command-timing"
            New-Item -ItemType Directory -Force -Path $logDir | Out-Null
            $safeId = $OperationId -replace '[^A-Za-z0-9_.-]', '-'
            $logPath = Join-Path $logDir ("{0}-{1:yyyyMMddHHmmssfff}.log" -f $safeId, $started)
            $previousErrorActionPreference = $ErrorActionPreference
            $ErrorActionPreference = "Continue"
            try {
                & $exe @args *> $logPath
            } finally {
                $ErrorActionPreference = $previousErrorActionPreference
            }
            $artifactRefs = @($logPath)
        } else {
            & $exe @args
        }
        $exitCode = $LASTEXITCODE
        if ($null -eq $exitCode) {
            $exitCode = 0
        }
    } catch {
        $exitCode = 1
        throw
    } finally {
        $sw.Stop()
        $Records.Add([pscustomobject]@{
            operation_id = $OperationId
            command_or_surface = ($Command -join " ")
            cwd_or_context = $RootDir
            started_at = $started.ToString("o")
            duration_ms = [int64]$sw.ElapsedMilliseconds
            exit_status = $(if ($exitCode -eq 0) { "pass" } else { "fail" })
            classification = $(if ($sw.ElapsedMilliseconds -le 2000) { "fast" } elseif ($sw.ElapsedMilliseconds -le 5000) { "watch" } else { "long_gate_expected" })
            artifact_refs = $artifactRefs
        })
    }
    if ($exitCode -ne 0) {
        exit $exitCode
    }
}

Push-Location $RootDir
try {
    if ($Mode -eq "quick") {
        Invoke-Timed "cargo-fmt-check" @("cargo", "fmt", "-p", "vida", "--", "--check")
        if ($TestFilter.Trim().Length -gt 0) {
            Invoke-Timed "cargo-test-focused" @("cargo", "test", "-p", "vida", $TestFilter, "--", "--nocapture", "--test-threads=1")
        } else {
            Invoke-Timed "cargo-test-no-run" @("cargo", "test", "-p", "vida", "--no-run")
        }
    } elseif ($Mode -eq "runtime-smoke") {
        Invoke-Timed "cargo-build-debug" @("cargo", "build", "-p", "vida")
        Invoke-Timed "debug-vida-status" @(".\target\debug\vida.exe", "status", "--json")
    } elseif ($Mode -eq "release-install") {
        Invoke-Timed "vida-release-install" @("vida", "release", "install", "--json")
        Invoke-Timed "installed-vida-status" @("vida", "status", "--json")
    }
} finally {
    Pop-Location
    if ($Json) {
        $Records | ConvertTo-Json -Depth 6
    } else {
        foreach ($record in $Records) {
            Write-Host ("[{0}] {1} {2}ms" -f $record.exit_status, $record.operation_id, $record.duration_ms)
        }
    }
}
