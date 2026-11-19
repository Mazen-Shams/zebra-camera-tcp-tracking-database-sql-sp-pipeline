"""
Unified Stored Procedure Verification Utility
Tests csp_AutoProdChange_BY_TRED_CARD(@Card, @MCH_CODE, @Resault OUTPUT) against production MS SQL Server.

Usage:
  python test_sp.py [machine_code] [barcode]           -> Safe Dry-Run Rollback (Default, 0 rows saved)
  python test_sp.py [machine_code] [barcode] --live    -> Live Commit Mode (Permanently saved)

Examples:
  python test_sp.py TRGB1 SE0700G73X
  python test_sp.py C1 SE0700G73X --live
"""

import os
import sys
import pyodbc

# Ensure parent directory is in sys.path to import db_manager
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from db_manager import db_manager


def test_production_sp(machine_code: str = "TRGB1", barcode: str = "SE0700G73X", is_live: bool = False):
    db_manager.load_config()
    target = db_manager.prod_target
    server = target.server
    database = target.database
    username = target.username
    password = target.password
    driver = target.get_odbc_driver()
    sp_name = target.sp_name

    mode_label = "\033[93m[LIVE COMMIT MODE]\033[0m" if is_live else "\033[92m[DRY-RUN SAFETY MODE]\033[0m"
    safety_desc = "conn.commit() -> Permanently saved in DB" if is_live else "conn.rollback() -> 0 rows modified"

    print("=========================================================================")
    print(f" Production Stored Procedure Tester - {mode_label}")
    print(f" Target Server : {server} | Database: {database}")
    print(f" Procedure     : {sp_name}")
    print(f" Execution Mode: {safety_desc}")
    print("=========================================================================\n")

    auth_str = "Trusted_Connection=yes;" if not username or username.lower() in ['trusted', 'windows', 'sspi', ''] else f"UID={username};PWD={password};"
    conn_str = f"DRIVER={{{driver}}};SERVER={server};DATABASE={database};{auth_str}Timeout=5;TrustServerCertificate=yes;"

    print(f"[1/4] Connecting to SQL Server '{server}'...")
    try:
        conn = pyodbc.connect(conn_str)
        conn.autocommit = False
        cursor = conn.cursor()
        print(f"      \033[92m[OK] Connected successfully to '{server}' -> DB '{database}'!\033[0m\n")

        print(f"[2/4] Test Inputs:")
        print(f"      @Card (Barcode)   : '{barcode}' (len={len(barcode)})")
        print(f"      @MCH_CODE         : '{machine_code}' (len={len(machine_code)})")
        print(f"      @Resault          : OUTPUT variable\n")

        sql = f"SET NOCOUNT ON; DECLARE @res BIT; EXEC {sp_name} @Card=?, @MCH_CODE=?, @Resault=@res OUTPUT; SELECT @res AS result;"
        
        print(f"[3/4] Executing SQL: {sql}")
        cursor.execute(sql, (str(barcode)[:10], str(machine_code)[:8]))
        
        while cursor.description is None:
            if not cursor.nextset():
                break

        sp_result = None
        if cursor.description:
            row = cursor.fetchone()
            sp_result = bool(row[0]) if row and row[0] is not None else False

        print(f"      \033[94m[RESULT FROM SP] @Resault = {sp_result}\033[0m\n")

        if is_live:
            print("[4/4] Committing Transaction to Production Database...")
            conn.commit()
            print("      \033[93m[COMMITTED] Transaction permanently committed to Production DB!\033[0m\n")
        else:
            print("[4/4] Safety Check: Performing Transaction Rollback (Dry-Run Mode)...")
            conn.rollback()
            print("      \033[92m[SUCCESS] Transaction rolled back cleanly. 0 rows committed to Production DB!\033[0m\n")

        conn.close()
        print("=========================================================================")
        print(" TEST COMPLETED SUCCESSFULLY")
        print("=========================================================================\n")
        return True

    except Exception as e:
        print(f"\n\033[91m[ERROR] Execution Failed: {e}\033[0m\n")
        return False


if __name__ == "__main__":
    args = [arg for arg in sys.argv[1:] if not arg.startswith("-")]
    live_flag = "--live" in sys.argv or "-l" in sys.argv

    # Support flexible parameter ordering: test_sp.py [machine_code] [barcode] OR test_sp.py [barcode]
    if len(args) >= 2:
        m_code = args[0]
        b_code = args[1]
    elif len(args) == 1:
        m_code = "TRGB1"
        b_code = args[0]
    else:
        m_code = "TRGB1"
        b_code = "SE0700G73X"

    test_production_sp(machine_code=m_code, barcode=b_code, is_live=live_flag)

