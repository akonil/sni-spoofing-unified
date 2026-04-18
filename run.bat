@echo off
setlocal enabledelayedexpansion

REM Check if running as administrator
net session >nul 2>&1
if %errorlevel% neq 0 (
    echo Error: This script must be run as Administrator.
    echo Right-click Command Prompt and select "Run as administrator", then run this script again.
    pause
    exit /b 1
)

set PROJECT_DIR=%~dp0
set BINARY=%PROJECT_DIR%target\release\sni-spoof.exe
set CONFIG=%1
if "%CONFIG%"=="" set CONFIG=%PROJECT_DIR%config.json

if not exist "%CONFIG%" (
    echo Error: Config file not found: %CONFIG%
    pause
    exit /b 1
)

if not exist "%BINARY%" (
    echo Binary not found. Building...
    cd /d "%PROJECT_DIR%"
    cargo build --release
    if !errorlevel! neq 0 (
        echo Build failed.
        pause
        exit /b 1
    )
)

echo Starting SNI Spoof Proxy...
echo Config: %CONFIG%
echo.

if not defined RUST_LOG set RUST_LOG=warn
"%BINARY%" "%CONFIG%"
pause
