mod config;
mod db;
mod listener;
mod logger;
mod types;
mod web;

use config::{find_devices_ini_path, load_app_config, load_devices_config, DeviceConfig};
use db::DatabaseManager;
use listener::listen_to_device;
use types::{AppStateData, BarcodeItem, SharedAppState, WsMessage};
use web::create_web_router;

use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task::JoinHandle;

const DEFAULT_PORT: u16 = 22172;

async fn watch_devices_config(
    shared_state: SharedAppState,
    tx_barcode: mpsc::Sender<BarcodeItem>,
    cli_override: bool,
    initial_devices: Vec<DeviceConfig>,
) {
    let mut active_tasks: HashMap<String, JoinHandle<()>> = HashMap::new();
    let mut last_mtime: Option<SystemTime> = None;

    let spawn_device = |dev: &DeviceConfig,
                        tasks: &mut HashMap<String, JoinHandle<()>>,
                        state: &SharedAppState,
                        tx: &mpsc::Sender<BarcodeItem>| {
        let machine_code = dev.machine_code.clone();
        let host = dev.ip.clone();
        let shared_state_listener = state.clone();
        let tx_barcode_listener = tx.clone();

        let handle = tokio::spawn(async move {
            listen_to_device(
                machine_code,
                host,
                DEFAULT_PORT,
                shared_state_listener,
                tx_barcode_listener,
            )
            .await;
        });
        tasks.insert(dev.machine_code.clone(), handle);
    };

    for dev in &initial_devices {
        spawn_device(dev, &mut active_tasks, &shared_state, &tx_barcode);
    }

    if cli_override {
        println!("[CONFIG] CLI IP override active. Config file hot-reloading disabled.");
        return;
    }

    let ini_path = match find_devices_ini_path() {
        Some(p) => p,
        None => return,
    };

    if let Ok(meta) = std::fs::metadata(&ini_path) {
        if let Ok(mtime) = meta.modified() {
            last_mtime = Some(mtime);
        }
    }

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        if let Ok(meta) = std::fs::metadata(&ini_path) {
            if let Ok(mtime) = meta.modified() {
                if last_mtime != Some(mtime) {
                    last_mtime = Some(mtime);

                    let current_devices = load_devices_config();
                    let current_codes: HashSet<String> =
                        current_devices.iter().map(|d| d.machine_code.clone()).collect();

                    let mut updated_count = 0;

                    {
                        let mut state = shared_state.state.write().await;

                        for d in &current_devices {
                            let code = &d.machine_code;
                            let ip = &d.ip;

                            let existing_ip = state.devices.get(code).map(|s| s.ip.clone());
                            let ip_changed = existing_ip.as_ref() != Some(ip);
                            let not_running = !active_tasks.contains_key(code);

                            if ip_changed || not_running {
                                if let Some(handle) = active_tasks.remove(code) {
                                    handle.abort();
                                }

                                state.devices.insert(
                                    code.clone(),
                                    types::DeviceState {
                                        ip: ip.clone(),
                                        status: "INIT".into(),
                                        keepalive: false,
                                        retry: 0,
                                        last_barcode: "N/A".into(),
                                        last_time: "N/A".into(),
                                    },
                                );

                                spawn_device(d, &mut active_tasks, &shared_state, &tx_barcode);
                                updated_count += 1;
                            }
                        }

                        let removed_codes: Vec<String> = active_tasks
                            .keys()
                            .filter(|k| !current_codes.contains(*k))
                            .cloned()
                            .collect();

                        for code in removed_codes {
                            if let Some(handle) = active_tasks.remove(&code) {
                                handle.abort();
                            }
                            state.devices.remove(&code);
                            updated_count += 1;
                        }

                        if updated_count > 0 {
                            let snapshot = state.get_snapshot();
                            let _ = shared_state.tx.send(snapshot);
                            println!(
                                "\x1b[92m[CONFIG HOT-RELOAD] Reloaded devices.ini - {} device target(s) updated.\x1b[0m",
                                updated_count
                            );
                            logger::log_system(&format!(
                                "[CONFIG HOT-RELOAD] Reloaded devices.ini - {} device target(s) updated.",
                                updated_count
                            ));
                        }
                    }
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=========================================================================");
    println!("  Production-Grade Rust Multi-Camera Async Barcode Listener Service");
    println!("=========================================================================");

    logger::log_system("RUST Engine Service starting up...");

    let app_config = load_app_config();
    let db_manager = Arc::new(DatabaseManager::new(app_config));

    let mut target_devices = load_devices_config();

    // Check CLI arguments for custom IP overrides or --verbose / -v flag
    let args: Vec<String> = env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "--verbose" || a == "-v");

    let mut cli_ips = Vec::new();
    for arg in &args {
        if !arg.starts_with('-') {
            cli_ips.push(arg.trim().to_string());
        }
    }

    if !cli_ips.is_empty() {
        let mut cli_devices = Vec::new();
        for (idx, ip) in cli_ips.iter().enumerate() {
            if !ip.is_empty() {
                if let Some(matched) = target_devices.iter().find(|d| d.ip == *ip) {
                    cli_devices.push(matched.clone());
                } else {
                    cli_devices.push(DeviceConfig {
                        machine_code: format!("TEST{}", idx + 1),
                        ip: ip.clone(),
                    });
                }
            }
        }
        if !cli_devices.is_empty() {
            target_devices = cli_devices;
        }
    }

    if target_devices.is_empty() {
        let err_msg = "No valid device IP addresses found in devices.ini or CLI arguments!";
        println!("\x1b[91m[ERROR] {}\x1b[0m", err_msg);
        logger::log_system(&format!("ERROR: {}", err_msg));
        return Ok(());
    }

    logger::log_system(&format!("Loaded {} camera devices into worker pool.", target_devices.len()));

    // Initialize Shared App State
    let mut initial_devices = HashMap::new();
    for dev in &target_devices {
        initial_devices.insert(
            dev.machine_code.clone(),
            types::DeviceState {
                ip: dev.ip.clone(),
                status: "INIT".into(),
                keepalive: false,
                retry: 0,
                last_barcode: "N/A".into(),
                last_time: "N/A".into(),
            },
        );
    }

    let (tx_ws, _) = broadcast::channel::<WsMessage>(100);
    let app_state_data = AppStateData {
        devices: initial_devices,
        total_reads: 0,
        good_reads: 0,
        bad_reads: 0,
        db_success: 0,
    };

    let shared_state = SharedAppState {
        state: Arc::new(RwLock::new(app_state_data)),
        tx: tx_ws.clone(),
    };

    let (tx_barcode, mut rx_barcode) = mpsc::channel::<BarcodeItem>(500);

    // 1. Spawn Barcode Consumer Task
    let shared_state_consumer = shared_state.clone();
    let db_manager_consumer = db_manager.clone();
    tokio::spawn(async move {
        while let Some(item) = rx_barcode.recv().await {
            if verbose {
                println!("[SCAN READ] Machine: {:<6} | Host: {:<15} | Barcode: {}", item.machine_code, item.host, item.barcode);
            }

            let db_msg: String;
            {
                let mut app_state = shared_state_consumer.state.write().await;
                app_state.total_reads += 1;

                if let Some(dev) = app_state.devices.get_mut(&item.machine_code) {
                    dev.last_barcode = item.barcode.clone();
                    dev.last_time = item.timestamp.clone();
                }

                let is_bad_read = matches!(
                    item.barcode.to_uppercase().as_str(),
                    "NOREAD" | "BAD_READ" | "FAIL"
                );

                if is_bad_read {
                    app_state.bad_reads += 1;
                    db_msg = "BAD READ (DB SKIPPED)".into();
                } else {
                    app_state.good_reads += 1;
                    db_msg = "DB EXECUTION QUEUED".into();
                }

                let scan_event = WsMessage::ScanEvent {
                    timestamp: item.timestamp.clone(),
                    machine_code: item.machine_code.clone(),
                    host: item.host.clone(),
                    barcode: item.barcode.clone(),
                    db_status: db_msg.clone(),
                };

                let _ = shared_state_consumer.tx.send(scan_event);
                let _ = shared_state_consumer.tx.send(app_state.get_snapshot());
            }

            // Execute DB Stored Procedure in background if good read
            if !matches!(
                item.barcode.to_uppercase().as_str(),
                "NOREAD" | "BAD_READ" | "FAIL"
            ) {
                let db_ref = db_manager_consumer.clone();
                let state_ref = shared_state_consumer.clone();
                let m_code = item.machine_code.clone();
                let b_code = item.barcode.clone();

                tokio::spawn(async move {
                    let (ok, _) = db_ref.execute_stored_procedure(&m_code, &b_code).await;
                    if ok {
                        let mut app_state = state_ref.state.write().await;
                        app_state.db_success += 1;
                        let _ = state_ref.tx.send(app_state.get_snapshot());
                    }
                });
            }
        }
    });

    // 2. Spawn dynamic devices.ini config watcher (Hot-Reload on the fly)
    let shared_state_watcher = shared_state.clone();
    let tx_barcode_watcher = tx_barcode.clone();
    let is_cli_override = !cli_ips.is_empty();
    let initial_targets = target_devices.clone();

    tokio::spawn(async move {
        watch_devices_config(
            shared_state_watcher,
            tx_barcode_watcher,
            is_cli_override,
            initial_targets,
        )
        .await;
    });

    // 3. Start Embedded Axum Web Server with Configurable & Automatic Fallback Port Binding
    let port_str = env::var("WEB_PORT").unwrap_or_else(|_| "8088".into());
    let initial_port: u16 = port_str.parse().unwrap_or(8088);

    let router = create_web_router(shared_state);
    let mut active_port = initial_port;
    let listener = loop {
        match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", active_port)).await {
            Ok(l) => break l,
            Err(e) => {
                if active_port < initial_port + 20 {
                    println!("\x1b[93m[WARN] Port {} is in use, retrying on port {}...\x1b[0m", active_port, active_port + 1);
                    active_port += 1;
                } else {
                    let err_msg = format!("Failed to bind Web Server port starting from {}: {}", initial_port, e);
                    println!("\x1b[91m[ERROR] {}\x1b[0m", err_msg);
                    logger::log_system(&format!("ERROR: {}", err_msg));
                    return Err(e.into());
                }
            }
        }
    };

    let status_msg = format!("[RUST ENGINE] Live UI Dashboard running at http://localhost:{}", active_port);
    println!("\x1b[92m{}\x1b[0m", status_msg);
    logger::log_system(&status_msg);

    axum::serve(listener, router).await?;

    Ok(())
}

