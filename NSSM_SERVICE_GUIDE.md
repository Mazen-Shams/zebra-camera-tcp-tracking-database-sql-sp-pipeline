# NSSM (Non-Sucking Service Manager) Complete Industrial Guide
**Project**: Manex Tread Printing RE ("Tread Trace 26 Cam V15" Python Replacement)  
**Target Services**: `pycamera` (Python Service) & `camera` (Legacy LabVIEW Service)  
**Host Environment**: Windows Server (RDP)  

---

## 1. Executive Summary & Why NSSM is Used
In industrial 24/7 manufacturing environments, background worker applications (like camera TCP listeners and barcode traceability pipelines) must run continuously without requiring an active user desktop session or an open command prompt window.

**NSSM (Non-Sucking Service Manager)** wraps standard executables (`.exe`) into native Windows Services (`services.msc`), providing:
- **Auto-Startup on Boot**: Runs before any user logs in over RDP.
- **Crash Recovery & Auto-Restart**: If the process terminates unexpectedly, NSSM restarts it within milliseconds.
- **I/O Redirection**: Routes standard output (`stdout`) and error output (`stderr`) directly into categorized log files.
- **Zero Orphan/Zombie Processes**: Sends direct termination signals to the target process tree rather than leaving child processes stuck holding TCP ports.

---

## 2. Active Production Deployment Paths

On the factory RDP server, the active production deployment path is:
```text
C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace
```

### Registered Service Definition (`pycamera`):
| Parameter | Value |
| :--- | :--- |
| **Service Name** | `pycamera` |
| **Application Path** | `C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace\python_portable\python.exe` |
| **Startup Directory (AppDirectory)** | `C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace` |
| **Parameters (AppParameters)** | `python_listener.py` |
| **Startup Type** | `SERVICE_AUTO_START` |
| **Web UI Dashboard** | `http://localhost:8088` |

---

## 3. Essential Command-Line Cheat Sheet

> [!IMPORTANT]
> Always run Command Prompt or PowerShell as **Administrator** when executing `nssm` commands.

### 3.1 Checking Service Status
```powershell
# Check current operational status of pycamera
nssm status pycamera

# Check status of legacy LabVIEW camera service
nssm status camera
```
**Expected Return Codes**:
- `SERVICE_RUNNING`: Active and operating normally in the background.
- `SERVICE_STOPPED`: Gracefully stopped by administrator.
- `SERVICE_PAUSED`: Rapid-crash restart throttling triggered (see Troubleshooting).

### 3.2 Starting, Stopping, and Restarting
```powershell
# Start the Python service
nssm start pycamera

# Restart the Python service (e.g. after code changes)
nssm restart pycamera

# Gracefully stop the service
nssm stop pycamera
```

### 3.3 Listing All NSSM & Windows Services
```powershell
# View both camera services side-by-side in PowerShell
Get-Service -Name *camera*

# View all running Windows services
Get-Service | Where-Object {$_.Status -eq "Running"}
```

---

## 4. How to Install & Configure a New Service

### Step-by-Step CLI Command Sequence:
```powershell
# 1. Install service pointing to Portable Python
nssm install pycamera "C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace\python_portable\python.exe" "python_listener.py"

# 2. Set the working directory (Crucial for resolving .env, devices.ini, and logs)
nssm set pycamera AppDirectory "C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace"

# 3. Configure automatic boot start and human-readable description
nssm set pycamera Start SERVICE_AUTO_START
nssm set pycamera Description "TreadScan Python C1 Barcode Traceability Service"

# 4. (Recommended) Configure stdout/stderr file logging
nssm set pycamera AppStdout "C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace\logs\nssm_stdout.log"
nssm set pycamera AppStderr "C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace\logs\nssm_stderr.log"

# 5. Start the service
nssm start pycamera
```

---

## 5. How to Edit an Existing Service

### Method A: Open the Interactive GUI Editor
If you want to view or modify settings using the visual window:
```powershell
nssm edit pycamera
```
*(Opens the GUI editor showing all tabs: Application, Details, Log on, Dependencies, Process, Shutdown, Exit actions, I/O, File rotation, Environment).*

### Method B: Modify Individual Settings via CLI
```powershell
# Change the startup parameters
nssm set pycamera AppParameters "python_listener.py --verbose"

# Change the startup directory
nssm set pycamera AppDirectory "C:\New\Path\TreadScan_Python_Trace"

# Query the current value of a parameter
nssm get pycamera AppDirectory
```

---

