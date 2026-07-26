if (-not ("VidaProcessRunner" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Diagnostics;
using System.Text;
using System.Threading;

public static class VidaProcessRunner
{
    public sealed class Result
    {
        public int ExitCode { get; set; }
        public string Stdout { get; set; }
        public string Stderr { get; set; }
    }

    public static Result Run(string filePath, string[] arguments, string workingDirectory, bool captureOutput, int flushTimeoutMilliseconds)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = filePath,
            UseShellExecute = false,
            WorkingDirectory = workingDirectory ?? string.Empty,
            RedirectStandardOutput = captureOutput,
            RedirectStandardError = captureOutput
        };
        foreach (var argument in arguments ?? Array.Empty<string>())
        {
            startInfo.ArgumentList.Add(argument ?? string.Empty);
        }

        var stdout = new StringBuilder();
        var stderr = new StringBuilder();
        using var stdoutDone = new ManualResetEventSlim(!captureOutput);
        using var stderrDone = new ManualResetEventSlim(!captureOutput);
        using var process = new Process { StartInfo = startInfo };
        if (captureOutput)
        {
            process.OutputDataReceived += (_, eventArgs) =>
            {
                if (eventArgs.Data != null) stdout.AppendLine(eventArgs.Data);
                else stdoutDone.Set();
            };
            process.ErrorDataReceived += (_, eventArgs) =>
            {
                if (eventArgs.Data != null) stderr.AppendLine(eventArgs.Data);
                else stderrDone.Set();
            };
        }
        if (!process.Start()) throw new InvalidOperationException("Failed to start process: " + filePath);
        if (captureOutput)
        {
            process.BeginOutputReadLine();
            process.BeginErrorReadLine();
        }
        while (!process.HasExited)
        {
            process.WaitForExit(250);
        }
        if (captureOutput)
        {
            var deadline = DateTime.UtcNow.AddMilliseconds(flushTimeoutMilliseconds);
            var remaining = Math.Max(0, (int)(deadline - DateTime.UtcNow).TotalMilliseconds);
            stdoutDone.Wait(remaining);
            remaining = Math.Max(0, (int)(deadline - DateTime.UtcNow).TotalMilliseconds);
            stderrDone.Wait(remaining);
        }
        return new Result { ExitCode = process.ExitCode, Stdout = stdout.ToString(), Stderr = stderr.ToString() };
    }
}
"@
}

function Invoke-VidaProcess {
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,
        [string[]]$ArgumentList = @(),
        [string]$WorkingDirectory = "",
        [string]$StdoutPath = "",
        [string]$StderrPath = "",
        [int]$OutputFlushTimeoutMilliseconds = 1000
    )

    $captureOutput = -not [string]::IsNullOrWhiteSpace($StdoutPath) -or
        -not [string]::IsNullOrWhiteSpace($StderrPath)
    $result = [VidaProcessRunner]::Run(
        $FilePath,
        [string[]]$ArgumentList,
        $WorkingDirectory,
        $captureOutput,
        $OutputFlushTimeoutMilliseconds
    )
    if ($captureOutput) {
        $encoding = New-Object System.Text.UTF8Encoding $false
        if (-not [string]::IsNullOrWhiteSpace($StdoutPath)) {
            [System.IO.File]::WriteAllText($StdoutPath, $result.Stdout, $encoding)
        }
        if (-not [string]::IsNullOrWhiteSpace($StderrPath)) {
            [System.IO.File]::WriteAllText($StderrPath, $result.Stderr, $encoding)
        }
    }
    return [int]$result.ExitCode
}
