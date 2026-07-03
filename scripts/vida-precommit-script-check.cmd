@echo off
setlocal

call "%~dp0vida-dev-gate.cmd" -Mode script-check
exit /b %ERRORLEVEL%
