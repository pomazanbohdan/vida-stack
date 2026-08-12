[CmdletBinding()]
param(
    [string]$MinimumVersion = "1.97.1",
    [switch]$Json
)

$ErrorActionPreference = "Stop"
$rootDir = Split-Path -Parent $PSScriptRoot

function Fail {
    param([string]$Message)
    throw "[rust-toolchain] ERROR: $Message"
}

$binary = $env:VIDA_RUST_TOOLCHAIN_BIN
$arguments = @(
    "--minimum-version", $MinimumVersion,
    "--format", $(if ($Json) { "json" } else { "text" }),
    "--text-style", "powershell"
)

if (-not [string]::IsNullOrWhiteSpace($binary)) {
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        Fail "VIDA_RUST_TOOLCHAIN_BIN does not point to a file: $binary"
    }
    $env:VIDA_REPO_ROOT = $rootDir
    & $binary @arguments
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
    exit 0
}

$goName = if ([string]::IsNullOrWhiteSpace($env:GO)) { "go" } else { $env:GO }
$goCommand = Get-Command $goName -ErrorAction SilentlyContinue
if ($null -eq $goCommand) {
    Fail "go is required to run the Rust toolchain verifier"
}

$moduleDir = Join-Path $rootDir "tools\verify-rust-toolchain"
Push-Location $moduleDir
try {
    $env:VIDA_REPO_ROOT = $rootDir
    & $goCommand.Source "run" "." @arguments
    if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}
