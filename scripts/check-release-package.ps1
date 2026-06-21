param(
    [string]$Version = "",
    [switch]$Json,
    [switch]$KeepArtifacts,
    [Alias("h")]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "vida-windows-env.ps1")
Initialize-VidaWindowsEnvironment -NormalizeBuildTemp
$PwshPath = Resolve-VidaCommandPath "pwsh" @(
    "C:\Program Files\PowerShell\7\pwsh.exe",
    "$env:ProgramFiles\PowerShell\7\pwsh.exe",
    (Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps\pwsh.exe")
) -Required

function Show-Help {
    @"
Usage:
  pwsh -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/check-release-package.ps1 [-Version vX.Y.Z] [-Json] [-KeepArtifacts]

Creates an isolated Windows release-bin fixture, runs scripts/build-release.ps1
with -SkipBuild -Windows -ReleaseBinDir, and validates the generated manifest
and zip archive shape without running release install.
"@
}

function Fail {
    param([string]$Message)
    throw "[release-package-check] ERROR: $Message"
}

function Get-ReleaseVersion {
    if (-not [string]::IsNullOrWhiteSpace($Version)) {
        if ($Version.StartsWith("v")) {
            return $Version
        }
        return "v$Version"
    }

    $cargoToml = Join-Path $RootDir "crates/vida/Cargo.toml"
    foreach ($line in Get-Content -LiteralPath $cargoToml) {
        if ($line -match '^version\s*=\s*"([^"]+)"') {
            return "v$($Matches[1])"
        }
    }
    Fail "Unable to infer version from crates/vida/Cargo.toml."
}

function New-FixtureBinary {
    param(
        [string]$Directory,
        [string]$Name,
        [string]$VersionLine
    )
    $path = Join-Path $Directory $Name
    Set-Content -LiteralPath $path -Value "fixture binary for $Name" -Encoding ascii
    Set-Content -LiteralPath "$path.version" -Value $VersionLine -Encoding ascii
}

function Assert-Contains {
    param(
        [object[]]$Values,
        [string]$Expected,
        [string]$Label
    )
    if ($Values -notcontains $Expected) {
        Fail "$Label missing $Expected"
    }
}

function Assert-ZipEntry {
    param(
        [string]$ZipPath,
        [string]$Entry
    )
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::OpenRead($ZipPath)
    try {
        $entries = @($zip.Entries | ForEach-Object { $_.FullName })
        if ($entries -notcontains $Entry) {
            Fail "Zip archive missing expected entry: $Entry"
        }
    } finally {
        $zip.Dispose()
    }
}

if ($Help) {
    Show-Help
    exit 0
}

$workRoot = Join-Path $env:TEMP ("vida-release-package-check-{0}" -f ([System.Guid]::NewGuid().ToString("N")))
$releaseBinDir = Join-Path $workRoot "release-bin"
$distDir = Join-Path $workRoot "dist"
$resolvedVersion = Get-ReleaseVersion
$expectedVersion = $resolvedVersion.TrimStart("v")
$archiveBase = "vida-stack-$resolvedVersion-windows-x86_64"

try {
    New-Item -ItemType Directory -Force -Path $releaseBinDir | Out-Null
    New-FixtureBinary -Directory $releaseBinDir -Name "vida.exe" -VersionLine "vida $expectedVersion (built fixture)"
    New-FixtureBinary -Directory $releaseBinDir -Name "taskflow.exe" -VersionLine "taskflow $expectedVersion (built fixture)"
    New-FixtureBinary -Directory $releaseBinDir -Name "docflow.exe" -VersionLine "docflow $expectedVersion (built fixture)"
    New-FixtureBinary -Directory $releaseBinDir -Name "vida-pi-agent.exe" -VersionLine "vida-pi-agent $expectedVersion (built fixture)"
    New-FixtureBinary -Directory $releaseBinDir -Name "vida-coder.exe" -VersionLine "vida-coder $expectedVersion (built fixture)"

    $buildScript = Join-Path $RootDir "scripts/build-release.ps1"
    $receiptText = & $PwshPath -NoLogo -NoProfile -ExecutionPolicy Bypass -File $buildScript -SkipBuild -Windows -ReleaseBinDir $releaseBinDir -DistDir $distDir -Version $resolvedVersion -Json
    if ($LASTEXITCODE -ne 0) {
        Fail "Native release package smoke failed."
    }
    $receipt = ($receiptText -join "`n") | ConvertFrom-Json
    if ($receipt.status -ne "pass") {
        Fail "Release package receipt status was $($receipt.status)."
    }
    if (-not $receipt.windows_release) {
        Fail "Release package receipt did not report windows_release=true."
    }
    if (-not $receipt.skip_build) {
        Fail "Release package receipt did not report skip_build=true."
    }

    $manifest = Get-Content -LiteralPath $receipt.manifest_path -Raw | ConvertFrom-Json
    if ($manifest.artifact_name -ne $archiveBase) {
        Fail "Manifest artifact_name mismatch: $($manifest.artifact_name)"
    }
    foreach ($binary in @("bin/vida.exe", "bin/taskflow.exe", "bin/docflow.exe", "bin/vida-pi-agent.exe", "bin/vida-coder.exe")) {
        Assert-Contains -Values $manifest.bundled_binaries -Expected $binary -Label "manifest bundled_binaries"
        Assert-ZipEntry -ZipPath $receipt.zip_path -Entry "$archiveBase/$binary"
    }
    foreach ($label in @("vida", "taskflow", "docflow", "vida-coder")) {
        if (-not $manifest.binary_versions.$label.matches_expected_version) {
            Fail "Manifest binary version mismatch for $label."
        }
    }
    if (-not (Test-Path -LiteralPath $receipt.checksum_path -PathType Leaf)) {
        Fail "Checksum file missing: $($receipt.checksum_path)"
    }

    $summary = [ordered]@{
        status = "pass"
        artifact_name = $archiveBase
        manifest_path = $receipt.manifest_path
        zip_path = $receipt.zip_path
        checksum_path = $receipt.checksum_path
        release_bin_dir = $releaseBinDir
        dist_dir = $distDir
    }
    if ($Json) {
        $summary | ConvertTo-Json -Depth 6
    } else {
        Write-Host "[release-package-check] pass $archiveBase"
        Write-Host "[release-package-check] Manifest: $($receipt.manifest_path)"
        Write-Host "[release-package-check] Zip: $($receipt.zip_path)"
    }
} catch {
    if ($Json) {
        [ordered]@{
            status = "blocked"
            blocker_codes = @("release_package_check_failed")
            error = $_.Exception.Message
        } | ConvertTo-Json -Depth 4
    } else {
        Write-Error $_.Exception.Message
    }
    exit 1
} finally {
    if (-not $KeepArtifacts -and (Test-Path -LiteralPath $workRoot)) {
        Remove-Item -LiteralPath $workRoot -Recurse -Force
    }
}
