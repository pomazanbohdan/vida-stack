function Enter-VidaBuildConcurrencyGuard {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootDir,
        [string]$Scope = "build",
        [int]$TimeoutSeconds = -1
    )

    $resolvedRoot = [System.IO.Path]::GetFullPath($RootDir)
    $lockDir = Join-Path $resolvedRoot ".vida\data\state\script-locks"
    $lockPath = Join-Path $lockDir "$Scope.lock"
    $lockMetadataPath = "$lockPath.meta.json"

    $currentToken = $env:VIDA_BUILD_SCRIPT_LOCK_TOKEN
    $inheritedToken = $env:VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN
    $hasMatchingInheritedToken = -not [string]::IsNullOrWhiteSpace($currentToken) -and
        -not [string]::IsNullOrWhiteSpace($inheritedToken) -and
        $currentToken -eq $inheritedToken

    if ($env:VIDA_BUILD_SCRIPT_LOCK_HELD -eq "1" -and
        $env:VIDA_BUILD_SCRIPT_LOCK_PATH -eq $lockPath -and
        ($env:VIDA_BUILD_SCRIPT_LOCK_PID -eq [string]$PID -or $hasMatchingInheritedToken)) {
        return [pscustomobject]@{
            Reentrant = $true
            LockPath = $lockPath
        }
    }

    if ($TimeoutSeconds -lt 0) {
        if ([string]::IsNullOrWhiteSpace($env:VIDA_BUILD_LOCK_TIMEOUT_SECONDS)) {
            $TimeoutSeconds = 0
        } else {
            $TimeoutSeconds = [int]$env:VIDA_BUILD_LOCK_TIMEOUT_SECONDS
        }
    }

    Assert-VidaBuildGuardLocalPath -LiteralPath $lockPath -Description "lock"
    Assert-VidaBuildGuardLocalPath -LiteralPath $lockMetadataPath -Description "metadata"
    New-Item -ItemType Directory -Force -Path $lockDir | Out-Null
    Assert-VidaBuildGuardLocalPath -LiteralPath $lockPath -Description "lock"
    Assert-VidaBuildGuardLocalPath -LiteralPath $lockMetadataPath -Description "metadata"
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)

    while ($true) {
        try {
            Assert-VidaBuildGuardLocalPath -LiteralPath $lockPath -Description "lock"
            $stream = [System.IO.File]::Open(
                $lockPath,
                [System.IO.FileMode]::OpenOrCreate,
                [System.IO.FileAccess]::ReadWrite,
                [System.IO.FileShare]::None
            )
            $stream.SetLength(0)
            $body = @(
                "pid=$PID",
                "scope=$Scope",
                "root=$resolvedRoot",
                "started_at=$((Get-Date).ToString("o"))",
                "command=$([System.Environment]::CommandLine)"
            ) -join [Environment]::NewLine
            $bytes = [System.Text.Encoding]::UTF8.GetBytes($body)
            $stream.Write($bytes, 0, $bytes.Length)
            $stream.Flush()
            $metadataJson = [ordered]@{
                pid = $PID
                scope = $Scope
                root = $resolvedRoot
                started_at = (Get-Date).ToString("o")
                command = [System.Environment]::CommandLine
            } | ConvertTo-Json -Depth 4
            Set-VidaBuildGuardMetadata -LockMetadataPath $lockMetadataPath -Json $metadataJson

            $previousHeld = $env:VIDA_BUILD_SCRIPT_LOCK_HELD
            $previousPath = $env:VIDA_BUILD_SCRIPT_LOCK_PATH
            $previousScope = $env:VIDA_BUILD_SCRIPT_LOCK_SCOPE
            $previousPid = $env:VIDA_BUILD_SCRIPT_LOCK_PID
            $previousToken = $env:VIDA_BUILD_SCRIPT_LOCK_TOKEN
            $previousInheritedToken = $env:VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN
            $lockToken = [guid]::NewGuid().ToString("N")

            $env:VIDA_BUILD_SCRIPT_LOCK_HELD = "1"
            $env:VIDA_BUILD_SCRIPT_LOCK_PATH = $lockPath
            $env:VIDA_BUILD_SCRIPT_LOCK_SCOPE = $Scope
            $env:VIDA_BUILD_SCRIPT_LOCK_PID = [string]$PID
            $env:VIDA_BUILD_SCRIPT_LOCK_TOKEN = $lockToken
            $env:VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN = $lockToken

            return [pscustomobject]@{
                Reentrant = $false
                Stream = $stream
                LockPath = $lockPath
                LockMetadataPath = $lockMetadataPath
                PreviousHeld = $previousHeld
                PreviousPath = $previousPath
                PreviousScope = $previousScope
                PreviousPid = $previousPid
                PreviousToken = $previousToken
                PreviousInheritedToken = $previousInheritedToken
            }
        } catch [System.IO.IOException] {
            $owner = Get-VidaBuildGuardLockOwner -LockMetadataPath $lockMetadataPath
            if ($owner -and $owner.lock_owner_status -eq "dead") {
                Remove-VidaBuildGuardLocalFile -LiteralPath $lockPath
                Remove-VidaBuildGuardLocalFile -LiteralPath $lockMetadataPath
                continue
            }
            if ($TimeoutSeconds -le 0 -or (Get-Date) -ge $deadline) {
                $script:VidaBuildGuardLastBlockedOwner = $owner
                $ownerText = Format-VidaBuildGuardLockOwner -Owner $owner
                throw "Build/test/install guard blocked another run. Lock: $lockPath. $ownerText recovery_action=wait_for_owner_or_stop_owner_process; set VIDA_BUILD_LOCK_TIMEOUT_SECONDS to wait instead of failing fast."
            }
            Start-Sleep -Seconds 1
        }
    }
}


