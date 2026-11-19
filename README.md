# 🦓 Zebra Industrial Camera Async TCP Tracking & SQL Database Stored Procedure Pipeline

**Enterprise Multi-Camera Barcode Traceability, Low-Level TCP Keep-Alive Probing, and Transactional SQL Server Pipeline**

> **Overview**: Production-grade industrial tracking architecture replacing legacy LabVIEW camera services across 26 tire-building machines. Combines an asynchronous Python socket engine with low-level Windows kernel socket Keep-Alive probing (SIO_KEEPALIVE_VALS), automated reconnection, live WebSocket-streamed dark-mode dashboard (:8088), and transaction-safe MS SQL Server stored procedure dispatch (csp_AutoProdChange_BY_TRED_CARD).

---
# Manex Tread Printing RE - Reverse Engineering & Production Replacement

This repository contains the complete reverse-engineering analysis, simulation suite, and production Python replacement architecture for **"Tread Trace 26 Cam V15"**, an industrial tire tread printing and barcode traceability system.

---

## 📌 Repository Organization

### 1. 📂 **[`TreadScan_Python_Trace/`](TreadScan_Python_Trace/)** — Master Python Production Suite
The active, 100% Python production service and industrial suite:
* **`python_listener.py`**: Concurrent Async TCP Listener + OS-Level Keep-Alive (`SIO_KEEPALIVE_VALS`) + Web UI on port `8088`.
* **`db_manager.py`**: Consolidated database manager supporting Pure CSV Audit logging (`logs/sql_audit.csv`) and Factory Production MS SQL Server (`sql-server.plant.local` / `csp_AutoProdChange_BY_TRED_CARD`) with `ENABLE_LIVE_COMMIT` dry-run rollback safety.
* **`dashboard.html`**: Embedded real-time dark-mode status grid and live WebSocket scan feed.
* **`.env`**: Central environment configuration file (Hot-Reloadable).
* **`tools/`**: Diagnostic tools (`test_sp.py`, `sql_cli.py`, `fake_camera.py`, `logger.py`, `install_dependencies.py`).
* **`python_portable/`**: Self-contained embedded Python runtime for zero-install deployment.

### 2. 📂 **[`LabVIEW_Legacy_System/`](LabVIEW_Legacy_System/)** — Legacy LabVIEW Archive
The original compiled LabVIEW binary application (`Tread Trace 26 Cam V15.exe`), data configuration INI files, error logs, and legacy launcher.

### 3. 📂 **[`RUST Engine Trial/`](RUST%20Engine%20Trial/)** & **[`TreadTrace_Plant_Release/`](TreadTrace_Plant_Release/)** — Archived Rust Trial
The completed Rust engine trial (archived and not continued).

---

## 📄 Master Documentation Suite

For full technical analysis, socket deadlock root cause findings, and deployment steps, refer to:
* 👉 **[starting_prompt.md](starting_prompt.md)**: Ready-to-use starting prompt to bootstrap future AI agent sessions.
* 👉 **[HANDOFF_AND_ANALYSIS.md](HANDOFF_AND_ANALYSIS.md)**: Master handoff document, empirical findings, and historical analysis.
* 👉 **[TECHNICAL_ARCHITECTURE.md](TECHNICAL_ARCHITECTURE.md)**: Low-level engineering architecture, OS socket Keep-Alive math, and database pipeline.
* 👉 **[NSSM_SERVICE_GUIDE.md](NSSM_SERVICE_GUIDE.md)**: Windows Service management reference (`pycamera` & `camera`), CLI cheat sheet, and troubleshooting.


