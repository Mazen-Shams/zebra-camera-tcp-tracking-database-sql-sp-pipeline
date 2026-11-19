"""
Interactive Command-Line SQL Terminal for SQL Server (sqlcmd alternative via pyodbc)
Run queries, inspect tables, and execute stored procedures directly from terminal!
"""

import sys
import os
import pyodbc

# Ensure parent directory is in sys.path to import db_manager
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from db_manager import db_manager

def run_sql_cli(server=None, database=None, query=None):
    db_manager.load_config()
    target = db_manager.prod_target
    server = server or target.server
    database = database or target.database
    username = target.username
    password = target.password
    driver = target.get_odbc_driver()

    print("=========================================================================")
    print(" SQL Server Terminal CLI (pyodbc Engine)")
    print(f" Server: '{server}' | Database: '{database}' | Driver: '{driver}'")
    print("=========================================================================\n")

    auth_str = "Trusted_Connection=yes;" if not username or username.lower() in ['trusted', 'windows', 'sspi', ''] else f"UID={username};PWD={password};"
    conn_str = (
        f"DRIVER={{{driver}}};"
        f"SERVER={server};"
        f"DATABASE={database};"
        f"{auth_str}"
        f"Timeout=5;"
        f"TrustServerCertificate=yes;"
    )

    try:
        conn = pyodbc.connect(conn_str)
        print("[OK] Connected to SQL Server! Type 'exit' or 'quit' to close.\n")
        cursor = conn.cursor()

        if query:
            execute_query(cursor, query)
            conn.close()
            return

        while True:
            try:
                user_q = input("sql> ").strip()
                if not user_q:
                    continue
                if user_q.lower() in ['exit', 'quit']:
                    print("Exiting SQL CLI tool.")
                    break
                execute_query(cursor, user_q)
            except KeyboardInterrupt:
                print("\nExiting SQL CLI tool.")
                break
        conn.close()

    except Exception as e:
        print(f"[ERROR] Connection Error: {e}\n")

def execute_query(cursor, query):
    try:
        cursor.execute(query)
        if cursor.description:
            columns = [column[0] for column in cursor.description]
            rows = cursor.fetchall()
            print(" | ".join(columns))
            print("-" * 75)
            for row in rows:
                print(" | ".join(str(val) for val in row))
            print(f"\n({len(rows)} row(s) returned)\n")
        else:
            cursor.commit()
            print(f"[OK] Query executed successfully. ({cursor.rowcount} row(s) affected)\n")
    except Exception as e:
        print(f"[SQL ERROR] {e}\n")

if __name__ == "__main__":
    target = sys.argv[1] if len(sys.argv) > 1 and not sys.argv[1].upper().startswith("SELECT") else None
    q = sys.argv[2] if len(sys.argv) > 2 else (sys.argv[1] if len(sys.argv) > 1 and sys.argv[1].upper().startswith("SELECT") else None)
    run_sql_cli(server=target, query=q)