function Test-VidaBuildGuardReparsePoint {
    param([string]$LiteralPath)

    $item = Get-Item -LiteralPath $LiteralPath -Force -ErrorAction SilentlyContinue
    return $item -and (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0)
}

function Assert-VidaBuildGuardLocalPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$LiteralPath,
        [string]$Description = "path"
    )

    $parentPath = Split-Path -Parent $LiteralPath
    $segments = New-Object System.Collections.Generic.List[string]
    $current = [System.IO.Path]::GetFullPath($parentPath)
    while (-not [string]::IsNullOrWhiteSpace($current)) {
        [void]$segments.Add($current)
        $next = Split-Path -Parent $current
        if ([string]::IsNullOrWhiteSpace($next) -or $next -eq $current) {
            break
        }
        $current = $next
    }

    for ($index = $segments.Count - 1; $index -ge 0; $index--) {
        $segment = $segments[$index]
        if (Test-Path -LiteralPath $segment) {
            if (Test-VidaBuildGuardReparsePoint -LiteralPath $segment) {
                throw "Refusing to use build guard $Description because directory is a symlink/reparse point: $segment"
            }
        }
    }

    if (Test-Path -LiteralPath $LiteralPath) {
        if (Test-VidaBuildGuardReparsePoint -LiteralPath $LiteralPath) {
            throw "Refusing to use build guard $Description because file is a symlink/reparse point: $LiteralPath"
        }
    }
}

function Set-VidaBuildGuardMetadata {
    param(
        [Parameter(Mandatory = $true)]
        [string]$LockMetadataPath,
        [Parameter(Mandatory = $true)]
        [string]$Json
    )

    Assert-VidaBuildGuardLocalPath -LiteralPath $LockMetadataPath -Description "metadata"
    $encoding = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($LockMetadataPath, $Json, $encoding)
}

