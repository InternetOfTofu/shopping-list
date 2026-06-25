// SPDX-License-Identifier: MIT
mod db;
mod models;
mod ws;

use std::sync::Arc;

use axum::{response::Html, routing::get, Router};
use tokio::sync::{broadcast, RwLock};

use models::{AppState, SharedState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (db, items) = db::init_db().await?;
    tracing::info!("loaded {} items from db", items.len());

    let (tx, _) = broadcast::channel::<String>(32);

    let state: SharedState = Arc::new(AppState {
        items: RwLock::new(items),
        db,
        tx,
    });

    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .with_state(state.clone())
        .fallback(get(|| async move {
            Html(include_str!("../templates/index.html"))
        }));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://localhost:{port}");

    axum::serve(listener, app).await?;

    Ok(())
}
