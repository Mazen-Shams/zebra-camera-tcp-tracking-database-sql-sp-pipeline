# TreadScan_Python_Trace: Production Industrial Barcode Service

This directory contains the production-ready 100% Python replacement service, multi-target database execution pipeline, real-time WebSocket dashboard, and diagnostic simulation tools.

---

## 📁 Component Architecture

- **[`python_listener.py`](python_listener.py)**: Master 27-camera concurrent `asyncio` listener service + OS TCP Keep-Alive (`SIO_KEEPALIVE_VALS`) + 5.0s Timeout & Safe Bounded Exponential Backoff + SO_LINGER TCP Reset + Self-Healing Worker Watchdog + Web UI on port `8088` + Hot-Reloading config watcher.
- **[`devices.ini`](devices.ini)**: Camera machine targets config file (located right next to `python_listener.py`).
- **[`db_manager.py`](db_manager.py)**: Consolidated multi-target `pyodbc` execution engine supporting Local CSV audit and Factory Production SQL Server (`sql-server.plant.local`) with 15s circuit breaker and dry-run safety.
- **[`.env`](.env)**: Hot-reloadable environment file to toggle database targets (`ENABLE_LOCAL_AUDIT`, `ENABLE_PRODUCTION`) and `ENABLE_LIVE_COMMIT`.
- **[`dashboard.html`](dashboard.html)**: Premium dark-mode HTML/CSS/JS dashboard rendering 27-camera grid in exact `devices.ini` sequence.
- **[`tools/`](tools/)**:
  - `fake_camera.py`: Simulated Zebra Industrial Scanner on `0.0.0.0:22172`.
  - `logger.py`: Categorized logger with automatic 10 MB file rotation (`.old`).
  - `test_sp.py`: Unified Stored Procedure verification utility (Safe Dry-Run default, `--live` mode).
  - `sql_cli.py`: Interactive SQL terminal utility.
  - `install_dependencies.py`: Python dependency installer.
- **[`launchers/`](launchers/)**: Windows batch launchers (`run_python_listener.bat`, `run_fake_camera.bat`, `install_dependencies.bat`).
- **[`python_portable/`](python_portable/)**: Self-contained embedded Python runtime for zero-install deployment.

---

## 🚀 Quick Start & Service Execution

### 1. Launching the Python Listener Service
Run the launcher batch script:
```cmd
launchers\run_python_listener.bat
```
Or run directly with Python:
```powershell
python python_listener.py
```
Open the live dashboard in any browser at: `http://localhost:8088` (or `http://localhost:8080`).

### 2. Running the Simulated Camera Stream
In a separate terminal, launch the mock Zebra scanner:
```cmd
launchers\run_fake_camera.bat
```

### 3. Testing Database Stored Procedure Execution
Test connection to production SQL Server (`sql-server.plant.local`) in safe dry-run mode (0 rows committed):
```powershell
python tools/test_sp.py TRGB1 TEST_BARCODE
```
For live insertion into the database:
```powershell
python tools/test_sp.py TRGB1 TEST_BARCODE --live
```

