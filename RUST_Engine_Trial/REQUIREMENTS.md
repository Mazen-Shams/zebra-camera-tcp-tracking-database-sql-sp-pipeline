# Environment Requirements & Setup Guide: Rust Engine Trial

This document outlines everything required to prepare a **fresh, clean Windows PC** to build, compile, and run the `tread_trace_rust` engine.

---

## 1. Requirements for Building / Compiling (Development PC)

To compile the source code (`.rs` files) into a standalone binary (`tread_trace_rust.exe`), the development machine requires the following 3 components:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       BUILD ENGINE PREREQUISITES                            │
│                                                                             │
│  1. Rust Toolchain (rustup + cargo) ──> Compiles .rs code & manages crates  │
│  2. MSVC C++ Build Tools (link.exe) ──> Links Windows kernel APIs & socket  │
│  3. Configuration Files (.env / ini)──> Configures IP & database targets    │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 🔹 Requirement 1: Rust Toolchain (`rustup` & `cargo`)
* **What it is**: The official Rust language installer (`rustup`) and package manager (`cargo`).
* **Why it's needed**: Rust is a compiled language. `rustc` compiles the source files into machine instructions, while `cargo` automatically downloads and manages project dependencies declared in `Cargo.toml` (`tokio`, `axum`, `tiberius`, `rusqlite`).
* **How to Install**:
  * **Option A (Automatic via winget)**: Run `winget install --id Rustlang.Rustup -e` in PowerShell.
  * **Option B (Manual)**: Download and run [`rustup-init.exe`](https://rustup.rs/). Select default option (`1. Proceed with installation`).

---

### 🔹 Requirement 2: Microsoft C++ Build Tools (`MSVC Linker / link.exe`)
* **What it is**: Microsoft's official C/C++ compiler toolchain and Windows SDK linker (`link.exe`).
* **Why it's needed**: 
  1. The Rust Windows target (`x86_64-pc-windows-msvc`) relies on `link.exe` to package the compiled binary into a native Windows `.exe`.
  2. The application interacts directly with Windows OS networking sockets (`ws2_32.lib`) for high-speed TCP Keep-Alive controls.
* **How to Install**:
  * Download the official **[Visual Studio Build Tools Installer](https://visualstudio.microsoft.com/visual-cpp-build-tools/)**.
  * Run `vs_BuildTools.exe` and check **Desktop development with C++**.
  * Click **Install** (requires ~1.5 GB disk space).

---

### 🔹 Requirement 3: System Configuration Files (`devices.ini` & `.env`)
* **What it is**: Active environment configuration and machine IP mapping files.
* **Why it's needed**:
  * `devices.ini`: Tells the service the IP address and `Machine_Code` for all 27 Zebra barcode readers (`192.168.1.80-110:22172`).
  * `.env`: Toggles multi-target database execution (`ENABLE_LOCAL_AUDIT`, `ENABLE_LAPTOP_B`, `ENABLE_PRODUCTION`) and safety dry-run transaction rollbacks (`DRY_RUN=true`).

---

## 2. Requirements for Deployment (Target Factory Plant PC)

Once compiled into `tread_trace_rust.exe`, **the target production machine requires ZERO runtime installation**:

| Requirement | Needed on Target Plant PC? | Reason / Explanation |
| :--- | :---: | :--- |
| **Python Runtime / Interpreter** | ❌ **NOT NEEDED** | Rust compiles to native machine code (`.exe`). |
| **ODBC Drivers / OLE DB** | ❌ **NOT NEEDED** | Uses native TDS protocol over TCP (`tiberius`) to talk directly to MS SQL Server. |
| **SQLite DLLs** | ❌ **NOT NEEDED** | SQLite engine is statically linked directly inside the `.exe` via `rusqlite` bundled feature. |
| **NSSM Service Wrapper** | 🟢 Optional | Can be registered as a Windows background service via NSSM or native `sc.exe`. |

---

## 🚀 1-Click Automated Setup Command (PowerShell)

On any fresh Windows PC, open PowerShell as Administrator and run:

```powershell
# 1. Install Rust Toolchain via Winget
winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements

# 2. Install Visual Studio C++ Build Tools
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--passive --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

Once installed, restart your terminal, navigate into `RUST Engine Trial/`, and run:

```cmd
cargo build --release
```

The output binary will be created at: `RUST Engine Trial/target/release/tread_trace_rust.exe`.

