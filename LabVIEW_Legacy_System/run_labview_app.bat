@echo off
title Legacy LabVIEW App (Tread Trace 26 Cam V15)
cd /d "%~dp0"
echo ========================================================
echo   Launching Legacy Tread Trace 26 Cam V15.exe
echo ========================================================
echo.

set DEFAULT_IP=10.132.97.153
if "%~1"=="" (
    set /p TARGET_IP="Enter Camera IP address for Device0 (TRGB1) [Default: %DEFAULT_IP%]: "
) else (
    set TARGET_IP=%~1
)

if "%TARGET_IP%"=="" set TARGET_IP=%DEFAULT_IP%

echo.
if exist "data\devices.ini" (
    powershell -Command "(Get-Content 'data\devices.ini') -replace 'IP_Address\s*=\s*\".*?\"', 'IP_Address   = \"'%TARGET_IP%'\"' | Set-Content 'data\devices.ini'"
    echo [CONFIG] Set Device0 IP to %TARGET_IP% in data\devices.ini
)

echo.
echo Launching LabVIEW executable...
start "" "Tread Trace 26 Cam V15.exe"
echo Done!
pause
