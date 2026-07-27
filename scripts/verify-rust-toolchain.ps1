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

$userProfile = [Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)
$cargoBin = Join-Path $userProfile ".cargo\bin"

function Resolve-CargoTool {
    param([string]$Name)

    $canonical = Join-Path $cargoBin "$Name.exe"
    if (Test-Path -LiteralPath $canonical -PathType Leaf) {
        return $canonical
    }
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }
    Fail "Unable to resolve $Name from $canonical or PATH."
}

$rustc = Resolve-CargoTool "rustc"
$rustcLines = @(& $rustc --version)
$rustcExitCode = $LASTEXITCODE
$rustcOutput = (($rustcLines | Select-Object -First 1) -join "").Trim()
if ($rustcExitCode -ne 0) {
    Fail "rustc --version failed."
}

$rustcMatch = [regex]::Match($rustcOutput, '^rustc (?<version>\d+\.\d+\.\d+)')
if (-not $rustcMatch.Success) {
    Fail "Unable to parse rustc version: $rustcOutput"
}

$actualVersion = [version]$rustcMatch.Groups["version"].Value
$minimum = [version]$MinimumVersion
if ($actualVersion -lt $minimum) {
    Fail "Rust $($rustcMatch.Groups["version"].Value) is below required minimum $MinimumVersion."
}

$rustup = Resolve-CargoTool "rustup"
$activeToolchainLines = @(& $rustup show active-toolchain 2>$null)
$rustupExitCode = $LASTEXITCODE
$activeToolchain = (($activeToolchainLines | Select-Object -First 1) -join "").Trim()
if ($rustupExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($activeToolchain)) {
    Fail "Unable to resolve the active rustup toolchain."
}

function Read-Metadata {
    param([string]$ManifestPath)

    $cargo = Resolve-CargoTool "cargo"
    $metadataLines = @(& $cargo metadata --manifest-path $ManifestPath --no-deps --format-version 1)
    $cargoExitCode = $LASTEXITCODE
    $metadataJson = ($metadataLines -join [Environment]::NewLine)
    if ($cargoExitCode -ne 0) {
        Fail "cargo metadata failed for $ManifestPath."
    }
    try {
        return $metadataJson | ConvertFrom-Json
    } catch {
        Fail "cargo metadata returned invalid JSON for $ManifestPath."
    }
}

$packages = New-Object System.Collections.Generic.List[object]
Push-Location $rootDir
try {
    $rootMetadata = Read-Metadata -ManifestPath (Join-Path $rootDir "Cargo.toml")
    foreach ($package in @($rootMetadata.packages)) {
        [void]$packages.Add($package)
    }
    $modelManifest = Join-Path $rootDir "tests/model/Cargo.toml"
    if (Test-Path -LiteralPath $modelManifest -PathType Leaf) {
        $modelMetadata = Read-Metadata -ManifestPath $modelManifest
        foreach ($package in @($modelMetadata.packages)) {
            [void]$packages.Add($package)
        }
    }
} finally {
    Pop-Location
}

$invalidPackages = @($packages | Where-Object { $_.rust_version -ne $MinimumVersion })
if ($invalidPackages.Count -gt 0) {
    Fail "Package rust-version mismatch: $($invalidPackages.name -join ', '). Expected $MinimumVersion."
}

$result = [ordered]@{
    status = "pass"
    required_minimum = $MinimumVersion
    rustc = $rustcOutput
    active_toolchain = $activeToolchain
    package_count = $packages.Count
}
if ($Json) {
    $result | ConvertTo-Json -Compress
} else {
    Write-Output ("[rust-toolchain] pass: {0}; active={1}; packages={2}" -f $rustcOutput, $activeToolchain, $packages.Count)
}
