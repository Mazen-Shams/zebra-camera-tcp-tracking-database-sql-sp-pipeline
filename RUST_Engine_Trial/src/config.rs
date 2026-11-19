use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub machine_code: String,
    pub ip: String,
}

#[derive(Debug, Clone)]
pub struct DbTargetConfig {
    pub target_name: String,
    pub server: String,
    pub database: String,
    pub username: String,
    pub password: String,
    pub driver: String,
    pub sp_name: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub enable_local: bool,
    pub enable_laptop_b: bool,
    pub enable_production: bool,
    pub dry_run: bool,
    pub laptop_b: DbTargetConfig,
    pub prod: DbTargetConfig,
    pub sqlite_path: String,
    pub csv_path: String,
}

pub fn find_devices_ini_path() -> Option<PathBuf> {
    let mut possible_paths: Vec<PathBuf> = vec![
        PathBuf::from("devices.ini"),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            possible_paths.push(exe_dir.join("devices.ini"));
        }
    }

    for p in possible_paths {
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub fn load_devices_config() -> Vec<DeviceConfig> {
    let mut devices = Vec::new();
    let found_path = find_devices_ini_path();

    if let Some(ini_path) = found_path {
        if let Ok(content) = fs::read_to_string(ini_path) {
            let mut current_section = String::new();
            let mut current_code = String::new();
            let mut current_ip = String::new();

            for line in content.lines() {
                let line = line.trim();
                if line.starts_with('[') && line.ends_with(']') {
                    if !current_code.is_empty() && !current_ip.is_empty() {
                        devices.push(DeviceConfig {
                            machine_code: current_code.clone(),
                            ip: current_ip.clone(),
                        });
                        current_code.clear();
                        current_ip.clear();
                    }
                    current_section = line[1..line.len() - 1].to_string();
                } else if line.contains('=') && current_section.to_lowercase().starts_with("device") {
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim();
                        let val = parts[1].trim().trim_matches(|c| c == '"' || c == '\'' || c == ' ');
                        if key.eq_ignore_ascii_case("Machine_Code") {
                            current_code = val.to_string();
                        } else if key.eq_ignore_ascii_case("IP_Address") {
                            current_ip = val.to_string();
                        }
                    }
                }
            }

            if !current_code.is_empty() && !current_ip.is_empty() {
                devices.push(DeviceConfig {
                    machine_code: current_code,
                    ip: current_ip,
                });
            }
        }
    }

    devices
}

pub fn load_app_config() -> AppConfig {
    let _ = dotenvy::from_filename(".env");
    let _ = dotenvy::dotenv();

    let enable_local = std::env::var("ENABLE_LOCAL_AUDIT")
        .unwrap_or_else(|_| "false".into())
        .to_lowercase()
        == "true";
    let enable_laptop_b = std::env::var("ENABLE_LAPTOP_B")
        .unwrap_or_else(|_| "false".into())
        .to_lowercase()
        == "true";
    let enable_production = std::env::var("ENABLE_PRODUCTION")
        .unwrap_or_else(|_| "false".into())
        .to_lowercase()
        == "true";

    // Support ENABLE_LIVE_COMMIT (Python standard: true = live, false = dry run)
    let dry_run = match std::env::var("ENABLE_LIVE_COMMIT") {
        Ok(val) => val.to_lowercase() != "true",
        Err(_) => std::env::var("DRY_RUN")
            .unwrap_or_else(|_| "true".into())
            .to_lowercase()
            == "true",
    };

    let laptop_b = DbTargetConfig {
        target_name: "Laptop B".into(),
        server: std::env::var("LAPTOP_B_SERVER").unwrap_or_else(|_| r"192.168.1.134\SQLEXPRESS".into()),
        database: std::env::var("LAPTOP_B_NAME").unwrap_or_else(|_| "PCS".into()),
        username: std::env::var("LAPTOP_B_USER").unwrap_or_else(|_| "sa".into()),
        password: std::env::var("LAPTOP_B_PASSWORD").unwrap_or_else(|_| "secretpassword123".into()),
        driver: std::env::var("LAPTOP_B_DRIVER").unwrap_or_else(|_| "ODBC Driver 18 for SQL Server".into()),
        sp_name: std::env::var("LAPTOP_B_SP_NAME").unwrap_or_else(|_| "DB_Excec_Tread_Barcode".into()),
    };

    let prod = DbTargetConfig {
        target_name: "Prod Server".into(),
        server: std::env::var("PROD_SERVER").unwrap_or_else(|_| "sql-server.plant.local".into()),
        database: std::env::var("PROD_NAME").unwrap_or_else(|_| "PCS".into()),
        username: std::env::var("PROD_USER").unwrap_or_else(|_| "sa".into()),
        password: std::env::var("PROD_PASSWORD").unwrap_or_else(|_| "secret".into()),
        driver: std::env::var("PROD_DRIVER").unwrap_or_else(|_| "ODBC Driver 17 for SQL Server".into()),
        sp_name: std::env::var("PROD_SP_NAME").unwrap_or_else(|_| "DB_Excec_Tread_Barcode".into()),
    };

    AppConfig {
        enable_local,
        enable_laptop_b,
        enable_production,
        dry_run,
        laptop_b,
        prod,
        sqlite_path: std::env::var("TEST_SQLITE_PATH").unwrap_or_else(|_| "simulated_pcs.db".into()),
        csv_path: std::env::var("TEST_CSV_AUDIT").unwrap_or_else(|_| "logs/sql_audit.csv".into()),
    }
}

