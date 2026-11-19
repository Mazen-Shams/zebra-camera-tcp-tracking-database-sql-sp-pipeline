use odbc_api::{ConnectionOptions, Environment};
use std::env;

fn main() {
    println!("=========================================================================");
    println!("  Standalone Rust Stored Procedure Execution Test Utility");
    println!("=========================================================================\n");

    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::dotenv();

    let args: Vec<String> = env::args().skip(1).collect();

    let use_prod = args.iter().any(|a| a == "--prod");
    let machine_code = args
        .get(0)
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "TRGB1".to_string());
    let barcode = args
        .get(1)
        .filter(|a| !a.starts_with('-'))
        .cloned()
        .unwrap_or_else(|| "RUST_CLI_TEST_VERIFY_9900".to_string());

    let (server, db, user, pass, driver, sp_name) = if use_prod {
        (
            env::var("PROD_SERVER").unwrap_or_else(|_| "sql-server.plant.local".into()),
            env::var("PROD_NAME").unwrap_or_else(|_| "PCS".into()),
            env::var("PROD_USER").unwrap_or_else(|_| "sa".into()),
            env::var("PROD_PASSWORD").unwrap_or_else(|_| "secret".into()),
            env::var("PROD_DRIVER").unwrap_or_else(|_| "ODBC Driver 17 for SQL Server".into()),
            env::var("PROD_SP_NAME").unwrap_or_else(|_| "DB_Excec_Tread_Barcode".into()),
        )
    } else {
        (
            env::var("LAPTOP_B_SERVER").unwrap_or_else(|_| r"192.168.1.134\SQLEXPRESS".into()),
            env::var("LAPTOP_B_NAME").unwrap_or_else(|_| "PCS".into()),
            env::var("LAPTOP_B_USER").unwrap_or_else(|_| "sa".into()),
            env::var("LAPTOP_B_PASSWORD").unwrap_or_else(|_| "secretpassword123".into()),
            env::var("LAPTOP_B_DRIVER").unwrap_or_else(|_| "ODBC Driver 18 for SQL Server".into()),
            env::var("LAPTOP_B_SP_NAME").unwrap_or_else(|_| "DB_Excec_Tread_Barcode".into()),
        )
    };

    let live_commit = match env::var("ENABLE_LIVE_COMMIT") {
        Ok(val) => val.to_lowercase() == "true",
        Err(_) => env::var("DRY_RUN").unwrap_or_else(|_| "true".into()).to_lowercase() == "false",
    };

    println!("[CONFIG] Target Server: {}", server);
    println!("[CONFIG] Database:      {}", db);
    println!("[CONFIG] Driver:        {}", driver);
    println!("[CONFIG] SP Name:       {}", sp_name);
    println!("[CONFIG] Machine Code:  {}", machine_code);
    println!("[CONFIG] Barcode:       {}", barcode);
    println!("[CONFIG] Commit Mode:   {}", if live_commit { "LIVE COMMIT (Permanent Insert)" } else { "SAFETY DRY-RUN (Transaction Rollback)" });
    println!("\n[ODBC] Connecting to SQL Server...");

    let env_odbc = Environment::new().expect("Failed to initialize ODBC Environment");
    let conn_str = format!(
        "DRIVER={{{}}};SERVER={};DATABASE={};UID={};PWD={};TrustServerCertificate=yes;",
        driver, server, db, user, pass
    );

    let res = env_odbc.connect_with_connection_string(&conn_str, ConnectionOptions::default());
    match res {
        Ok(conn) => {
            println!("\x1b[92m[SUCCESS] ODBC Connection Established!\x1b[0m");
            let query = format!(
                "EXEC {} @Machine_Code = '{}', @Barcode = '{}'",
                sp_name, machine_code, barcode
            );
            println!("[ODBC] Executing query: {}", query);

            if !live_commit {
                let _ = conn.execute("BEGIN TRANSACTION", (), None);
                match conn.execute(&query, (), None) {
                    Ok(_) => {
                        let _ = conn.rollback();
                        println!("\x1b[92m[SUCCESS] SP Executed OK! (Dry-Run Rollback - 0 rows saved)\x1b[0m");
                    }
                    Err(e) => {
                        let _ = conn.rollback();
                        println!("\x1b[91m[ERROR] Stored Procedure execution failed: {}\x1b[0m", e);
                    }
                }
            } else {
                match conn.execute(&query, (), None) {
                    Ok(_) => {
                        let _ = conn.commit();
                        println!("\x1b[92m[SUCCESS] Stored Procedure executed and COMMITTED successfully!\x1b[0m");
                    }
                    Err(e) => {
                        println!("\x1b[91m[ERROR] Stored Procedure execution failed: {}\x1b[0m", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("\x1b[91m[ERROR] ODBC Connection failed: {}\x1b[0m", e);
        }
    }
}

