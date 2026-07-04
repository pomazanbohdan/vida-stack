@echo off
setlocal

if not defined SystemRoot set "SystemRoot=C:\Windows"
if not defined windir set "windir=%SystemRoot%"
if not defined SystemDrive set "SystemDrive=C:"
if not defined ComSpec set "ComSpec=%SystemRoot%\System32\cmd.exe"
if not defined ProgramFiles set "ProgramFiles=C:\Program Files"
if not defined ProgramFiles(x86) set "ProgramFiles(x86)=C:\Program Files (x86)"
if not defined USERNAME if exist "%SystemRoot%\System32\whoami.exe" for /f "tokens=2 delims=\" %%U in ('"%SystemRoot%\System32\whoami.exe"') do if not defined USERNAME set "USERNAME=%%U"
if not defined USERPROFILE if defined HOMEDRIVE if defined HOMEPATH set "USERPROFILE=%HOMEDRIVE%%HOMEPATH%"
if not defined USERPROFILE if defined USERNAME set "USERPROFILE=%SystemDrive%\Users\%USERNAME%"
if defined USERPROFILE if not exist "%USERPROFILE%" set "USERPROFILE="
if not defined HOME if defined USERPROFILE set "HOME=%USERPROFILE%"
if not defined LOCALAPPDATA if defined USERPROFILE set "LOCALAPPDATA=%USERPROFILE%\AppData\Local"
if not defined TEMP if defined LOCALAPPDATA set "TEMP=%LOCALAPPDATA%\Temp"
if not defined TMP set "TMP=%TEMP%"
if defined TEMP if not exist "%TEMP%" mkdir "%TEMP%" >nul 2>nul

set "PATH=%SystemRoot%\System32;%SystemRoot%;%SystemRoot%\System32\Wbem;%LOCALAPPDATA%\Microsoft\WindowsApps;%ProgramFiles%\WindowsApps;%ProgramFiles%\PowerShell\7;%SystemRoot%\System32\WindowsPowerShell\v1.0;C:\Program Files\Git\cmd;C:\Program Files\Git\bin;%LOCALAPPDATA%\vida-stack\current\bin;%USERPROFILE%\.cargo\bin;C:\Users\%USERNAME%\.cargo\bin;%PATH%"

set "PWSH="
if defined VIDA_PWSH if exist "%VIDA_PWSH%" set "PWSH=%VIDA_PWSH%"
if not defined PWSH if exist "%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe" set "PWSH=%LOCALAPPDATA%\Microsoft\WindowsApps\pwsh.exe"
if not defined PWSH for %%P in (pwsh.exe) do if not "%%~$PATH:P"=="" set "PWSH=%%~$PATH:P"
if not defined PWSH if exist "%ProgramFiles%\PowerShell\7\pwsh.exe" set "PWSH=%ProgramFiles%\PowerShell\7\pwsh.exe"

if not defined PWSH (
  echo [vida-dev-gate] ERROR: PowerShell Core pwsh.exe was not found after Windows PATH bootstrap. Install Microsoft.PowerShell with winget and ensure pwsh.exe is on PATH. 1>&2
  exit /b 1
)

"%PWSH%" -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0vida-dev-gate.ps1" %*
exit /b %ERRORLEVEL%
