@echo off
title Python Async Barcode Listener
cd /d "%~dp0.."
echo ========================================================
echo   Starting Python Auto-Reconnecting Listener
echo ========================================================
echo.

set /p TARGET_IP="Enter target IP(s) space-separated [Press ENTER to load ALL devices from devices.ini]: "

echo.
if "%TARGET_IP%"=="" (
    echo Launching Python Service for ALL devices in devices.ini...
    python python_listener.py
) else (
    echo Launching Python Service for Target IP(s): %TARGET_IP%
    python python_listener.py %TARGET_IP%
)
pause