function Remove-VidaBuildGuardLocalFile {
    param([string]$LiteralPath)

    if ([string]::IsNullOrWhiteSpace($LiteralPath) -or -not (Test-Path -LiteralPath $LiteralPath)) {
        return
    }
    if (Test-VidaBuildGuardReparsePoint -LiteralPath $LiteralPath) {
        Write-Warning "Refusing to remove build guard symlink/reparse point: $LiteralPath"
        return
    }
    Remove-Item -LiteralPath $LiteralPath -Force -ErrorAction SilentlyContinue
}

function Get-VidaBuildGuardLockOwner {
    param([string]$LockMetadataPath)

    if (-not (Test-Path -LiteralPath $LockMetadataPath -PathType Leaf)) {
        return [pscustomobject]@{
            lock_owner_pid = $null
            lock_owner_status = "unknown"
            lock_owner_command = ""
            recovery_action = "wait_for_owner_or_stop_owner_process"
        }
    }

    try {
        $metadata = Get-Content -LiteralPath $LockMetadataPath -Raw | ConvertFrom-Json
        $ownerPid = if ($null -ne $metadata.pid) { [int]$metadata.pid } else { $null }
        $alive = $false
        if ($null -ne $ownerPid) {
            $alive = $null -ne (Get-Process -Id $ownerPid -ErrorAction SilentlyContinue)
        }
        return [pscustomobject]@{
            lock_owner_pid = $ownerPid
            lock_owner_status = $(if ($alive) { "alive" } else { "dead" })
            lock_owner_command = [string]$metadata.command
            recovery_action = $(if ($alive) { "wait_for_owner_or_stop_owner_process" } else { "remove_stale_lock_and_retry" })
        }
    } catch {
        return [pscustomobject]@{
            lock_owner_pid = $null
            lock_owner_status = "unknown"
            lock_owner_command = ""
            recovery_action = "wait_for_owner_or_stop_owner_process"
        }
    }
}

function Format-VidaBuildGuardLockOwner {
    param([object]$Owner)

    if ($null -eq $Owner) {
        return "lock_owner_pid=unknown lock_owner_status=unknown lock_owner_command=unknown"
    }
    $command = ([string]$Owner.lock_owner_command).Replace("`r", " ").Replace("`n", " ")
    if ([string]::IsNullOrWhiteSpace($command)) {
        $command = "unknown"
    }
    return "lock_owner_pid=$($Owner.lock_owner_pid) lock_owner_status=$($Owner.lock_owner_status) lock_owner_command=$command"
}

function Test-VidaBuildGuardWindowsHost {
    return [System.IO.Path]::DirectorySeparatorChar -eq "\"
}

function Get-VidaBuildGuardPathComparison {
    if (Test-VidaBuildGuardWindowsHost) {
        return [System.StringComparison]::OrdinalIgnoreCase
    }
    return [System.StringComparison]::Ordinal
}

function Test-VidaBuildGuardPathInsideRoot {
    param(
        [string]$Root,
        [string]$Path,
        [System.StringComparison]$Comparison
    )

    return $Path.Equals($Root, $Comparison) -or
        $Path.StartsWith($Root + [System.IO.Path]::DirectorySeparatorChar, $Comparison)
}

function Test-VidaBuildTargetRootSafeForCleanup {
    param(
        [string]$RootDir,
        [string]$TargetRoot
    )

    $comparison = Get-VidaBuildGuardPathComparison
    $normalizedRoot = [System.IO.Path]::GetFullPath($RootDir).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $normalizedTargetRoot = [System.IO.Path]::GetFullPath($TargetRoot).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $targetPathRoot = [System.IO.Path]::GetPathRoot($normalizedTargetRoot).TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )

    return -not $normalizedTargetRoot.Equals($targetPathRoot, $comparison) -and
        (Test-VidaBuildGuardPathInsideRoot -Root $normalizedRoot -Path $normalizedTargetRoot -Comparison $comparison) -and
        -not $normalizedTargetRoot.Equals($normalizedRoot, $comparison)
}

