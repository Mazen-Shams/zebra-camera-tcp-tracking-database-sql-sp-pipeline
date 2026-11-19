@echo off
TITLE Tread Trace 26 Cam - Portable Python SP Verification Utility
cd /d "%~dp0"
echo =========================================================================
echo  Tread Trace 26 Cam - Portable Python Stored Procedure Tester
echo =========================================================================
echo [INFO] Executing SP test via Portable Python...
echo.

if not exist "%~dp0python_portable\python.exe" (
    echo [ERROR] The 'python_portable' subfolder is missing in this directory!
    echo Please make sure to copy the entire 'python_portable' folder into:
    echo "%~dp0"
    echo.
    pause
    exit /b 1
)

"%~dp0python_portable\python.exe" tools\test_sp.py TRGB1 TEST_PORTABLE_VERIFY %*
pause
