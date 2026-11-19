"""
Production Database Execution Manager
Consolidated High-Performance Engine supporting:
1. Local CSV Audit Log (logs/sql_audit.csv)
2. Production SQL Server Target (sql-server.plant.local / csp_AutoProdChange_BY_TRED_CARD)

Features:
- Pure CSV local audit logging (Zero SQLite binary database dependencies)
- OS ODBC Driver Auto-Discovery (ODBC Driver 18/17, Native Client 11, SQL Server)
- 15-Second Offline Cooldown Circuit Breaker (prevents thread pool exhaustion if DB drops)
- Safe Transaction Rollback Mode (ENABLE_LIVE_COMMIT=false) vs. Live Commit (ENABLE_LIVE_COMMIT=true)
- Hot-Reloading .env configuration on every execution
"""

import os
import sys
import time
import csv
from datetime import datetime

try:
    import pyodbc
except ImportError:
    pyodbc = None

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
if BASE_DIR not in sys.path:
    sys.path.insert(0, BASE_DIR)

ENV_PATH = os.path.join(BASE_DIR, ".env")
LOGS_DIR = os.path.join(BASE_DIR, "logs")
CSV_PATH = os.path.join(LOGS_DIR, "sql_audit.csv")

from tools.logger import log_db_error, log_audit_trace


