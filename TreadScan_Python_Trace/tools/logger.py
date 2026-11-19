"""
Categorized High-Performance System Logging Module
Generates clean, timestamped category .log files in logs/:
- logs/db_errors.log        -> Stored procedure & SQL database errors
- logs/camera_tcp_errors.log -> TCP socket timeouts & camera disconnection events
- logs/audit_trace.log      -> Real-time barcode scan audit log (matches Web UI)
- logs/system.log           -> Service startup, shutdown & general events

Includes automatic 10 MB file rotation (.old) to protect disk space 24/7.
"""

import os
from datetime import datetime

BASE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
LOGS_DIR = os.path.join(BASE_DIR, 'logs')

DB_LOG_PATH = os.path.join(LOGS_DIR, 'db_errors.log')
CAMERA_LOG_PATH = os.path.join(LOGS_DIR, 'camera_tcp_errors.log')
AUDIT_LOG_PATH = os.path.join(LOGS_DIR, 'audit_trace.log')
SYSTEM_LOG_PATH = os.path.join(LOGS_DIR, 'system.log')

MAX_LOG_BYTES = 10 * 1024 * 1024  # 10 MB max per log file before automatic rotation

def _rotate_if_needed(filepath: str):
    """Rotates log file to .old if it exceeds 10 MB to prevent disk bloat."""
    if os.path.exists(filepath) and os.path.getsize(filepath) > MAX_LOG_BYTES:
        backup_path = filepath + ".old"
        try:
            if os.path.exists(backup_path):
                os.remove(backup_path)
            os.rename(filepath, backup_path)
        except Exception:
            pass

def _write_log(filepath: str, text: str):
    try:
        os.makedirs(LOGS_DIR, exist_ok=True)
        _rotate_if_needed(filepath)
        now_str = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        with open(filepath, 'a', encoding='utf-8') as f:
            f.write(f"[{now_str}] {text}\n")
    except Exception:
        pass

def log_db_error(message: str):
    """Logs database connection failures and stored procedure execution errors."""
    _write_log(DB_LOG_PATH, f"[DB ERROR] {message}")

def log_camera_error(machine_code: str, ip: str, message: str):
    """Logs camera network dropouts, socket timeouts, and reconnection attempts."""
    _write_log(CAMERA_LOG_PATH, f"[CAMERA ERROR] Machine: {machine_code} ({ip}) -> {message}")

def log_audit_trace(machine_code: str, barcode: str, db_status: str):
    """Logs full real-time scan audit trace matching the Web UI feed."""
    _write_log(AUDIT_LOG_PATH, f"[SCAN AUDIT] Machine: {machine_code} | Barcode: {barcode} | Status: {db_status}")

def log_system_event(message: str):
    """Logs general service lifecycle events (startup, shutdown, config reloads)."""
    _write_log(SYSTEM_LOG_PATH, f"[SYSTEM] {message}")

def ensure_log_headers():
    """Ensures logs/ directory exists."""
    os.makedirs(LOGS_DIR, exist_ok=True)