function Invoke-VidaBuildTargetProcessCleanup {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootDir,
        [Parameter(Mandatory = $true)]
        [string]$TargetRoot,
        [int]$ExcludeProcessId = $PID
    )

    $stopped = New-Object System.Collections.Generic.List[string]
    $failed = New-Object System.Collections.Generic.List[string]
    $skipped = $false
    $skipReason = $null

    if (-not (Test-VidaBuildGuardWindowsHost)) {
        $skipped = $true
        $skipReason = "non_windows_host"
    } elseif (-not (Test-Path -LiteralPath $TargetRoot)) {
        $skipped = $true
        $skipReason = "target_root_missing"
    } elseif (-not (Test-VidaBuildTargetRootSafeForCleanup -RootDir $RootDir -TargetRoot $TargetRoot)) {
        $skipped = $true
        $skipReason = "unsafe_target_root"
    } else {
        $targetRootFull = [System.IO.Path]::GetFullPath($TargetRoot).TrimEnd(
            [System.IO.Path]::DirectorySeparatorChar,
            [System.IO.Path]::AltDirectorySeparatorChar
        )
        $comparison = Get-VidaBuildGuardPathComparison
        $processes = Get-CimInstance Win32_Process | Where-Object {
            -not [string]::IsNullOrWhiteSpace($_.ExecutablePath) -and
            $_.ProcessId -ne $ExcludeProcessId
        }

        foreach ($process in $processes) {
            $exePath = $null
            try {
                $exePath = [System.IO.Path]::GetFullPath($process.ExecutablePath)
            } catch {
                continue
            }
            if (-not (Test-VidaBuildGuardPathInsideRoot -Root $targetRootFull -Path $exePath -Comparison $comparison)) {
                continue
            }

            $label = "{0}({1})" -f $process.Name, $process.ProcessId
            try {
                Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
                $deadline = (Get-Date).AddSeconds(5)
                while ((Get-Date) -lt $deadline) {
                    if (-not (Get-Process -Id $process.ProcessId -ErrorAction SilentlyContinue)) {
                        break
                    }
                    Start-Sleep -Milliseconds 100
                }
                if (Get-Process -Id $process.ProcessId -ErrorAction SilentlyContinue) {
                    [void]$failed.Add($label)
                } else {
                    [void]$stopped.Add($label)
                }
            } catch {
                [void]$failed.Add($label)
            }
        }
    }

    return [pscustomobject]@{
        Skipped = $skipped
        SkipReason = $skipReason
        TargetRoot = $TargetRoot
        StoppedProcesses = $stopped.ToArray()
        FailedProcesses = $failed.ToArray()
    }
}

function Exit-VidaBuildConcurrencyGuard {
    param([object]$Guard)

    if ($null -eq $Guard -or $Guard.Reentrant) {
        return
    }

    if ($Guard.Stream) {
        $Guard.Stream.Dispose()
    }
    Remove-VidaBuildGuardLocalFile -LiteralPath $Guard.LockPath
    if ($Guard.LockMetadataPath) {
        Remove-VidaBuildGuardLocalFile -LiteralPath $Guard.LockMetadataPath
    }

    foreach ($entry in @(
            @("VIDA_BUILD_SCRIPT_LOCK_HELD", $Guard.PreviousHeld),
            @("VIDA_BUILD_SCRIPT_LOCK_PATH", $Guard.PreviousPath),
            @("VIDA_BUILD_SCRIPT_LOCK_SCOPE", $Guard.PreviousScope),
            @("VIDA_BUILD_SCRIPT_LOCK_PID", $Guard.PreviousPid),
            @("VIDA_BUILD_SCRIPT_LOCK_TOKEN", $Guard.PreviousToken),
            @("VIDA_BUILD_SCRIPT_LOCK_INHERITED_TOKEN", $Guard.PreviousInheritedToken)
        )) {
        if ($null -eq $entry[1]) {
            Remove-Item "Env:$($entry[0])" -ErrorAction SilentlyContinue
        } else {
            Set-Item "Env:$($entry[0])" -Value $entry[1]
        }
    }
}
