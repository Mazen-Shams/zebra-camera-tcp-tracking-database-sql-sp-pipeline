use crate::types::{BarcodeItem, DeviceState, SharedAppState};
use chrono::Local;
use socket2::{SockRef, TcpKeepalive};
use std::time::Duration;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::net::TcpStream;
use tokio::sync::mpsc;

pub async fn listen_to_device(
    machine_code: String,
    host: String,
    port: u16,
    shared_state: SharedAppState,
    tx_barcode: mpsc::Sender<BarcodeItem>,
) {
    let mut reconnect_attempts = 0;

    // Initialize device state
    {
        let mut app_state = shared_state.state.write().await;
        app_state.devices.insert(
            machine_code.clone(),
            DeviceState {
                ip: host.clone(),
                status: "INIT".into(),
                keepalive: false,
                retry: 0,
                last_barcode: "N/A".into(),
                last_time: "N/A".into(),
            },
        );
        let snapshot = app_state.get_snapshot();
        let _ = shared_state.tx.send(snapshot);
    }

    loop {
        // Update state to CONNECTING
        {
            let mut app_state = shared_state.state.write().await;
            if let Some(dev) = app_state.devices.get_mut(&machine_code) {
                dev.status = "CONNECTING".into();
                dev.retry = reconnect_attempts;
            }
            let snapshot = app_state.get_snapshot();
            let _ = shared_state.tx.send(snapshot);
        }

        let addr = format!("{}:{}", host, port);
        let connect_res = tokio::time::timeout(
            Duration::from_millis(2000),
            TcpStream::connect(&addr),
        )
        .await;

        match connect_res {
            Ok(Ok(stream)) => {
                // Configure OS-level TCP Keep-Alive via socket2
                let mut keepalive_ok = false;
                if let Ok(std_stream) = stream.into_std() {
                    let sock = SockRef::from(&std_stream);
                    let keepalive_params = TcpKeepalive::new()
                        .with_time(Duration::from_millis(5000))
                        .with_interval(Duration::from_millis(1000));

                    if sock.set_tcp_keepalive(&keepalive_params).is_ok()
                        && sock.set_keepalive(true).is_ok()
                    {
                        keepalive_ok = true;
                    }

                    if let Ok(async_stream) = TcpStream::from_std(std_stream) {
                        // Mark state as CONNECTED
                        reconnect_attempts = 0;
                        {
                            let mut app_state = shared_state.state.write().await;
                            if let Some(dev) = app_state.devices.get_mut(&machine_code) {
                                dev.status = "CONNECTED".into();
                                dev.retry = 0;
                                dev.keepalive = keepalive_ok;
                            }
                            let snapshot = app_state.get_snapshot();
                            let _ = shared_state.tx.send(snapshot);
                        }

                        let (reader, _) = async_stream.into_split();
                        let mut buf_reader = BufReader::new(reader);
                        let mut line = String::new();

                        loop {
                            line.clear();
                            match buf_reader.read_line(&mut line).await {
                                Ok(0) => {
                                    // Connection closed by remote end
                                    crate::logger::log_camera_tcp_error(&format!("[{}] Remote socket closed connection", machine_code));
                                    break;
                                }
                                Ok(_) => {
                                    let barcode_str = line.trim().to_string();
                                    if !barcode_str.is_empty() {
                                        let now_str =
                                            Local::now().format("%H:%M:%S - %d/%m").to_string();

                                        let item = BarcodeItem {
                                            timestamp: now_str,
                                            machine_code: machine_code.clone(),
                                            host: host.clone(),
                                            barcode: barcode_str,
                                        };

                                        let _ = tx_barcode.send(item).await;
                                    }
                                }
                                Err(e) => {
                                    crate::logger::log_camera_tcp_error(&format!("[{}] Socket read error: {}", machine_code, e));
                                    break;
                                }
                            }
                        }
                    }
                }

                // Handle drop / disconnection
                {
                    let mut app_state = shared_state.state.write().await;
                    if let Some(dev) = app_state.devices.get_mut(&machine_code) {
                        dev.status = "KEEP_ALIVE_FAIL".into();
                        dev.keepalive = false;
                    }
                    let snapshot = app_state.get_snapshot();
                    let _ = shared_state.tx.send(snapshot);
                }
            }
            Ok(Err(e)) => {
                // Connection failed
                crate::logger::log_camera_tcp_error(&format!("[{}] Connection failed to {}: {}", machine_code, addr, e));
                {
                    let mut app_state = shared_state.state.write().await;
                    if let Some(dev) = app_state.devices.get_mut(&machine_code) {
                        dev.status = "OFFLINE".into();
                        dev.keepalive = false;
                    }
                    let snapshot = app_state.get_snapshot();
                    let _ = shared_state.tx.send(snapshot);
                }
            }
            Err(_) => {
                // Connection timed out (2.0s fast timeout)
                crate::logger::log_camera_tcp_error(&format!("[{}] Connection timeout (2.0s) to {}", machine_code, addr));
                {
                    let mut app_state = shared_state.state.write().await;
                    if let Some(dev) = app_state.devices.get_mut(&machine_code) {
                        dev.status = "TIMEOUT".into();
                        dev.keepalive = false;
                    }
                    let snapshot = app_state.get_snapshot();
                    let _ = shared_state.tx.send(snapshot);
                }
            }
        }

        reconnect_attempts += 1;
        let backoff_secs = (0.5 * (2.0_f64.powi((reconnect_attempts as i32) - 1))).min(5.0);
        {
            let mut app_state = shared_state.state.write().await;
            if let Some(dev) = app_state.devices.get_mut(&machine_code) {
                dev.status = format!("RECONNECTING ({})", reconnect_attempts);
                dev.retry = reconnect_attempts;
                dev.keepalive = false;
            }
            let snapshot = app_state.get_snapshot();
            let _ = shared_state.tx.send(snapshot);
        }

        tokio::time::sleep(Duration::from_secs_f64(backoff_secs)).await;
    }
}