def parse_env_file(filepath: str) -> dict:
    """Parses .env file dynamically without caching to allow hot-reloading."""
    env_vars = {}
    if os.path.exists(filepath):
        with open(filepath, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if line and not line.startswith('#') and '=' in line:
                    key, val = line.split('=', 1)
                    val = val.split('#')[0].strip().strip('"\'')
                    env_vars[key.strip()] = val
    return env_vars


# =========================================================================
# 1. Base Database Target (Circuit Breaker & Driver Auto-Discovery)
# =========================================================================
class BaseDatabaseTarget:
    def __init__(self, target_name: str, server: str, database: str, username: str, password: str, driver: str, sp_name: str):
        self.target_name = target_name
        self.server = server
        self.database = database
        self.username = username
        self.password = password
        self.requested_driver = driver
        self.sp_name = sp_name
        self.last_failure_time = 0

    def is_in_cooldown(self, cooldown_seconds: int = 15) -> bool:
        """Returns True if the target is in a 15s offline cooldown period after a connection failure."""
        return (time.time() - self.last_failure_time) < cooldown_seconds

    def record_failure(self):
        """Trips the 15-second offline cooldown circuit."""
        self.last_failure_time = time.time()

    def get_odbc_driver(self) -> str:
        """Auto-discovers installed Windows SQL Server ODBC drivers."""
        if not pyodbc:
            return 'SQL Server'
        installed = pyodbc.drivers()
        preferred_drivers = [
            self.requested_driver,
            'ODBC Driver 18 for SQL Server',
            'ODBC Driver 17 for SQL Server',
            'SQL Server Native Client 11.0',
            'SQL Server'
        ]
        for drv in preferred_drivers:
            if drv in installed:
                return drv
        return 'SQL Server'

    def get_connection_string(self) -> str:
        """Generates pyodbc connection string supporting SQL Auth or Windows Auth."""
        driver = self.get_odbc_driver()
        if self.username and self.username.lower() not in ['trusted', 'windows', 'sspi', '']:
            auth_str = f"UID={self.username};PWD={self.password};"
        else:
            auth_str = "Trusted_Connection=yes;"

        return (
            f"DRIVER={{{driver}}};"
            f"SERVER={self.server};"
            f"DATABASE={self.database};"
            f"{auth_str}"
            f"Timeout=3;"
            f"TrustServerCertificate=yes;"
        )


# =========================================================================
# 2. Local CSV Audit Target (Pure CSV File Logging)
# =========================================================================
class LocalAuditTarget:
    def __init__(self):
        self.target_name = "Local CSV Audit"
        self.csv_path = CSV_PATH
        self.init_csv_log()

    def init_csv_log(self):
        try:
            os.makedirs(LOGS_DIR, exist_ok=True)
            if not os.path.exists(self.csv_path):
                with open(self.csv_path, 'w', newline='', encoding='utf-8') as f:
                    writer = csv.writer(f)
                    writer.writerow(['Timestamp', 'Machine_Code', 'Barcode', 'Stored_Procedure', 'Status'])
        except Exception:
            pass

    def execute(self, machine_code: str, barcode: str, now_str: str) -> tuple[bool, str]:
        try:
            os.makedirs(LOGS_DIR, exist_ok=True)
            with open(self.csv_path, 'a', newline='', encoding='utf-8') as f:
                writer = csv.writer(f)
                writer.writerow([now_str, machine_code, barcode, "csp_AutoProdChange_BY_TRED_CARD", "AUDIT_LOGGED"])

            return True, f"[{self.target_name}] CSV_SAVED"
        except Exception as e:
            return False, f"[{self.target_name}] Error: {str(e)}"


# =========================================================================
# 3. Factory Production SQL Server Target
# =========================================================================
class ProductionTarget(BaseDatabaseTarget):
    def __init__(self, server: str, database: str, username: str, password: str, driver: str, sp_name: str):
        super().__init__(
            target_name="Prod Server (sql-server.plant.local)",
            server=server,
            database=database,
            username=username,
            password=password,
            driver=driver,
            sp_name=sp_name or "csp_AutoProdChange_BY_TRED_CARD"
        )

    def execute(self, machine_code: str, barcode: str, now_str: str, enable_live_commit: bool) -> tuple[bool, str]:
        if self.is_in_cooldown():
            return False, f"[{self.target_name}] Cooldown active (Offline)"

        if not pyodbc:
            return False, f"[{self.target_name}] pyodbc library missing"

        conn_str = self.get_connection_string()
        try:
            conn = pyodbc.connect(conn_str)
            conn.autocommit = False
            cursor = conn.cursor()

            # Execute Production SP (csp_AutoProdChange_BY_TRED_CARD @Card=?, @MCH_CODE=?, @Resault=@res OUTPUT)
            sql = f"SET NOCOUNT ON; DECLARE @res BIT; EXEC {self.sp_name} @Card=?, @MCH_CODE=?, @Resault=@res OUTPUT; SELECT @res AS result;"
            cursor.execute(sql, (str(barcode)[:10], str(machine_code)[:8]))
            
            while cursor.description is None:
                if not cursor.nextset():
                    break

            sp_result = None
            if cursor.description:
                row = cursor.fetchone()
                sp_result = bool(row[0]) if row and row[0] is not None else False

            if enable_live_commit:
                conn.commit()
                msg = f"[{self.target_name}] COMMITTED OK ({self.sp_name} -> Result: {sp_result})"
            else:
                conn.rollback()
                msg = f"[{self.target_name}] OK (Dry-Run Rollback - {self.sp_name} -> Result: {sp_result})"

            conn.close()
            return True, msg

        except Exception as e:
            self.record_failure()
            full_error = str(e).replace('\r', ' ').replace('\n', ' ').strip()
            return False, f"[{self.target_name}] Error: {full_error}"


# =========================================================================
# 4. Master Database Manager (Hot-Reload Pipeline)
# =========================================================================
class DatabaseManager:
    def __init__(self):
        self.load_config()

    def load_config(self):
        env_vars = parse_env_file(ENV_PATH)
        
        self.enable_local = env_vars.get('ENABLE_LOCAL_AUDIT', 'true').lower() in ['true', '1', 'yes']
        self.enable_production = env_vars.get('ENABLE_PRODUCTION', 'false').lower() in ['true', '1', 'yes']

        if 'ENABLE_LIVE_COMMIT' in env_vars:
            self.enable_live_commit = env_vars.get('ENABLE_LIVE_COMMIT', 'true').lower() in ['true', '1', 'yes']
        else:
            dry_run_val = env_vars.get('DRY_RUN', 'false').lower()
            self.enable_live_commit = dry_run_val not in ['true', '1', 'yes']

        self.local_target = LocalAuditTarget()

        self.prod_target = ProductionTarget(
            server=env_vars.get('PROD_SERVER', 'sql-server.plant.local'),
            database=env_vars.get('PROD_NAME', 'PCS'),
            username=env_vars.get('PROD_USER', 'YOUR_DB_USER'),
            password=env_vars.get('PROD_PASSWORD', 'YOUR_DB_USER'),
            driver=env_vars.get('PROD_DRIVER', 'ODBC Driver 17 for SQL Server'),
            sp_name=env_vars.get('PROD_SP_NAME', 'csp_AutoProdChange_BY_TRED_CARD')
        )

    def execute_stored_procedure(self, machine_code: str, barcode: str) -> tuple[bool, str]:
        """Executes across all enabled targets in .env concurrently."""
        self.load_config()
        now_str = datetime.now().strftime('%Y-%m-%d %H:%M:%S')
        results = []
        overall_success = True

        # 1. Target 1: Local CSV Audit Log
        if self.enable_local:
            ok, msg = self.local_target.execute(machine_code, barcode, now_str)
            results.append(msg)
            if not ok:
                log_db_error(msg)
                overall_success = False

        # 2. Target 2: Factory Production Server
        if self.enable_production:
            ok, msg = self.prod_target.execute(machine_code, barcode, now_str, self.enable_live_commit)
            results.append(msg)
            if not ok:
                log_db_error(msg)
                overall_success = False

        if not results:
            results.append("No targets enabled in .env")

        combined_msg = " | ".join(results)
        log_audit_trace(machine_code, barcode, combined_msg)
        return overall_success, combined_msg


db_manager = DatabaseManager()

