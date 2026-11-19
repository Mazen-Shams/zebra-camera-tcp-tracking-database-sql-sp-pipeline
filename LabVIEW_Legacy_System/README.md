# Legacy LabVIEW 2020 System ("Tread Trace 26 Cam V15")

This directory contains the original compiled LabVIEW binary application and its legacy runtime environment.

---

## 📁 Contents

- **`Tread Trace 26 Cam V15.exe`**: Compiled LabVIEW application (originally executed as Windows Service `camera` via NSSM).
- **`Tread Trace 26 Cam V15.ini` & `.aliases`**: LabVIEW runtime configuration.
- **`niwebserver.conf`**: National Instruments web server config.
- **`data/`**: Configuration INI files, device lists, error logs, and table data used by the LabVIEW application.
- **`run_labview_app.bat`**: Windows batch launcher to run the executable locally.

---

## ⚠️ Known Production Limitation: Half-Open TCP Socket Deadlock

When a Zebra camera drops power or network link:
1. The camera terminates without sending a TCP `FIN` packet.
2. LabVIEW's `TCP Read` has no timeout and waits indefinitely.
3. Windows defaults to 2 hours of idle before notifying the process.
4. LabVIEW never returns to the `TCP Open Connection` state, so it never reconnects when the camera reboots.

> **Production Replacement**: This legacy application has been superseded by the modern auto-reconnecting Python service in [`../TreadScan_Python_Trace/`](../TreadScan_Python_Trace/).