## 6. How to Safely Remove a Service

To completely delete a service from Windows (`services.msc`):

```powershell
# 1. Stop the service first
nssm stop pycamera

# 2. Remove the service (with confirmation flag)
nssm remove pycamera confirm
```
> [!NOTE]
> Removing a service via NSSM only unregisters it from Windows Service Manager. It does **not** delete any code, logs, or python files on your hard drive.

---

## 7. Deep Troubleshooting & Root Cause Resolutions

### Issue 1: `pycamera: Unexpected status SERVICE_PAUSED in response to START control`
* **Root Cause**: Python crashed immediately upon launch (e.g. invalid file path, syntax error, missing library, or port collision). NSSM's built-in **Restart Throttle** paused the service to prevent CPU pegging.
* **Resolution**:
  1. Test execution interactively in terminal to see the exact red traceback:
     ```powershell
     cd "C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace"
     .\python_portable\python.exe python_listener.py
     ```
  2. If an error is printed (e.g. path error, port busy), fix it in `.env` or `devices.ini`.
  3. Reset the service:
     ```powershell
     nssm stop pycamera
     nssm start pycamera
     ```

### Issue 2: Paths with Spaces
* **Problem**: Paths containing spaces (like `Manex Tread Scanning Python Implementation`) can be misinterpreted as multiple arguments.
* **Resolution**: Always wrap paths in double quotes:
  ```powershell
  nssm set pycamera Application "C:\Users\adm.ms1khairha\Desktop\test\Manex Tread Scanning Python Implementation\TreadScan_Python_Trace\python_portable\python.exe"
  ```

### Issue 3: Zombie Process Holding Port `8088` or `22172`
* **Problem**: If someone ran `python_listener.py` manually in a background terminal, the port is locked, preventing NSSM from binding to `8088`.
* **Resolution**:
  1. Check what process owns the port:
     ```cmd
     netstat -ano | findstr :8088
     ```
  2. Kill the conflicting PID:
     ```cmd
     taskkill /F /PID <PID_NUMBER>
     ```
  3. Restart the service:
     ```powershell
     nssm restart pycamera
     ```

---

## 8. Dual-Service Architecture Summary

The plant server runs two independent services concurrently:

```mermaid
flowchart LR
    subgraph Windows Service Manager (services.msc)
        S1["camera (Legacy LabVIEW)"]
        S2["pycamera (Modern Python)"]
    end
    
    subgraph Physical Hardware Endpoints
        TRG["26 TRG Cameras\n(192.168.1.80 - 104)"]
        C1["Camera C1\n(192.168.1.107:22172)"]
    end

    subgraph Database Target
        SQL["MS SQL Server (sql-server.plant.local)\nDB: PCS | SP: csp_AutoProdChange_BY_TRED_CARD"]
    end

    S1 -->|Listens & Scans| TRG
    S1 -->|Executes SP| SQL

    S2 -->|Listens & Scans (TCP Keep-Alive)| C1
    S2 -->|Executes SP (Dry-Run / Live)| SQL
    S2 -->|Web Dashboard| UI["http://localhost:8088"]
```

---

## 9. Server CLI Administration via OpenSSH (`sshd`)

To manage NSSM services and execute Python diagnostics from a command-line interface without opening the full RDP graphical desktop, **OpenSSH Server** is installed on the host machine (`192.168.1.125`).

### 9.1 Service Definition & Paths:
* **Installation Directory**: `C:\Program Files\OpenSSH\`
* **Windows Service**: `sshd` (OpenSSH SSH Server, Startup Type: `Automatic`)
* **Configuration File**: `C:\ProgramData\ssh\sshd_config`
* **Firewall Rule**: `OpenSSH-Server-In-TCP` (TCP Port `22` Inbound, Enabled)

### 9.2 Managing Windows Services via SSH:
When connected to the server via SSH:
```cmd
:: Check status
nssm status pycamera

:: Restart the Python scanner service after code updates
nssm restart pycamera

:: Check legacy LabVIEW service
nssm status camera

:: Check REA JET printer service
nssm status printer
```

### 9.3 Network Perimeter Routing Note:
* **Inside Server**: Works immediately via `ssh localhost` or `ssh adm.ms1khairha@127.0.0.1`.
* **From Office Laptops**: The plant router firewall between the office network and OT VLAN (`192.168.1.x`) allows Ports `3389` (RDP) and `445` (SMB). To enable direct SSH from office laptops, a standard IT firewall ticket is required to allow TCP Port `22`.


