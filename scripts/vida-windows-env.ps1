function Test-VidaWindowsHost {
    return [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform([System.Runtime.InteropServices.OSPlatform]::Windows)
}

function Set-VidaEnvIfMissing {
    param(
        [string]$Name,
        [string]$Value
    )

    if ([string]::IsNullOrWhiteSpace((Get-Item -Path "Env:$Name" -ErrorAction SilentlyContinue).Value) -and
        -not [string]::IsNullOrWhiteSpace($Value)) {
        Set-Item -Path "Env:$Name" -Value $Value
    }
}

function Add-VidaPathEntries {
    param([string[]]$Entries)

    $separator = [System.IO.Path]::PathSeparator
    $existing = New-Object System.Collections.Generic.HashSet[string]([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($entry in (($env:Path -split [regex]::Escape([string]$separator)) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })) {
        try {
            [void]$existing.Add(([System.IO.Path]::GetFullPath($entry.Trim())))
        } catch {
            [void]$existing.Add($entry.Trim())
        }
    }

    $toPrepend = New-Object System.Collections.Generic.List[string]
    foreach ($entry in $Entries) {
        if ([string]::IsNullOrWhiteSpace($entry) -or -not (Test-Path -LiteralPath $entry)) {
            continue
        }
        $fullPath = [System.IO.Path]::GetFullPath($entry)
        if ($existing.Add($fullPath)) {
            $toPrepend.Add($fullPath)
        }
    }

    if ($toPrepend.Count -gt 0) {
        $env:Path = (($toPrepend.ToArray() + @($env:Path)) -join [string]$separator)
    }
}

function Resolve-VidaSystemDrive {
    if (-not [string]::IsNullOrWhiteSpace($env:SystemDrive)) {
        return $env:SystemDrive.TrimEnd('\')
    }
    foreach ($candidate in @($env:SystemRoot, $env:windir, $env:USERPROFILE, $HOME, "C:\Windows")) {
        if ([string]::IsNullOrWhiteSpace($candidate)) {
            continue
        }
        try {
            $root = [System.IO.Path]::GetPathRoot([System.IO.Path]::GetFullPath($candidate))
            if (-not [string]::IsNullOrWhiteSpace($root)) {
                return $root.TrimEnd('\')
            }
        } catch {
            continue
        }
    }
    return "C:"
}

function Resolve-VidaUserProfile {
    $userProfile = $env:USERPROFILE
    if (-not [string]::IsNullOrWhiteSpace($userProfile)) {
        try {
            $resolvedProfile = [System.IO.Path]::GetFullPath($userProfile)
            if ((Split-Path -Leaf $resolvedProfile) -eq "Users" -or -not (Test-Path -LiteralPath $resolvedProfile)) {
                $userProfile = ""
            } else {
                $userProfile = $resolvedProfile
            }
        } catch {
            $userProfile = ""
        }
    }
    if ([string]::IsNullOrWhiteSpace($userProfile) -and -not [string]::IsNullOrWhiteSpace($HOME)) {
        try {
            $homePath = [System.IO.Path]::GetFullPath($HOME)
            if ((Split-Path -Leaf $homePath) -ne "Users" -and (Test-Path -LiteralPath $homePath)) {
                $userProfile = $homePath
            }
        } catch {
            $userProfile = ""
        }
    }
    if ([string]::IsNullOrWhiteSpace($userProfile)) {
        $specialProfile = [System.Environment]::GetFolderPath([System.Environment+SpecialFolder]::UserProfile)
        if (-not [string]::IsNullOrWhiteSpace($specialProfile) -and (Test-Path -LiteralPath $specialProfile)) {
            $userProfile = $specialProfile
        }
    }
    if ([string]::IsNullOrWhiteSpace($userProfile) -and
        -not [string]::IsNullOrWhiteSpace($env:HOMEDRIVE) -and
        -not [string]::IsNullOrWhiteSpace($env:HOMEPATH)) {
        $userProfile = Join-Path $env:HOMEDRIVE $env:HOMEPATH
    }
    if ([string]::IsNullOrWhiteSpace($userProfile)) {
        $userName = $env:USERNAME
        if ([string]::IsNullOrWhiteSpace($userName)) {
            $userName = [System.Environment]::UserName
        }
        if (-not [string]::IsNullOrWhiteSpace($userName)) {
            $userProfile = Join-Path "C:\Users" $userName
        }
    }
    return $userProfile
}

function Assert-VidaWritableTemp {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        throw "[vida-windows-env] TEMP/TMP path is empty."
    }

    New-Item -ItemType Directory -Force -Path $Path | Out-Null
    $probePath = Join-Path $Path ("vida-temp-probe-{0}.tmp" -f ([System.Guid]::NewGuid().ToString("N")))
    try {
        Set-Content -LiteralPath $probePath -Value "ok" -Encoding ascii
        Remove-Item -LiteralPath $probePath -Force
    } catch {
        throw "[vida-windows-env] TEMP/TMP path is not writable: $Path. $($_.Exception.Message)"
    }
}

function Set-VidaBuildTemp {
    if (-not (Test-VidaWindowsHost)) {
        return
    }

    $candidateRoots = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:VIDA_MSVC_TEMP_DIR)) {
        $candidateRoots.Add($env:VIDA_MSVC_TEMP_DIR)
    }

    $localAppData = $env:LOCALAPPDATA
    if ([string]::IsNullOrWhiteSpace($localAppData)) {
        $userProfile = Resolve-VidaUserProfile
        if (-not [string]::IsNullOrWhiteSpace($userProfile)) {
            $localAppData = Join-Path $userProfile "AppData\Local"
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($localAppData)) {
        $candidateRoots.Add((Join-Path $localAppData "Temp\vida-stack\msvc"))
    }

    $repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))
    $candidateRoots.Add((Join-Path $repoRoot ".vida\build-temp\msvc"))

    foreach ($candidateRoot in $candidateRoots) {
        if ([string]::IsNullOrWhiteSpace($candidateRoot)) {
            continue
        }
        $tempRoot = [System.IO.Path]::GetFullPath($candidateRoot)
        try {
            Assert-VidaWritableTemp $tempRoot
            $env:TEMP = $tempRoot
            $env:TMP = $tempRoot
            return
        } catch {
            continue
        }
    }

    throw "[vida-windows-env] Unable to resolve a writable TEMP/TMP root for MSVC."
}

