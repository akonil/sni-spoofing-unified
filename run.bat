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
set ARG=%1

REM Check if argument is a flag (starts with --)
if "%ARG:~0,2%"=="--" (
    REM Pass flag directly to binary
    echo Running: %BINARY% %ARG%
    echo.
    if not defined RUST_LOG set RUST_LOG=warn
    "%BINARY%" %ARG%
    pause
    exit /b 0
)

REM Handle config file
set CONFIG=%ARG%
if "%CONFIG%"=="" set CONFIG=%PROJECT_DIR%config.json

if not exist "%CONFIG%" (
    echo Error: Config file not found: %CONFIG%
    echo.
    echo Usage:
    echo   run.bat               - Run with default config.json
    echo   run.bat path\config   - Run with custom config
    echo   run.bat --wizard      - Interactive setup
    echo   run.bat --preset hcaptcha - Use hCaptcha preset
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
