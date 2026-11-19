@echo off
TITLE Tread Trace 26 Cam - Portable Python Barcode Listener Service
cd /d "%~dp0"
echo =========================================================================
echo  Tread Trace 26 Cam - Portable Python Barcode Listener Service
echo =========================================================================
echo [INFO] Using self-contained Portable Python Environment (Zero Admin Required)
echo [INFO] Web UI Dashboard will be served at http://localhost:8088
echo.

if not exist "%~dp0python_portable\python.exe" (
    echo.
    echo \033[91m[ERROR] The 'python_portable' subfolder is missing in this directory!\033[0m
    echo Please make sure to copy the entire 'python_portable' folder into:
    echo "%~dp0"
    echo.
    pause
    exit /b 1
)

"%~dp0python_portable\python.exe" python_listener.py %*
pause

