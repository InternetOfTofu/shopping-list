# Shopping List

Real-time collaborative shopping list — single **7 MB** binary, zero external dependencies at runtime. Add, edit, and delete items; changes sync instantly to every open browser via WebSocket.

## Quick start

```bash
# build (requires Rust)
cargo build --release

# run
./target/release/shopping-list
# → http://localhost:3000
```

Open the URL in multiple browser tabs — any change in one tab appears immediately in the others.

## Configuration

| Env var             | Default               | Purpose              |
|---------------------|-----------------------|----------------------|
| `PORT`              | `3000`                | HTTP listen port     |
| `SHOPPING_LIST_DB`  | `./data/shopping.db`  | SQLite database path |

```bash
PORT=8080 SHOPPING_LIST_DB=/var/lib/shopping/shopping.db ./shopping-list
```

## Architecture

```
Browser A ──WebSocket──┐
                       ├── Rust server (Axum + libsql) ── SQLite
Browser B ──WebSocket──┘
```

- **Server** is the single source of truth — mutations flow through it, state is broadcast to all clients
- **Clients** show changes immediately with a "Syncing…" indicator; the indicator clears when the server confirms
- **Database** is embedded [libsql](https://github.com/tursodatabase/libsql) (SQLite-compatible); data persists across restarts
- **No auth, no accounts** — designed for a shared device or trusted local network

## Sync states

| Status    | Visual                                          | Means                        |
|-----------|-------------------------------------------------|------------------------------|
| synced    | normal opacity                                  | confirmed by server          |
| pending   | pulsing opacity + "Syncing…" chip               | sent to server, awaiting ack |
| deleting  | strikethrough + "Deleting…" chip + Undo button   | delete sent, awaiting ack    |

On disconnect the client shows a red banner and reconnects with exponential backoff (1s → 2s → 4s → 8s → 10s cap).

## Project structure

```
src/
├── main.rs      # entry point, Axum router
├── models.rs    # Item, ClientMessage, AppState types
├── db.rs        # libsql init, CRUD operations
└── ws.rs        # WebSocket handler, broadcast, dispatch

templates/
└── index.html   # embedded into the binary at compile time

data/            # runtime SQLite database (gitignored)
```

## Tech

| Layer   | Choice                              |
|---------|-------------------------------------|
| Server  | Axum 0.7 + Tokio 1.x                |
| Sync    | WebSocket + `tokio::sync::broadcast`|
| DB      | libsql 0.4 (embedded SQLite)        |
| Frontend| Vanilla JS + CSS (Material)         |

## Other note
This software is AI-generated, and is designed for my personal use.
