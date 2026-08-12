@echo off
setlocal
call "%~dp0vida-dev-gate.cmd" -Mode semantic-focused -Json
exit /b %errorlevel%
