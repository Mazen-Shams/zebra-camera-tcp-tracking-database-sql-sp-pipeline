use crate::config::{load_app_config, AppConfig, DbTargetConfig};
use chrono::Local;
use odbc_api::{ConnectionOptions, Environment};
use rusqlite::{params, Connection as SqliteConnection};
use std::collections::HashMap;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct DatabaseManager {
    config: AppConfig,
    cooldowns: Arc<Mutex<HashMap<String, Instant>>>,
    env: Arc<Environment>,
}

impl DatabaseManager {
    pub fn new(config: AppConfig) -> Self {
        // Ensure logs directory exists
        if let Some(parent) = Path::new(&config.csv_path).parent() {
            let _ = create_dir_all(parent);
        }

        // Initialize SQLite local audit table
        if config.enable_local {
            if let Ok(conn) = SqliteConnection::open(&config.sqlite_path) {
                let _ = conn.execute(
                    "CREATE TABLE IF NOT EXISTS PCS_Tread_Traceability (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        timestamp TEXT,
                        machine_code TEXT,
                        barcode TEXT,
                        stored_procedure TEXT,
                        status TEXT
                    )",
                    [],
                );
            }
        }

        // Initialize CSV audit log header
        if !Path::new(&config.csv_path).exists() {
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&config.csv_path)
            {
                let _ = writeln!(
                    file,
                    "Timestamp,Machine_Code,Barcode,Stored_Procedure,Status,Mode"
                );
            }
        }

        let env = Arc::new(Environment::new().expect("Failed to initialize ODBC Environment"));

        Self {
            config,
            cooldowns: Arc::new(Mutex::new(HashMap::new())),
            env,
        }
    }

    pub async fn execute_stored_procedure(
        &self,
        machine_code: &str,
        barcode: &str,
    ) -> (bool, String) {
        let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let current_config = load_app_config();
        let mut results = Vec::new();
        let mut overall_success = true;

        // 1. Target 1: Local Audit
        if current_config.enable_local {
            self.write_sqlite(&now_str, machine_code, barcode, "LOCAL_SAVED");
            self.write_csv(
                &now_str,
                machine_code,
                barcode,
                "DB_Excec_Tread_Barcode",
                "LOCAL_SAVED",
                "LOCAL_AUDIT",
            );
            results.push("[Local Audit] SAVED".to_string());
        }

        // 2. Target 2: Laptop B
        if current_config.enable_laptop_b {
            let (ok, msg) = self
                .execute_sql_target_blocking(
                    &current_config.laptop_b,
                    current_config.dry_run,
                    &current_config.csv_path,
                    machine_code,
                    barcode,
                    &now_str,
                )
                .await;
            results.push(msg.clone());
            if !ok {
                overall_success = false;
                crate::logger::log_db_error(&msg);
            }
        }

        // 3. Target 3: Production Server
        if current_config.enable_production {
            let (ok, msg) = self
                .execute_sql_target_blocking(
                    &current_config.prod,
                    current_config.dry_run,
                    &current_config.csv_path,
                    machine_code,
                    barcode,
                    &now_str,
                )
                .await;
            results.push(msg.clone());
            if !ok {
                overall_success = false;
                crate::logger::log_db_error(&msg);
            }
        }

        if results.is_empty() {
            results.push("No targets enabled in .env".to_string());
        }

        let summary = results.join(" | ");
        crate::logger::log_audit_trace(&format!("MACHINE: {} | BARCODE: {} | STATUS: {}", machine_code, barcode, summary));

        (overall_success, summary)
    }

    async fn execute_sql_target_blocking(
        &self,
        target: &DbTargetConfig,
        dry_run: bool,
        csv_path: &str,
        machine_code: &str,
        barcode: &str,
        now_str: &str,
    ) -> (bool, String) {
        let target_key = target.server.clone();

        // 15-second offline cooldown check
        {
            let cooldowns = self.cooldowns.lock().await;
            if let Some(last_fail) = cooldowns.get(&target_key) {
                if last_fail.elapsed() < Duration::from_secs(15) {
                    let msg = format!("[{}] Cooldown active (Offline)", target.target_name);
                    return (false, msg);
                }
            }
        }

        let env = self.env.clone();
        let target_cfg = target.clone();
        let machine_code = machine_code.to_string();
        let barcode = barcode.to_string();
        let now_str = now_str.to_string();
        let csv_path = csv_path.to_string();

        // Run ODBC connection and execution in blocking task thread pool
        let res = tokio::task::spawn_blocking(move || {
            let conn_str = format!(
                "DRIVER={{{}}};SERVER={};DATABASE={};UID={};PWD={};TrustServerCertificate=yes;",
                target_cfg.driver,
                target_cfg.server,
                target_cfg.database,
                target_cfg.username,
                target_cfg.password
            );

            let conn_options = ConnectionOptions::default();

            match env.connect_with_connection_string(&conn_str, conn_options) {
                Ok(conn) => {
                    let sp_query = format!(
                        "EXEC {} @Machine_Code = '{}', @Barcode = '{}'",
                        target_cfg.sp_name, machine_code, barcode
                    );

                    if dry_run {
                        let _ = conn.execute("BEGIN TRANSACTION", (), None);
                        let exec_res = conn.execute(&sp_query, (), None);
                        let _ = conn.rollback();

                        match exec_res {
                            Ok(_) => {
                                let msg = format!("[{}] OK (Dry-Run Rollback)", target_cfg.target_name);
                                Self::write_csv_file(&csv_path, &now_str, &machine_code, &barcode, &target_cfg.sp_name, &msg, "DRY-RUN");
                                (true, msg)
                            }
                            Err(e) => {
                                let err_msg = format!("[{}] SP Error: {}", target_cfg.target_name, e);
                                Self::write_csv_file(&csv_path, &now_str, &machine_code, &barcode, &target_cfg.sp_name, &err_msg, "OFFLINE FALLBACK");
                                (false, err_msg)
                            }
                        }
                    } else {
                        match conn.execute(&sp_query, (), None) {
                            Ok(_) => {
                                let _ = conn.commit();
                                let msg = format!("[{}] COMMITTED OK", target_cfg.target_name);
                                Self::write_csv_file(&csv_path, &now_str, &machine_code, &barcode, &target_cfg.sp_name, &msg, "LIVE");
                                (true, msg)
                            }
                            Err(e) => {
                                let err_msg = format!("[{}] SP Error: {}", target_cfg.target_name, e);
                                Self::write_csv_file(&csv_path, &now_str, &machine_code, &barcode, &target_cfg.sp_name, &err_msg, "OFFLINE FALLBACK");
                                (false, err_msg)
                            }
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("[{}] Connection Error: {}", target_cfg.target_name, e);
                    Self::write_csv_file(&csv_path, &now_str, &machine_code, &barcode, &target_cfg.sp_name, &err_msg, "OFFLINE FALLBACK");
                    (false, err_msg)
                }
            }
        })
        .await;

        match res {
            Ok((ok, msg)) => {
                if !ok {
                    self.record_failure(&target_key).await;
                }
                (ok, msg)
            }
            Err(e) => {
                self.record_failure(&target_key).await;
                (false, format!("[{}] Execution Task Join Error: {}", target.target_name, e))
            }
        }
    }

    async fn record_failure(&self, target_key: &str) {
        let mut cooldowns = self.cooldowns.lock().await;
        cooldowns.insert(target_key.to_string(), Instant::now());
    }

    fn write_sqlite(&self, timestamp: &str, machine_code: &str, barcode: &str, status: &str) {
        if let Ok(conn) = SqliteConnection::open(&self.config.sqlite_path) {
            let _ = conn.execute(
                "INSERT INTO PCS_Tread_Traceability (timestamp, machine_code, barcode, stored_procedure, status)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![timestamp, machine_code, barcode, "DB_Excec_Tread_Barcode", status],
            );
        }
    }

    fn write_csv(
        &self,
        timestamp: &str,
        machine_code: &str,
        barcode: &str,
        sp_name: &str,
        status: &str,
        mode: &str,
    ) {
        Self::write_csv_file(&self.config.csv_path, timestamp, machine_code, barcode, sp_name, status, mode);
    }

    fn write_csv_file(
        path: &str,
        timestamp: &str,
        machine_code: &str,
        barcode: &str,
        sp_name: &str,
        status: &str,
        mode: &str,
    ) {
        if let Ok(mut file) = OpenOptions::new().append(true).open(path) {
            let _ = writeln!(
                file,
                "{},{},{},{},{},{}",
                timestamp, machine_code, barcode, sp_name, status, mode
            );
        }
    }
}

