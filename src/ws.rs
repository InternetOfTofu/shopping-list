// SPDX-License-Identifier: MIT
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use chrono::Utc;
use tokio::sync::broadcast;

use crate::db;
use crate::models::{AppState, ClientMessage, FullSyncMessage, Item, SharedState};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: SharedState) {
    send_full_sync(&mut socket, &state).await;

    let mut rx = state.tx.subscribe();

    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        send_full_sync(&mut socket, &state).await;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            client_msg = socket.recv() => {
                match client_msg {
                    Some(Ok(Message::Text(text))) => {
                        dispatch(&text, &state).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    tracing::info!("WebSocket disconnected");
}

async fn send_full_sync(socket: &mut WebSocket, state: &Arc<AppState>) {
    let items = state.items.read().await.clone();
    let msg = FullSyncMessage {
        msg_type: "full_sync".to_string(),
        items,
    };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = socket.send(Message::Text(json)).await;
    }
}

async fn dispatch(text: &str, state: &SharedState) {
    let msg: ClientMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("bad client message: {e}");
            return;
        }
    };

    let conn = match state.db.connect() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("db connect failed: {e}");
            return;
        }
    };

    let now = Utc::now().to_rfc3339();

    match msg {
        ClientMessage::Add {
            id,
            name,
            where_to_buy,
            description,
            done,
        } => {
            let item = Item {
                id,
                name,
                where_to_buy,
                description,
                done,
                created_at: now.clone(),
                modified_at: now.clone(),
            };

            if let Err(e) = db::insert_item(&conn, &item).await {
                tracing::error!("insert failed: {e}");
                return;
            }

            state.items.write().await.insert(0, item);
        }

        ClientMessage::Update { id, changes } => {
            let mut items = state.items.write().await;
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                let new_name = changes
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&item.name)
                    .to_string();
                let new_where = changes
                    .get("whereToBuy")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&item.where_to_buy)
                    .to_string();
                let new_desc = changes
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&item.description)
                    .to_string();
                let new_modified = now.clone();

                if let Err(e) =
                    db::update_item(&conn, &id, &new_name, &new_where, &new_desc, &new_modified)
                        .await
                {
                    tracing::error!("update failed: {e}");
                    return;
                }

                item.name = new_name;
                item.where_to_buy = new_where;
                item.description = new_desc;
                item.modified_at = new_modified;
            }
        }

        ClientMessage::Delete { id } => {
            if let Err(e) = db::delete_item(&conn, &id).await {
                tracing::error!("delete failed: {e}");
                return;
            }

            state.items.write().await.retain(|i| i.id != id);
        }

        ClientMessage::ToggleDone { id } => {
            let mut items = state.items.write().await;
            let new_modified = now.clone();
            if let Some(item) = items.iter_mut().find(|i| i.id == id) {
                let new_done = !item.done;

                if let Err(e) = db::toggle_done_item(&conn, &item.id, new_done, &new_modified).await
                {
                    tracing::error!("toggle_done failed: {e}");
                    return;
                }

                item.done = new_done;
                item.modified_at = new_modified;
            }
        }
    }

    broadcast(state).await;
}

async fn broadcast(state: &AppState) {
    let items = state.items.read().await.clone();
    let msg = FullSyncMessage {
        msg_type: "full_sync".to_string(),
        items,
    };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = state.tx.send(json);
    }
}
