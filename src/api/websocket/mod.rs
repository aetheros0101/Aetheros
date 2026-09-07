// ============================================================
// src/api/websocket/mod.rs
//
// Faz 7: WebSocket → EventBus stream.
//
// İstemci bağlanır → EventBus'a subscribe olur →
// her SystemEvent JSON olarak iletilir.
//
// Axum WebSocket upgrade flow:
//   GET /ws → upgrade → ws_handler()
//   → EventBus::subscribe() → broadcast::Receiver
//   → döngü: recv() → JSON serialize → send()
//   → client disconnect veya bus kapanınca çık
// ============================================================

pub mod events;
pub mod subscriptions;

use axum::{
    extract::Query,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json;
use tracing::{
    debug,
    info,
    warn,
};

use crate::api::middleware::authenticate_ws;
use crate::api::rest::router::AppState;
use crate::events::bus::SystemEvent;

/// Tarayıcı WebSocket API'si custom header gönderemediği için kimlik
/// bilgisi query string üzerinden taşınır: /ws?token=<jwt> veya
/// /ws?api_key=<key>. Bkz. `crate::api::middleware::authenticate_ws`.
#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
    pub api_key: Option<String>,
}

/// WebSocket upgrade endpoint.
/// Router'a şöyle eklenir:
///   .route("/ws", get(ws_upgrade_handler))
pub async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(auth): Query<WsAuthQuery>,
) -> Response {
    if !authenticate_ws(auth.token.as_deref(), auth.api_key.as_deref()) {
        warn!("WebSocket bağlantısı reddedildi: geçersiz/eksik kimlik bilgisi");
        return (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: provide ?token=<jwt> or ?api_key=<key>",
        )
            .into_response();
    }

    ws.on_upgrade(move |socket| ws_handler(socket, state))
        .into_response()
}

async fn ws_handler(
    mut socket: WebSocket,
    state: AppState,
) {
    info!("WebSocket client connected");

    let mut receiver = state.events.subscribe();

    loop {
        tokio::select! {
            // EventBus'tan yeni event
            result = receiver.recv() => {
                match result {
                    Ok(event) => {
                        let json = match serialize_event(&event) {
                            Some(j) => j,
                            None => continue,
                        };

                        if socket
                            .send(Message::Text(json.into()))
                            .await
                            .is_err()
                        {
                            // İstemci bağlantıyı kapattı
                            debug!("WebSocket client disconnected");
                            break;
                        }
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Buffer taştı — istemci çok yavaş
                        warn!(skipped = n, "WebSocket client lagging, events skipped");
                    }

                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // EventBus kapandı (runtime shutdown)
                        info!("EventBus closed, closing WebSocket");
                        break;
                    }
                }
            }

            // İstemciden gelen mesaj (ping/pong veya subscription)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => {
                        info!("WebSocket client closed connection");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket
                            .send(Message::Pong(data))
                            .await;
                    }
                    _ => {}
                }
            }
        }
    }

    info!("WebSocket session ended");
}

/// SystemEvent → JSON string.
/// Serialize edilemeyen event'ler None döner (skip).
fn serialize_event(event: &SystemEvent) -> Option<String> {
    match event {
        SystemEvent::Task(e) => {
            serde_json::to_string(&serde_json::json!({
                "type": "task",
                "event": format!("{:?}", e)
            }))
            .ok()
        }
        SystemEvent::Runtime(e) => {
            serde_json::to_string(&serde_json::json!({
                "type": "runtime",
                "event": format!("{:?}", e)
            }))
            .ok()
        }
        SystemEvent::Worker(e) => {
            serde_json::to_string(&serde_json::json!({
                "type": "worker",
                "event": format!("{:?}", e)
            }))
            .ok()
        }
        SystemEvent::Telemetry(e) => {
            serde_json::to_string(&serde_json::json!({
                "type": "telemetry",
                "event": format!("{:?}", e)
            }))
            .ok()
        }
    }
}
