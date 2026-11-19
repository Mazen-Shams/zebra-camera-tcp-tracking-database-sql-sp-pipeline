@echo off
TITLE Tread Trace Rust Engine - Environment Setup Script
echo =========================================================================
echo  Tread Trace 26 Cam - Rust Engine Development Setup
echo =========================================================================
echo.

echo [1/3] Checking Windows Package Manager (winget)...
where winget >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] winget is not installed on this system.
    echo Please install Rustup manually from: https://rustup.rs/
    echo Please install Visual Studio C++ Build Tools from: https://visualstudio.microsoft.com/visual-cpp-build-tools/
    pause
    exit /b 1
)

echo [2/3] Installing Rust Toolchain (Rustup)...
winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements

echo.
echo [3/3] Installing Visual Studio 2022 C++ Build Tools...
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

echo.
echo =========================================================================
echo  Setup Complete! Please restart your terminal window, then run:
echo    cd "RUST Engine Trial"
echo    cargo build --release
echo =========================================================================
pause
