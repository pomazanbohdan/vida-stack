$ErrorActionPreference = "Stop"

$rootDir = Split-Path -Parent $PSScriptRoot
$windowsEnvScript = Join-Path $PSScriptRoot "vida-windows-env.ps1"
if (-not (Test-Path -LiteralPath $windowsEnvScript -PathType Leaf)) {
    throw "[vida-cargo-msvc] Missing Windows environment helper: $windowsEnvScript"
}

. $windowsEnvScript
. (Join-Path $PSScriptRoot "vida-process-runner.ps1")

# Leave VIDA_MSVC_TEMP_DIR unset by default so vida-windows-env can pick an
# external writable temp root. Repo-local temp roots create nested VIDA projects
# and can make project-root discovery ambiguous during bootstrap-heavy tests.
if ([string]::IsNullOrWhiteSpace($env:CARGO_TERM_COLOR)) {
    $env:CARGO_TERM_COLOR = "never"
}
if ([string]::IsNullOrWhiteSpace($env:CARGO_TERM_PROGRESS_WHEN)) {
    $env:CARGO_TERM_PROGRESS_WHEN = "never"
}

Import-VidaMsvcEnvironment

& (Join-Path $PSScriptRoot "verify-rust-toolchain.ps1") | Out-Null

$cargo = Get-Command "cargo" -ErrorAction Stop
$cargoArgs = @($args)
if ($cargoArgs.Count -eq 0) {
    throw "[vida-cargo-msvc] Pass cargo arguments, for example: test -p vida --bin vida status_surface"
}

function Convert-VidaCargoTestArgs {
    param([string[]]$Arguments)

    if ($Arguments.Count -eq 0 -or $Arguments[0] -ne "test" -or $Arguments -contains "--") {
        return $Arguments
    }

    $harnessFlags = @(
        "--nocapture",
        "--test-threads",
        "--exact",
        "--ignored",
        "--include-ignored",
        "--skip",
        "--show-output",
        "--format"
    )
    for ($index = 1; $index -lt $Arguments.Count; $index++) {
        $arg = $Arguments[$index]
        $isHarnessArg = $false
        foreach ($flag in $harnessFlags) {
            if ($arg -eq $flag -or $arg.StartsWith("$flag=")) {
                $isHarnessArg = $true
                break
            }
        }
        if ($isHarnessArg) {
            return @($Arguments[0..($index - 1)] + @("--") + $Arguments[$index..($Arguments.Count - 1)])
        }
    }

    return $Arguments
}

$cargoArgs = @(Convert-VidaCargoTestArgs -Arguments $cargoArgs)

$exitCode = Invoke-VidaProcess -FilePath $cargo.Source -ArgumentList $cargoArgs -WorkingDirectory $rootDir
exit $exitCode
