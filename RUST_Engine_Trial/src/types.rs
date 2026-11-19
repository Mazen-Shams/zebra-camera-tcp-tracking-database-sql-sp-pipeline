use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceState {
    pub ip: String,
    pub status: String,
    pub keepalive: bool,
    pub retry: usize,
    pub last_barcode: String,
    pub last_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_devices: usize,
    pub connected: usize,
    pub total_reads: u64,
    pub good_reads: u64,
    pub bad_reads: u64,
    pub db_success: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "SNAPSHOT")]
    Snapshot {
        devices: HashMap<String, DeviceState>,
        stats: Stats,
    },
    #[serde(rename = "SCAN_EVENT")]
    ScanEvent {
        timestamp: String,
        machine_code: String,
        host: String,
        barcode: String,
        db_status: String,
    },
}

#[derive(Debug, Clone)]
pub struct BarcodeItem {
    pub timestamp: String,
    pub machine_code: String,
    pub host: String,
    pub barcode: String,
}

pub struct AppStateData {
    pub devices: HashMap<String, DeviceState>,
    pub total_reads: u64,
    pub good_reads: u64,
    pub bad_reads: u64,
    pub db_success: u64,
}

impl AppStateData {
    pub fn get_snapshot(&self) -> WsMessage {
        let connected_count = self
            .devices
            .values()
            .filter(|d| d.status == "CONNECTED")
            .count();

        WsMessage::Snapshot {
            devices: self.devices.clone(),
            stats: Stats {
                total_devices: self.devices.len(),
                connected: connected_count,
                total_reads: self.total_reads,
                good_reads: self.good_reads,
                bad_reads: self.bad_reads,
                db_success: self.db_success,
            },
        }
    }
}

#[derive(Clone)]
pub struct SharedAppState {
    pub state: Arc<RwLock<AppStateData>>,
    pub tx: broadcast::Sender<WsMessage>,
}

