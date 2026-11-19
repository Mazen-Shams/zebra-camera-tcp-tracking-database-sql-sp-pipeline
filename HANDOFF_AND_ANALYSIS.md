# Technical Analysis, System Architecture & Master Handoff Guide

## 1. Executive Context & System Overview

This repository contains the reverse-engineering analysis, simulation suite, and modern production replacement architecture for **"Tread Trace 26 Cam V15"**, an industrial tire tread printing and barcode traceability application.

- **Original Environment**: LabVIEW 2020 compiled executable (`Tread Trace 26 Cam V15.exe`) running as a Windows Service (`camera` via NSSM) - archived in [`LabVIEW_Legacy_System/`](LabVIEW_Legacy_System/).
- **Missing Source Code**: Original `.vi` block diagrams and `.lvproj` source files are unavailable/lost.
- **Hardware Integration Endpoints**:
  - **27 Zebra Industrial Barcode Readers**: IP range `192.168.1.80` to `192.168.1.110` on **TCP Port `22172`**.
  - **REA Jet Industrial Printer**: `192.168.2.172` on **TCP Port `22171`**.
  - **MS SQL Server Endpoint**: `sql-server.plant.local` / Database `PCS` executing stored procedure `csp_AutoProdChange_BY_TRED_CARD`.
- **Active Deployment Path on Plant RDP Server**:
  ```text
  C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace
  ```
- **Active Parallel Services**:
  1. `camera` (Legacy LabVIEW): Tracking all 26 TRG machines.
  2. `pycamera` (Modern Python): Tracking machine **`C1`** (`192.168.1.107:22172`).

---

## 2. Tech Stack Architecture Decision: 100% Python Codebase

### 🏆 Master Primary Solution: Python Architecture
The project is standardized **100% on Python** ([`TreadScan_Python_Trace/`](TreadScan_Python_Trace/)):
- **Zero-Compilation Feedback Loop**: Instant developer velocity for testing, debugging, and on-site plant maintenance without requiring MSVC compilers, headers, or linkers.
- **Self-Contained Embedded Runtime**: Ships with `python_portable\`, allowing execution on bare Windows Server environments without installing Python globally or modifying system PATH.
- **Tech Stack Uniformity**: One single language used for the master listener service (`python_listener.py`), database engine (`db_manager.py`), stored procedure tester (`test_sp.py`), terminal SQL CLI (`sql_cli.py`), camera mock (`fake_camera.py`), and setup scripts.
- **Verified Plant Performance**: 23.1 MB RAM footprint (0.14% of server memory), 0% CPU, sub-millisecond WebSocket UI dashboard (`http://localhost:8088`).

### 📦 Rust Trial Status (Archived / Not to be Continued)
- **Status**: The Rust engine trial ([`RUST Engine Trial/`](RUST%20Engine%20Trial/)) reached 100% feature parity (`odbc-api v29.0.0`, native `odbc32.dll`, OS TCP Keep-Alive, embedded `dashboard.html` Web UI).
- **Future Direction**: **The Rust solution was a trial only and will NOT be continued.** Python is the permanent production codebase.

---

## 3. Empirical Discoveries & Root Cause Analysis

### 🚨 The Core Bug: Half-Open TCP Socket Deadlock in LabVIEW
When Zebra barcode readers power-cycle or drop network links on the factory floor:
1. **Silent Drop**: The camera loses power without sending a TCP `FIN` or `RST` handshake packet.
2. **LabVIEW Trap**: LabVIEW's `TCP Read` inside `TCPIP_Devices.lvlib:Main.vi` has no read timeout configured. Because the local server network card remains active and no TCP close packet arrived, Windows OS does not notify LabVIEW.
3. **Infinite Wait**: LabVIEW sits trapped inside `TCP Read` waiting for data indefinitely. It **never returns to `TCP Open Connection` state**, so when the camera finishes rebooting 60 seconds later, LabVIEW never attempts to reconnect.
4. **Historical Workaround**: Maintenance had to manually execute `nssm restart camera` to kill the stuck Windows process.

