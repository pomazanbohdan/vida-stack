param(
    [string]$Version = "",
    [string]$ReleaseBinDir = "",
    [string]$ReleaseSuffix = "",
    [string]$DistDir = "",
    [switch]$SkipBuild,
    [switch]$Windows,
    [switch]$Json,
    [Alias("h")]
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$RootDir = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot "vida-windows-env.ps1")
Initialize-VidaWindowsEnvironment -NormalizeBuildTemp
$CargoPath = Resolve-VidaCommandPath "cargo" @((Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe")) -Required

function Show-Help {
    @"
Usage:
  .\scripts\vida-dev-gate.cmd -Mode release-package [-ReleaseVersion vX.Y.Z] [-SkipBuild] [-Windows] [-ReleaseBinDir <dir>] [-Json]

Options:
  -Version <tag>        Release tag such as v0.9.7. Defaults to crates/vida/Cargo.toml.
  -SkipBuild            Package existing release binaries without running Cargo.
  -Windows              Build a Windows package shape and require .exe runtime binaries.
  -ReleaseBinDir <dir>  Directory containing existing release binaries for -SkipBuild.
  -ReleaseSuffix <id>   Artifact suffix. -Windows defaults this to windows-x86_64.
  -DistDir <dir>        Output directory. Defaults to ./dist.
  -Json                 Emit a machine-readable packaging receipt.

Windows skip-build example:
  .\scripts\vida-dev-gate.cmd -Mode release-package -SkipBuild -Windows -ReleaseBinDir .\.vida\cargo-target\release -Json
"@
}

function Fail {
    param([string]$Message)
    throw "[release-package] ERROR: $Message"
}

function Assert-SafeArtifactName {
    param(
        [string]$Value,
        [string]$Label
    )
    if ([string]::IsNullOrWhiteSpace($Value)) {
        Fail "$Label must not be empty."
    }
    if ([System.IO.Path]::IsPathRooted($Value)) {
        Fail "$Label must be an artifact name, not a rooted path: $Value"
    }
    if ($Value.Contains("/") -or $Value.Contains("\")) {
        Fail "$Label must not contain path separators: $Value"
    }
    if ($Value.Contains("..")) {
        Fail "$Label must not contain '..': $Value"
    }
    if ($Value -notmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$') {
        Fail "$Label may contain only ASCII letters, digits, '.', '_', and '-': $Value"
    }
}

function Test-PathWithinRoot {
    param(
        [string]$Root,
        [string]$Path
    )
    $resolvedRoot = [System.IO.Path]::GetFullPath($Root)
    $resolvedPath = [System.IO.Path]::GetFullPath($Path)
    $rootWithSeparator = $resolvedRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    return $resolvedPath.StartsWith($rootWithSeparator, [System.StringComparison]::OrdinalIgnoreCase)
}

function Test-Truthy {
    param([string]$Value)
    return $Value -match '^(1|true|TRUE|yes|YES)$'
}

function Test-HostWindows {
    return [System.IO.Path]::DirectorySeparatorChar -eq "\"
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

function Copy-RequiredFile {
    param(
        [string]$Source,
        [string]$Destination
    )
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        Fail "Missing required file: $Source"
    }
    $parent = Split-Path -Parent $Destination
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

function Copy-RequiredTree {
    param(
        [string]$Source,
        [string]$Destination
    )
    if (-not (Test-Path -LiteralPath $Source -PathType Container)) {
        Fail "Missing required directory: $Source"
    }
    if (Test-Path -LiteralPath $Destination) {
        Remove-Item -LiteralPath $Destination -Recurse -Force
    }
    $parent = Split-Path -Parent $Destination
    if (-not [string]::IsNullOrWhiteSpace($parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    Copy-Item -LiteralPath $Source -Destination $Destination -Recurse -Force
}

function Copy-SidecarWithoutMetadata {
    param(
        [string]$Source,
        [string]$Destination
    )
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        Fail "Missing required file: $Source"
    }
    $lines = New-Object System.Collections.Generic.List[string]
    foreach ($line in Get-Content -LiteralPath $Source) {
        if ($line -eq "-----") {
            break
        }
        [void]$lines.Add($line)
    }
    Set-Content -LiteralPath $Destination -Value $lines -Encoding utf8
}

function Get-BinaryFileName {
    param([string]$Name)
    if ($script:WindowsRelease) {
        return "$Name.exe"
    }
    return $Name
}

function Copy-RuntimeBinary {
    param(
        [string]$Name,
        [string]$Destination
    )
    $source = Join-Path $script:ResolvedReleaseBinDir (Get-BinaryFileName $Name)
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        Fail "Missing built runtime binary for release target $script:ResolvedReleaseSuffix`: $source"
    }
    Copy-RequiredFile -Source $source -Destination $Destination
    $versionStamp = "$source.version"
    if (Test-Path -LiteralPath $versionStamp -PathType Leaf) {
        Copy-RequiredFile -Source $versionStamp -Destination "$Destination.version"
    }
}

function Read-VersionStamp {
    param([string]$BinaryPath)
    $versionStamp = "$BinaryPath.version"
    if (Test-Path -LiteralPath $versionStamp -PathType Leaf) {
        $line = Get-Content -LiteralPath $versionStamp -TotalCount 1
        if (-not [string]::IsNullOrWhiteSpace($line)) {
            return $line.Trim()
        }
    }
    return ""
}

function Invoke-BinaryFirstLine {
    param(
        [string]$BinaryPath,
        [string[]]$Arguments
    )
    try {
        $output = & $BinaryPath @Arguments 2>$null | Select-Object -First 1
        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($output)) {
            return ([string]$output).Trim()
        }
    } catch {
        return ""
    }
    return ""
}

function Get-RuntimeBinaryVersionLine {
    param(
        [string]$Label,
        [string]$BinaryPath
    )
    $line = Invoke-BinaryFirstLine -BinaryPath $BinaryPath -Arguments @("--version")
    if ([string]::IsNullOrWhiteSpace($line)) {
        $line = Read-VersionStamp -BinaryPath $BinaryPath
    }
    if ([string]::IsNullOrWhiteSpace($line)) {
        Fail "Packaged $Label version could not be read from $BinaryPath or $BinaryPath.version."
    }
    return $line
}

function Assert-RuntimeBinaryVersion {
    param(
        [string]$Label,
        [string]$BinaryPath,
        [string]$ExpectedVersion
    )
    $line = Get-RuntimeBinaryVersionLine -Label $Label -BinaryPath $BinaryPath
    $pattern = "^{0}\s+{1}(\s+\(built .+\))?$" -f [regex]::Escape($Label), [regex]::Escape($ExpectedVersion)
    if ($line -notmatch $pattern) {
        Fail "Packaged $Label version mismatch: expected '$Label $ExpectedVersion' with optional build timestamp, got '$line' from $BinaryPath."
    }
    return $line
}

function Assert-CommandOrVersionStamp {
    param(
        [string]$Label,
        [string]$BinaryPath,
        [string[]]$Arguments
    )
    $line = Invoke-BinaryFirstLine -BinaryPath $BinaryPath -Arguments $Arguments
    if (-not [string]::IsNullOrWhiteSpace($line)) {
        return
    }
    if (-not [string]::IsNullOrWhiteSpace((Read-VersionStamp -BinaryPath $BinaryPath))) {
        return
    }
    Fail "Packaged $Label command check failed and no version stamp was available: $BinaryPath $($Arguments -join ' ')"
}

function New-BinaryVersionRecord {
    param(
        [string]$Label,
        [string]$RelativePath,
        [string]$VersionLine,
        [string]$ExpectedVersion
    )
    $pattern = "^{0}\s+(\S+)(?:\s+\(built ([^)]+)\))?$" -f [regex]::Escape($Label)
    $match = [regex]::Match($VersionLine, $pattern)
    $actualVersion = if ($match.Success) { $match.Groups[1].Value } else { "" }
    $timestamp = if ($match.Success -and $match.Groups[2].Success) { $match.Groups[2].Value } else { $null }
    return [ordered]@{
        path = $RelativePath
        version_line = $VersionLine
        expected_version = $ExpectedVersion
        matches_expected_version = ($actualVersion -eq $ExpectedVersion)
        build_timestamp = $timestamp
    }
}

function New-ZipFromPackageRoot {
    param(
        [string]$PackageRoot,
        [string]$SourceDir,
        [string]$ZipPath
    )
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path -LiteralPath $ZipPath) {
        Remove-Item -LiteralPath $ZipPath -Force
    }
    $zip = [System.IO.Compression.ZipFile]::Open($ZipPath, [System.IO.Compression.ZipArchiveMode]::Create)
    try {
        $files = Get-ChildItem -LiteralPath $SourceDir -Recurse -File | Sort-Object FullName
        foreach ($file in $files) {
            $relative = [System.IO.Path]::GetRelativePath($PackageRoot, $file.FullName).Replace("\", "/")
            if ($relative -eq ".." -or $relative.StartsWith("../") -or [System.IO.Path]::IsPathRooted($relative)) {
                Fail "Refusing to create zip entry outside package root: $relative"
            }
            [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $file.FullName, $relative) | Out-Null
        }
    } finally {
        $zip.Dispose()
    }
}

function New-TarGzFromPackageRoot {
    param(
        [string]$PackageRoot,
        [string]$ArchiveBase,
        [string]$TarPath
    )
    $tar = Get-Command "tar" -ErrorAction SilentlyContinue
    if (-not $tar) {
        return $null
    }
    if (Test-Path -LiteralPath $TarPath) {
        Remove-Item -LiteralPath $TarPath -Force
    }
    Push-Location $PackageRoot
    try {
        & $tar.Source "-czf" $TarPath $ArchiveBase
        if ($LASTEXITCODE -ne 0) {
            Fail "tar failed while creating $TarPath."
        }
    } finally {
        Pop-Location
    }
    return $TarPath
}

function Get-Sha256Line {
    param([string]$Path)
    $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    return "$hash  $([System.IO.Path]::GetFileName($Path))"
}

function Test-ReleaseManifest {
    param(
        [string]$ManifestPath,
        [string]$ArchiveBase,
        [string[]]$ExpectedBinaries
    )
    if (-not (Test-Path -LiteralPath $ManifestPath -PathType Leaf)) {
        Fail "Missing release manifest: $ManifestPath"
    }
    $manifest = Get-Content -LiteralPath $ManifestPath -Raw | ConvertFrom-Json
    if ($manifest.artifact_name -ne $ArchiveBase) {
        Fail "Manifest artifact_name mismatch: $($manifest.artifact_name)"
    }
    foreach ($binary in $ExpectedBinaries) {
        if ($manifest.bundled_binaries -notcontains $binary) {
            Fail "Manifest missing bundled binary: $binary"
        }
    }
    foreach ($label in @("vida", "taskflow", "docflow", "vida-coder")) {
        if (-not $manifest.binary_versions.$label.matches_expected_version) {
            Fail "Manifest binary version mismatch for $label."
        }
    }
    return $manifest
}

function Test-ZipContainsBinaries {
    param(
        [string]$ZipPath,
        [string[]]$ExpectedEntries
    )
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [System.IO.Compression.ZipFile]::OpenRead($ZipPath)
    try {
        $entries = @($zip.Entries | ForEach-Object { $_.FullName })
        foreach ($entry in $ExpectedEntries) {
            if ($entries -notcontains $entry) {
                Fail "Zip archive missing expected entry: $entry"
            }
        }
    } finally {
        $zip.Dispose()
    }
}

if ($Help) {
    Show-Help
    exit 0
}

try {
    if (-not $SkipBuild -and (Test-Truthy $env:VIDA_RELEASE_SKIP_BUILD)) {
        $SkipBuild = $true
    }
    if ([string]::IsNullOrWhiteSpace($ReleaseBinDir) -and -not [string]::IsNullOrWhiteSpace($env:VIDA_RELEASE_BIN_DIR)) {
        $ReleaseBinDir = $env:VIDA_RELEASE_BIN_DIR
    }
    if ([string]::IsNullOrWhiteSpace($ReleaseSuffix) -and -not [string]::IsNullOrWhiteSpace($env:VIDA_RELEASE_SUFFIX)) {
        $ReleaseSuffix = $env:VIDA_RELEASE_SUFFIX
    }
    if ($Windows -and [string]::IsNullOrWhiteSpace($ReleaseSuffix)) {
        $ReleaseSuffix = "windows-x86_64"
    }
    if (-not $Windows -and $ReleaseSuffix -eq "windows-x86_64") {
        $Windows = $true
    }
    if (-not $Windows -and (Test-HostWindows) -and [string]::IsNullOrWhiteSpace($ReleaseSuffix)) {
        $Windows = $true
        $ReleaseSuffix = "windows-x86_64"
    }

    $script:WindowsRelease = [bool]$Windows
    $script:ResolvedReleaseSuffix = $ReleaseSuffix
    $resolvedVersion = Get-ReleaseVersion
    Assert-SafeArtifactName -Value $resolvedVersion -Label "Release version"
    if (-not [string]::IsNullOrWhiteSpace($script:ResolvedReleaseSuffix)) {
        Assert-SafeArtifactName -Value $script:ResolvedReleaseSuffix -Label "Release suffix"
    }
    $expectedVersion = $resolvedVersion.TrimStart("v")
    $archiveBase = "vida-stack-$resolvedVersion"
    if (-not [string]::IsNullOrWhiteSpace($script:ResolvedReleaseSuffix)) {
        $archiveBase = "$archiveBase-$script:ResolvedReleaseSuffix"
    }
    Assert-SafeArtifactName -Value $archiveBase -Label "Archive base"
    if ([string]::IsNullOrWhiteSpace($DistDir)) {
        $DistDir = Join-Path $RootDir "dist"
    }
    $DistDir = [System.IO.Path]::GetFullPath($DistDir)
    $packageRoot = [System.IO.Path]::GetFullPath((Join-Path $DistDir "package"))
    $stageDir = [System.IO.Path]::GetFullPath((Join-Path $packageRoot $archiveBase))
    if (-not (Test-PathWithinRoot -Root $packageRoot -Path $stageDir)) {
        Fail "Resolved staging directory escapes package root: $stageDir"
    }
    $cargoTargetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
        Join-Path $RootDir ".vida\cargo-target"
    } else {
        $env:CARGO_TARGET_DIR
    }
    if ([string]::IsNullOrWhiteSpace($ReleaseBinDir)) {
        $ReleaseBinDir = Join-Path $cargoTargetRoot "release"
    }
    $script:ResolvedReleaseBinDir = [System.IO.Path]::GetFullPath($ReleaseBinDir)

    if (-not $SkipBuild) {
        if (Test-HostWindows) {
            Import-VidaMsvcEnvironment
        }
        Push-Location $RootDir
        try {
            & $CargoPath build --release -p vida -p taskflow-cli -p docflow-cli -p vida-pi-agent -p vida-coder
            if ($LASTEXITCODE -ne 0) {
                Fail "Cargo release build failed."
            }
        } finally {
            Pop-Location
        }
    }

    if (Test-Path -LiteralPath $DistDir) {
        Remove-Item -LiteralPath $DistDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "bin") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $stageDir "install/assets") | Out-Null

    Copy-RequiredFile -Source (Join-Path $RootDir "AGENTS.md") -Destination (Join-Path $stageDir "AGENTS.md")
    Copy-SidecarWithoutMetadata -Source (Join-Path $RootDir "AGENTS.sidecar.md") -Destination (Join-Path $stageDir "AGENTS.sidecar.md")
    Copy-RequiredTree -Source (Join-Path $RootDir ".codex") -Destination (Join-Path $stageDir ".codex")
    foreach ($hostTemplate in @(".qwen", ".kilo", ".opencode")) {
        $source = Join-Path $RootDir $hostTemplate
        if (Test-Path -LiteralPath $source -PathType Container) {
            Copy-RequiredTree -Source $source -Destination (Join-Path $stageDir $hostTemplate)
        }
    }
    Copy-RequiredTree -Source (Join-Path $RootDir "vida") -Destination (Join-Path $stageDir "vida")
    Get-ChildItem -LiteralPath $stageDir -Recurse -Directory -Filter "__pycache__" | Remove-Item -Recurse -Force
    Get-ChildItem -LiteralPath $stageDir -Recurse -File -Filter "*.pyc" | Remove-Item -Force

    $binDir = Join-Path $stageDir "bin"
    $vidaBin = Join-Path $binDir (Get-BinaryFileName "vida")
    $taskflowBin = Join-Path $binDir (Get-BinaryFileName "taskflow")
    $docflowBin = Join-Path $binDir (Get-BinaryFileName "docflow")
    $piAgentBin = Join-Path $binDir (Get-BinaryFileName "vida-pi-agent")
    $vidaCoderBin = Join-Path $binDir (Get-BinaryFileName "vida-coder")
    Copy-RuntimeBinary -Name "vida" -Destination $vidaBin
    Copy-RuntimeBinary -Name "taskflow" -Destination $taskflowBin
    Copy-RuntimeBinary -Name "docflow" -Destination $docflowBin
    Copy-RuntimeBinary -Name "vida-pi-agent" -Destination $piAgentBin
    Copy-RuntimeBinary -Name "vida-coder" -Destination $vidaCoderBin

    $vidaVersionLine = Assert-RuntimeBinaryVersion -Label "vida" -BinaryPath $vidaBin -ExpectedVersion $expectedVersion
    $taskflowVersionLine = Assert-RuntimeBinaryVersion -Label "taskflow" -BinaryPath $taskflowBin -ExpectedVersion $expectedVersion
    $docflowVersionLine = Assert-RuntimeBinaryVersion -Label "docflow" -BinaryPath $docflowBin -ExpectedVersion $expectedVersion
    $vidaCoderVersionLine = Assert-RuntimeBinaryVersion -Label "vida-coder" -BinaryPath $vidaCoderBin -ExpectedVersion $expectedVersion
    Assert-CommandOrVersionStamp -Label "vida-pi-agent" -BinaryPath $piAgentBin -Arguments @("--help")
    Assert-CommandOrVersionStamp -Label "vida-coder" -BinaryPath $vidaCoderBin -Arguments @("provider-check", "--json")

    Remove-Item -LiteralPath @(
        "$vidaBin.version",
        "$taskflowBin.version",
        "$docflowBin.version",
        "$piAgentBin.version",
        "$vidaCoderBin.version"
    ) -Force -ErrorAction SilentlyContinue

    Copy-RequiredFile -Source (Join-Path $RootDir "docs/framework/templates/vida.config.yaml.template") -Destination (Join-Path $stageDir "install/assets/vida.config.yaml.template")
    Copy-RequiredFile -Source (Join-Path $RootDir "docs/product/spec/templates/feature-design-document.template.md") -Destination (Join-Path $stageDir "install/assets/feature-design-document.template.md")

    $binaryRoots = if ($script:WindowsRelease) {
        @("bin/vida.exe", "bin/taskflow.exe", "bin/docflow.exe", "bin/vida-pi-agent.exe", "bin/vida-coder.exe")
    } else {
        @("bin/vida", "bin/taskflow", "bin/docflow", "bin/vida-pi-agent", "bin/vida-coder")
    }
    $manifestPath = Join-Path $DistDir "$archiveBase.manifest.json"
    $manifest = [ordered]@{
        artifact_name = $archiveBase
        version = $resolvedVersion
        built_at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssK")
        package_root = $archiveBase
        included_roots = @(
            "AGENTS.md",
            "AGENTS.sidecar.md",
            ".codex/",
            ".qwen/",
            ".kilo/",
            ".opencode/"
        ) + $binaryRoots + @(
            "install/assets/",
            "vida/"
        )
        installed_entrypoints = @("vida", "taskflow", "docflow", "vida-pi-agent", "vida-coder", "vida docflow", "vida taskflow")
        bundled_binaries = $binaryRoots
        binary_versions = [ordered]@{
            vida = New-BinaryVersionRecord -Label "vida" -RelativePath $binaryRoots[0] -VersionLine $vidaVersionLine -ExpectedVersion $expectedVersion
            taskflow = New-BinaryVersionRecord -Label "taskflow" -RelativePath $binaryRoots[1] -VersionLine $taskflowVersionLine -ExpectedVersion $expectedVersion
            docflow = New-BinaryVersionRecord -Label "docflow" -RelativePath $binaryRoots[2] -VersionLine $docflowVersionLine -ExpectedVersion $expectedVersion
            "vida-coder" = New-BinaryVersionRecord -Label "vida-coder" -RelativePath $binaryRoots[4] -VersionLine $vidaCoderVersionLine -ExpectedVersion $expectedVersion
        }
        installer_managed_runtimes = @("vida", "taskflow", "docflow", "vida-pi-agent", "vida-coder")
        launcher_contracts = [ordered]@{
            taskflow = "vida taskflow"
            docflow = "vida docflow"
        }
        installed_compatibility_contracts = [ordered]@{
            taskflow = "canonical taskflow runtime"
            docflow = "canonical docflow runtime"
            "vida docflow" = "canonical docflow runtime"
            "vida taskflow" = "canonical taskflow runtime"
        }
    }
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding utf8

    $zipPath = Join-Path $DistDir "$archiveBase.zip"
    New-ZipFromPackageRoot -PackageRoot $packageRoot -SourceDir $stageDir -ZipPath $zipPath
    $tarPath = New-TarGzFromPackageRoot -PackageRoot $packageRoot -ArchiveBase $archiveBase -TarPath (Join-Path $DistDir "$archiveBase.tar.gz")

    $installerAsset = Join-Path $DistDir "vida-install.sh"
    $windowsInstallerAsset = Join-Path $DistDir "vida-install.ps1"
    Copy-RequiredFile -Source (Join-Path $RootDir "install/install.sh") -Destination $installerAsset
    Copy-RequiredFile -Source (Join-Path $RootDir "install/install.ps1") -Destination $windowsInstallerAsset

    $releaseNotesSource = Join-Path $RootDir "install/release-notes-$resolvedVersion.md"
    $releaseNotesOut = Join-Path $DistDir "release-notes.md"
    if (Test-Path -LiteralPath $releaseNotesSource -PathType Leaf) {
        Copy-RequiredFile -Source $releaseNotesSource -Destination $releaseNotesOut
    } else {
        $readme = Get-Content -LiteralPath (Join-Path $RootDir "README.md")
        $capture = $false
        $notes = New-Object System.Collections.Generic.List[string]
        foreach ($line in $readme) {
            if ($line -match '^## ') {
                if ($capture) {
                    break
                }
                $capture = $true
            }
            if ($capture) {
                [void]$notes.Add($line)
            }
        }
        Set-Content -LiteralPath $releaseNotesOut -Value $notes -Encoding utf8
    }

    $checksumFiles = @($zipPath)
    if ($tarPath) {
        $checksumFiles += $tarPath
    }
    $checksumFiles += @($installerAsset, $windowsInstallerAsset)
    $checksumPath = Join-Path $DistDir "$archiveBase.sha256"
    $checksumFiles | ForEach-Object { Get-Sha256Line -Path $_ } | Set-Content -LiteralPath $checksumPath -Encoding utf8

    [void](Test-ReleaseManifest -ManifestPath $manifestPath -ArchiveBase $archiveBase -ExpectedBinaries $binaryRoots)
    Test-ZipContainsBinaries -ZipPath $zipPath -ExpectedEntries ($binaryRoots | ForEach-Object { "$archiveBase/$_" })

    $receipt = [ordered]@{
        status = "pass"
        artifact_name = $archiveBase
        version = $resolvedVersion
        windows_release = [bool]$script:WindowsRelease
        skip_build = [bool]$SkipBuild
        release_bin_dir = $script:ResolvedReleaseBinDir
        dist_dir = $DistDir
        package_root = $stageDir
        manifest_path = $manifestPath
        zip_path = $zipPath
        tar_gz_path = $tarPath
        checksum_path = $checksumPath
        bundled_binaries = $binaryRoots
    }
    if ($Json) {
        $receipt | ConvertTo-Json -Depth 8
    } else {
        Write-Host "[release-package] Built $archiveBase"
        Write-Host "[release-package] Manifest: $manifestPath"
        Write-Host "[release-package] Zip: $zipPath"
        if ($tarPath) {
            Write-Host "[release-package] Tar: $tarPath"
        }
        Write-Host "[release-package] Checksums: $checksumPath"
    }
} catch {
    if ($Json) {
        [ordered]@{
            status = "blocked"
            blocker_codes = @("release_package_failed")
            error = $_.Exception.Message
        } | ConvertTo-Json -Depth 4
    } else {
        Write-Error $_.Exception.Message
    }
    exit 1
}