function Initialize-VidaWindowsEnvironment {
    param([switch]$NormalizeBuildTemp)

    if (-not (Test-VidaWindowsHost)) {
        return
    }

    $windowsRoot = $env:SystemRoot
    if ([string]::IsNullOrWhiteSpace($windowsRoot)) {
        $windowsRoot = $env:windir
    }
    if ([string]::IsNullOrWhiteSpace($windowsRoot)) {
        $windowsRoot = "C:\Windows"
    }
    $systemDrive = Resolve-VidaSystemDrive
    Set-VidaEnvIfMissing "SystemDrive" $systemDrive
    Set-VidaEnvIfMissing "SystemRoot" $windowsRoot
    Set-VidaEnvIfMissing "windir" $windowsRoot
    Set-VidaEnvIfMissing "ComSpec" (Join-Path $windowsRoot "System32\cmd.exe")
    Set-VidaEnvIfMissing "ProgramData" (Join-Path $systemDrive "ProgramData")
    Set-VidaEnvIfMissing "ProgramFiles" "C:\Program Files"
    Set-VidaEnvIfMissing "ProgramFiles(x86)" "C:\Program Files (x86)"

    $userProfile = Resolve-VidaUserProfile
    if (-not [string]::IsNullOrWhiteSpace($userProfile)) {
        Set-Item -Path "Env:USERPROFILE" -Value $userProfile
    }
    if (-not [string]::IsNullOrWhiteSpace($userProfile)) {
        $profileRoot = [System.IO.Path]::GetPathRoot($userProfile)
        Set-Item -Path "Env:HOMEDRIVE" -Value $profileRoot.TrimEnd('\')
        Set-Item -Path "Env:HOMEPATH" -Value $userProfile.Substring($profileRoot.Length - 1)
        Set-Item -Path "Env:LOCALAPPDATA" -Value (Join-Path $userProfile "AppData\Local")
        Set-Item -Path "Env:APPDATA" -Value (Join-Path $userProfile "AppData\Roaming")
    }

    if ($NormalizeBuildTemp) {
        Set-VidaBuildTemp
    } else {
        Set-VidaEnvIfMissing "TEMP" (Join-Path $env:LOCALAPPDATA "Temp")
        Set-VidaEnvIfMissing "TMP" $env:TEMP
        if (-not [string]::IsNullOrWhiteSpace($env:TEMP)) {
            Assert-VidaWritableTemp $env:TEMP
        }
    }

    $powerShellPackageRoots = @()
    if (-not [string]::IsNullOrWhiteSpace($env:ProgramFiles)) {
        $powerShellPackageRoots = @(Get-ChildItem -LiteralPath (Join-Path $env:ProgramFiles "WindowsApps") -Directory -Filter "Microsoft.PowerShell_*_x64__8wekyb3d8bbwe" -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending |
            ForEach-Object { $_.FullName })
    }

    $pathEntries = @(
        (Join-Path $windowsRoot "System32"),
        $windowsRoot,
        (Join-Path $windowsRoot "System32\Wbem"),
        (Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps"),
        $powerShellPackageRoots,
        (Join-Path $env:ProgramFiles "PowerShell\7"),
        (Join-Path $env:ProgramFiles "WindowsApps"),
        (Join-Path $windowsRoot "System32\WindowsPowerShell\v1.0"),
        "C:\Program Files\Git\cmd",
        "C:\Program Files\Git\bin",
        (Join-Path $userProfile ".cargo\bin"),
        (Join-Path $env:LOCALAPPDATA "vida-stack\current\bin")
    )
    Add-VidaPathEntries $pathEntries
}

function Resolve-VidaPowerShellPath {
    param(
        [switch]$Required
    )

    $candidatePaths = New-Object System.Collections.Generic.List[string]
    if (-not [string]::IsNullOrWhiteSpace($env:VIDA_PWSH)) {
        $candidatePaths.Add($env:VIDA_PWSH)
    }
    if (-not [string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        $candidatePaths.Add((Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps\pwsh.exe"))
    }

    $pwshCommand = Get-Command "pwsh" -ErrorAction SilentlyContinue
    if ($pwshCommand) {
        $candidatePaths.Add($pwshCommand.Source)
    }

    if (-not [string]::IsNullOrWhiteSpace($env:ProgramFiles)) {
        $candidatePaths.Add((Join-Path $env:ProgramFiles "PowerShell\7\pwsh.exe"))
        $packageRoot = Join-Path $env:ProgramFiles "WindowsApps"
        if (Test-Path -LiteralPath $packageRoot) {
            foreach ($packagePwsh in (Get-ChildItem -LiteralPath $packageRoot -Directory -Filter "Microsoft.PowerShell_*_x64__8wekyb3d8bbwe" -ErrorAction SilentlyContinue |
                Sort-Object Name -Descending |
                ForEach-Object { Join-Path $_.FullName "pwsh.exe" })) {
                $candidatePaths.Add($packagePwsh)
            }
        }
    }

    $seen = New-Object System.Collections.Generic.HashSet[string]([System.StringComparer]::OrdinalIgnoreCase)
    foreach ($candidate in $candidatePaths) {
        if ([string]::IsNullOrWhiteSpace($candidate)) {
            continue
        }
        try {
            $fullPath = [System.IO.Path]::GetFullPath($candidate)
        } catch {
            $fullPath = $candidate
        }
        if (-not $seen.Add($fullPath)) {
            continue
        }
        if (Test-Path -LiteralPath $fullPath -PathType Leaf) {
            return $fullPath
        }
    }

    if ($Required) {
        throw "[vida-windows-env] PowerShell Core pwsh.exe was not found. Install Microsoft.PowerShell with winget or set VIDA_PWSH to pwsh.exe."
    }
    return $null
}

function Resolve-VidaCommandPath {
    param(
        [string]$Name,
        [string[]]$Candidates = @(),
        [switch]$Required
    )

    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    foreach ($candidate in $Candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return $candidate
        }
    }

    if ($Required) {
        throw "[vida-windows-env] Required command not found: $Name"
    }
    return $Name
}

function Assert-VidaWindowsBuildTools {
    if (-not (Test-VidaWindowsHost)) {
        return
    }

    $clPath = Resolve-VidaCommandPath "cl.exe"
    $linkPath = Resolve-VidaCommandPath "link.exe"
    if ($clPath -eq "cl.exe" -or $linkPath -eq "link.exe") {
        throw "[vida-windows-env] Visual Studio C++ build tools are not on PATH after environment import."
    }
    Assert-VidaWritableTemp $env:TEMP
}

function Import-VidaMsvcEnvironment {
    if (-not (Test-VidaWindowsHost)) {
        return
    }

    Initialize-VidaWindowsEnvironment -NormalizeBuildTemp
    if ((Get-Command "cl.exe" -ErrorAction SilentlyContinue) -and
        (Get-Command "link.exe" -ErrorAction SilentlyContinue) -and
        -not [string]::IsNullOrWhiteSpace($env:VCINSTALLDIR)) {
        Assert-VidaWindowsBuildTools
        return
    }

    $vcvarsCandidates = @(
        $env:VIDA_VCVARS64,
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\Professional\VC\Auxiliary\Build\vcvars64.bat"),
        (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat")
    )
    $vcvarsPath = $vcvarsCandidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
    if (-not $vcvarsPath) {
        throw "[vida-windows-env] Visual Studio vcvars64.bat was not found. Install Visual Studio Build Tools with the C++ workload or set VIDA_VCVARS64."
    }

    $cmdPath = Resolve-VidaCommandPath "cmd.exe" @((Join-Path $env:SystemRoot "System32\cmd.exe")) -Required
    & $cmdPath /d /s /c "`"$vcvarsPath`" >nul && set" | ForEach-Object {
        if ($_ -match '^(.*?)=(.*)$') {
            Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2]
        }
    }

    Initialize-VidaWindowsEnvironment -NormalizeBuildTemp
    Assert-VidaWindowsBuildTools
}
