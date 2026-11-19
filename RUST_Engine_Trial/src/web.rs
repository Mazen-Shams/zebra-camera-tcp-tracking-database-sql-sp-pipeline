use crate::types::SharedAppState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use std::fs;
use std::path::PathBuf;

const EMBEDDED_DASHBOARD: &str = include_str!("../dashboard.html");

pub fn create_web_router(shared_state: SharedAppState) -> Router {
    Router::new()
        .route("/", get(handle_index))
        .route("/ws", get(handle_websocket))
        .with_state(shared_state)
}

async fn handle_index() -> impl IntoResponse {
    let mut possible_paths = vec![
        PathBuf::from("dashboard.html"),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            possible_paths.push(exe_dir.join("dashboard.html"));
        }
    }

    for p in &possible_paths {
        if p.exists() {
            if let Ok(content) = fs::read_to_string(p) {
                return Html(content);
            }
        }
    }

    // Fall back to statically embedded dashboard HTML compiled into the binary
    Html(EMBEDDED_DASHBOARD.to_string())
}

async fn handle_websocket(
    ws: WebSocketUpgrade,
    State(shared_state): State<SharedAppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, shared_state))
}

async fn handle_socket(mut socket: WebSocket, shared_state: SharedAppState) {
    let mut rx = shared_state.tx.subscribe();

    // Send initial snapshot on client connect
    {
        let app_state = shared_state.state.read().await;
        let snapshot = app_state.get_snapshot();
        if let Ok(json_str) = serde_json::to_string(&snapshot) {
            let _ = socket.send(Message::Text(json_str)).await;
        }
    }

    // Broadcast stream loop
    while let Ok(msg) = rx.recv().await {
        if let Ok(json_str) = serde_json::to_string(&msg) {
            if socket.send(Message::Text(json_str)).await.is_err() {
                break;
            }
        }
    }
}

