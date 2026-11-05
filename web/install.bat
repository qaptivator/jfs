@echo off
setlocal EnableExtensions
where powershell >nul 2>&1
if errorlevel 1 (
    echo jfs installer requires Windows PowerShell.
    exit /b 1
)
powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "irm https://jfs.acdev.app/install.ps1 | iex"
exit /b %ERRORLEVEL%
