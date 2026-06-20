@echo off
cd /d "%~dp0"

where cargo >nul 2>&1
if errorlevel 1 (
    echo [ERROR] Cargo was not found. Install Rust from https://rustup.rs/
    pause
    exit /b 1
)

echo [BUILD] Building lmu-dualsense-bridge...
cargo build
if errorlevel 1 (
    echo [ERROR] Build failed.
    pause
    exit /b 1
)

echo [INFO] LMU may be started before or after this bridge.
echo [INFO] The bridge will retry the LMU connection every 2 seconds.
target\debug\lmu-dualsense-bridge.exe --telemetry lmu --output dsx-udp
set "APP_EXIT_CODE=%ERRORLEVEL%"
pause
exit /b %APP_EXIT_CODE%
