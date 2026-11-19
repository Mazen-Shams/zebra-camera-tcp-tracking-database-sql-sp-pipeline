@echo off
title Zebra Camera Simulator (127.0.0.1:22172)
cd /d "%~dp0.."
echo ========================================================
echo   Starting Simulated Zebra Camera Server on 127.0.0.1:22172
echo ========================================================
python tools/fake_camera.py
pause