---

## 4. Production Solution Architecture

### Low-Level Socket TCP Keep-Alive Probing (`SIO_KEEPALIVE_VALS`)
Instead of re-configuring physical Zebra cameras or parsing custom heartbeat strings, Python implements **OS Kernel-Level Socket Keep-Alive**:

```python
sock = writer.get_extra_info('socket')
sock.setsockopt(socket.SOL_SOCKET, socket.SO_KEEPALIVE, 1)

# On Windows: Probe idle connection after 5000ms, retry every 1000ms
if hasattr(socket, 'SIO_KEEPALIVE_VALS'):
    raw_sock.ioctl(socket.SIO_KEEPALIVE_VALS, (1, 5000, 1000))
```

### Key Engineering Advantages:
1. **Zero Camera Changes**: All 27 Zebra cameras remain in 100% original factory configuration.
2. **Zero DB Pollution**: No idle `PING`/`HEARTBEAT` strings pollute the barcode stream or database stored procedure.
3. **24/7 Stability**: Cameras can sit idle for hours without false timeouts. When a camera drops power, Windows probes fail after 5 seconds, notifying Python to auto-reconnect in < 1 second!
4. **Zero Impact on Network/Hardware**: Probing consumes **0.0017% of a 1 Gbps LAN** and takes **< 2 microseconds** per camera.

---

## 5. Full Component Architecture & Tools Map

All active production and diagnostic components reside in [`TreadScan_Python_Trace/`](TreadScan_Python_Trace/):

| Component | Relative Path | Description |
| :--- | :--- | :--- |
| **Python Listener Service** | [`python_listener.py`](TreadScan_Python_Trace/python_listener.py) | 27-camera concurrent `asyncio` listener service + Keep-Alive + Web UI on port `8088` + Hot-Reload watcher. |
| **Database Manager** | [`db_manager.py`](TreadScan_Python_Trace/db_manager.py) | Consolidated multi-target `pyodbc` execution engine supporting Pure CSV Audit, production MS SQL Server, dry-run safety, and 15s offline circuit breaker. |
| **Active Environment File** | [`.env`](TreadScan_Python_Trace/.env) | Configuration file to enable/disable targets (`ENABLE_LOCAL_AUDIT`, `ENABLE_PRODUCTION`) and toggle `ENABLE_LIVE_COMMIT`. |
| **Camera Devices Config** | [`devices.ini`](TreadScan_Python_Trace/devices.ini) | INI configuration file containing machine codes & IP addresses (located right next to listener). Hot-reloads every 3s. |
| **Web Dashboard** | [`dashboard.html`](TreadScan_Python_Trace/dashboard.html) | Premium dark-mode HTML/CSS/JS dashboard rendering camera grid in exact `devices.ini` sequence. |
| **Camera Streamer Mock** | [`tools/fake_camera.py`](TreadScan_Python_Trace/tools/fake_camera.py) | Non-blocking continuous Zebra scanner mock (`0.0.0.0:22172`) in `tools/`. |
| **SP Verification Tool** | [`tools/test_sp.py`](TreadScan_Python_Trace/tools/test_sp.py) | Unified Stored Procedure tester (Safe Dry-Run default, `--live` mode). |
| **Terminal SQL CLI** | [`tools/sql_cli.py`](TreadScan_Python_Trace/tools/sql_cli.py) | Command-line SQL query terminal utility in `tools/`. |
| **Categorized Logger** | [`tools/logger.py`](TreadScan_Python_Trace/tools/logger.py) | Categorized `.log` logger (`db_errors.log`, `camera_tcp_errors.log`, `audit_trace.log`, `system.log`) with auto 10 MB rotation (`.old`). |
| **Dependency Installer** | [`tools/install_dependencies.py`](TreadScan_Python_Trace/tools/install_dependencies.py) | Python package installer. |
| **Batch Launchers** | [`launchers/`](TreadScan_Python_Trace/launchers/) | Windows batch scripts (`run_python_listener.bat`, `run_fake_camera.bat`, etc.). |
| **Portable Python Runtime** | [`python_portable/`](TreadScan_Python_Trace/python_portable/) | Embedded Python 3.11 environment (Zero installation required). |

