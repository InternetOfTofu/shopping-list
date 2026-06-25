// SPDX-License-Identifier: MIT
mod db;
mod models;
mod ws;

use std::io::Write;
use std::sync::Arc;

use axum::{response::Html, routing::get, Router};
use tokio::sync::{broadcast, RwLock};

use models::{AppState, SharedState};

fn install_signal_handler() {
    unsafe {
        libc::signal(
            libc::SIGSEGV,
            sigsegv_handler as *const () as libc::sighandler_t,
        );
    }
}

extern "C" fn sigsegv_handler(sig: libc::c_int) {
    let msg = format!("FATAL: received signal {sig} (SIGSEGV) — segmentation fault\n");
    unsafe {
        libc::write(libc::STDERR_FILENO, msg.as_ptr().cast(), msg.len());
    }
    std::process::abort();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Flush backtraces for every panic
    std::env::set_var("RUST_BACKTRACE", "1");

    // Unbuffered stderr subscriber so crash logs survive
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Panic hook — dump to stderr before the process can die silently
    std::panic::set_hook(Box::new(|info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "Box<dyn Any>".to_string());
        let backtrace = std::backtrace::Backtrace::force_capture();
        eprintln!("FATAL: panic at {loc}: {payload}");
        eprintln!("{backtrace}\n");
        let _ = std::io::stderr().flush();
    }));

    install_signal_handler();

    tracing::info!(
        "shopping-list v{APP_VERSION} starting",
        APP_VERSION = env!("CARGO_PKG_VERSION")
    );

    tracing::info!("initializing database…");
    let (db, items) = db::init_db().await?;
    tracing::info!("loaded {} items from database", items.len());

    let (tx, _) = broadcast::channel::<String>(32);

    let state: SharedState = Arc::new(AppState {
        items: RwLock::new(items),
        db,
        tx,
    });

    tracing::info!("building router…");
    let app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .with_state(state)
        .fallback(get(|| async move {
            Html(include_str!("../templates/index.html"))
        }));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let addr = format!("0.0.0.0:{port}");
    tracing::info!("binding to {addr}…");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(
        "listening on http://localhost:{port} (PID: {})",
        std::process::id()
    );

    axum::serve(listener, app).await?;

    Ok(())
}
