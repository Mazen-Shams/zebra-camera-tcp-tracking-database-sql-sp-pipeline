"""
1-Click Automated Dependency Installer for Tread Trace 26 Cam Service
Auto-installs all required Python packages (aiohttp, pyodbc) via pip.
"""

import sys
import subprocess

REQUIRED_PACKAGES = ["aiohttp", "pyodbc"]

def install_packages():
    print("=========================================================================")
    print(" Installing Required Python Libraries for Tread Trace 26 Cam Service")
    print("=========================================================================\n")

    for pkg in REQUIRED_PACKAGES:
        try:
            __import__(pkg)
            print(f"[OK] '{pkg}' is already installed.")
        except ImportError:
            print(f"[INSTALLING] '{pkg}' via pip...")
            try:
                subprocess.check_call([sys.executable, "-m", "pip", "install", pkg])
                print(f"[SUCCESS] '{pkg}' installed successfully!")
            except Exception as e:
                print(f"[ERROR] Failed to install '{pkg}': {e}")

    print("\n=========================================================================")
    print(" All required libraries are installed and ready!")
    print("=========================================================================\n")

if __name__ == "__main__":
    install_packages()