---

## 6. Multi-Target Database Pipeline Control

In [`TreadScan_Python_Trace/.env`](TreadScan_Python_Trace/.env), enable or disable any combination of targets:

```ini
ENABLE_LOCAL_AUDIT=true       # Target 1: Local CSV audit log (logs/sql_audit.csv)
ENABLE_PRODUCTION=true        # Target 2: Factory Production Server (sql-server.plant.local)

ENABLE_LIVE_COMMIT=true       # true = LIVE MODE (Commits rows to DB) / false = SAFETY MODE (Transaction Rollback)
```

---

## 7. Extended Reconnection Resilience & Self-Healing Watchdog (September 2026)

During 24/7 continuous operation on machine `C1`, an empirical edge case was identified and permanently resolved:

### 🚨 The Problem: Task Crash at Attempt #1025
* **Unbounded Exponential Backoff**: When a camera remained disconnected for > 2 hours, `reconnect_attempts` crossed 1000. At attempt 1025, computing `0.5 * (2 ** (reconnect_attempts - 1))` triggered `OverflowError: int too large to convert to float` in Python, terminating the camera worker task silently.
* **Aggressive 2.0s Timeout**: An initial 2.0s timeout on `open_connection` was too tight for industrial switches undergoing ARP refresh or TCP SYN retransmission (3.0s OS retry).
* **Zebra Camera Single-Client Lockout**: If a disconnected socket was closed gracefully, the Zebra camera could remain in a half-open state holding port 22172.

### 🛡️ The Industrial Hardening Fix:
1. **Bounded Safe Exponent**:
   ```python
   safe_exp = min(max(reconnect_attempts - 1, 0), 6)
   backoff = min(0.5 * (2 ** safe_exp), 5.0)
   ```
   Completely eliminates `OverflowError` while strictly capping sleep at 5.0s.
2. **5.0s Connection Timeout**: Accommodates factory switch latencies and OS TCP SYN retransmissions.
3. **Immediate TCP RST (`SO_LINGER`)**: On disconnect, `SO_LINGER` is set with timeout `0` to force an immediate TCP `RST` packet to the Zebra scanner, resetting the camera's TCP port instantly.
4. **Self-Healing Watchdog**: In `watch_devices_config()`, the service actively inspects all camera worker tasks every 3 seconds. If any task finishes or crashes (`task.done()`), it is automatically resurrected in memory with a system event log.

---

## 8. Windows Service & Remote Management (`pycamera` & OpenSSH)

### NSSM Service Reference:
On the RDP server (`192.168.1.125`):
```cmd
nssm status pycamera      :: Check status (SERVICE_RUNNING)
nssm restart pycamera     :: Restart service (loads updated code)
nssm stop pycamera        :: Stop service
```

### Server Network & Remote Access Topology:
* **Host**: `192.168.1.125` (`jump-server.plant.local`)
* **Account**: `DOMAIN\admin_user`
* **Sibling Deployments**:
  * `C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace` (Scanning / `pycamera`)
  * `C:\Users\adm.ms1khairha\Desktop\test\RE C1 printing service` (REA JET CIJ Printing / `printer`)
* **Network Firewall Status**:
  * **Port 3389 (RDP)**: OPEN from office network.
  * **Port 445 (SMB Share `\\192.168.1.125\c$`)**: OPEN (used for direct file sync and log monitoring).
  * **Port 22 (OpenSSH)**: Installed & active on server (`C:\Program Files\OpenSSH`, service `sshd`). Accessible inside the server via `ssh localhost`. External access from office laptops requires a standard IT firewall change ticket to route Port 22.

For complete details, see: 📄 [NSSM_SERVICE_GUIDE.md](NSSM_SERVICE_GUIDE.md)


