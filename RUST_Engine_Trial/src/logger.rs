use chrono::Local;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_LOG_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

static LOG_LOCK: Mutex<()> = Mutex::new(());

fn get_log_dir() -> PathBuf {
    let mut possible_paths = vec![
        PathBuf::from("logs"),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            possible_paths.push(exe_dir.join("logs"));
        }
    }

    for p in &possible_paths {
        if p.exists() {
            return p.clone();
        }
    }

    let default_dir = PathBuf::from("logs");
    let _ = create_dir_all(&default_dir);
    default_dir
}

fn rotate_if_needed(file_path: &Path) {
    if let Ok(metadata) = std::fs::metadata(file_path) {
        if metadata.len() >= MAX_LOG_SIZE_BYTES {
            let old_path = file_path.with_extension("log.old");
            let _ = std::fs::rename(file_path, old_path);
        }
    }
}

fn append_to_file(filename: &str, line: &str) {
    let _guard = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = get_log_dir();
    let file_path = dir.join(filename);

    rotate_if_needed(&file_path);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
    {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let _ = writeln!(file, "[{}] {}", timestamp, line);
    }
}

pub fn log_db_error(msg: &str) {
    append_to_file("db_errors.log", msg);
}

pub fn log_camera_tcp_error(msg: &str) {
    append_to_file("camera_tcp_errors.log", msg);
}

pub fn log_audit_trace(msg: &str) {
    append_to_file("audit_trace.log", msg);
}

pub fn log_system(msg: &str) {
    append_to_file("system.log", msg);
}

