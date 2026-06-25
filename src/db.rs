// SPDX-License-Identifier: MIT
use crate::models::Item;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn version_lt(a: &str, b: &str) -> bool {
    let parse = |v: &str| {
        v.split('.')
            .filter_map(|s| s.parse::<u64>().ok())
            .collect::<Vec<_>>()
    };
    parse(a) < parse(b)
}

pub async fn init_db() -> anyhow::Result<(libsql::Database, Vec<Item>)> {
    let db_path =
        std::env::var("SHOPPING_LIST_DB").unwrap_or_else(|_| String::from("./data/shopping.db"));

    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let db = libsql::Builder::new_local(&db_path).build().await?;
    let conn = db.connect()?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            where_to_buy TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            modified_at TEXT NOT NULL
        )",
        (),
    )
    .await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS _schema (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        (),
    )
    .await?;

    let mut rows = conn
        .query("SELECT value FROM _schema WHERE key = 'version'", ())
        .await?;
    let db_ver: String = match rows.next().await? {
        Some(row) => row.get(0)?,
        None => String::from("0.0.0"),
    };

    tracing::info!("DB schema version: {db_ver} (app version: {APP_VERSION})");

    if version_lt(&db_ver, "0.2.0") {
        tracing::info!("Running migration 0.2.0: adding done column");
        conn.execute(
            "ALTER TABLE items ADD COLUMN done INTEGER NOT NULL DEFAULT 0",
            (),
        )
        .await?;

        conn.execute(
            "INSERT OR REPLACE INTO _schema (key, value) VALUES ('version', ?1)",
            libsql::params![APP_VERSION],
        )
        .await?;
        tracing::info!("Schema upgraded to {APP_VERSION}");
    }

    let items = load_items(&db).await?;
    Ok((db, items))
}

async fn load_items(db: &libsql::Database) -> anyhow::Result<Vec<Item>> {
    let conn = db.connect()?;
    let mut rows = conn
        .query(
            "SELECT id, name, where_to_buy, description, done, created_at, modified_at \
             FROM items ORDER BY rowid DESC",
            (),
        )
        .await?;

    let mut items = Vec::new();
    while let Some(row) = rows.next().await? {
        items.push(Item {
            id: row.get(0)?,
            name: row.get(1)?,
            where_to_buy: row.get(2)?,
            description: row.get(3)?,
            done: row.get(4)?,
            created_at: row.get(5)?,
            modified_at: row.get(6)?,
        });
    }
    Ok(items)
}

pub async fn insert_item(conn: &libsql::Connection, item: &Item) -> anyhow::Result<()> {
    let id = &item.id;
    let name = &item.name;
    let wb = &item.where_to_buy;
    let desc = &item.description;
    let ca = &item.created_at;
    let ma = &item.modified_at;

    conn.execute(
        "INSERT INTO items (id, name, where_to_buy, description, done, created_at, modified_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        libsql::params![
            id.clone(),
            name.clone(),
            wb.clone(),
            desc.clone(),
            false,
            ca.clone(),
            ma.clone()
        ],
    )
    .await?;
    Ok(())
}

pub async fn update_item(
    conn: &libsql::Connection,
    id: &str,
    name: &str,
    where_to_buy: &str,
    description: &str,
    modified_at: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE items SET name = ?1, where_to_buy = ?2, description = ?3, modified_at = ?4 \
         WHERE id = ?5",
        libsql::params![name, where_to_buy, description, modified_at, id],
    )
    .await?;
    Ok(())
}

pub async fn delete_item(conn: &libsql::Connection, id: &str) -> anyhow::Result<()> {
    conn.execute("DELETE FROM items WHERE id = ?1", libsql::params![id])
        .await?;
    Ok(())
}

pub async fn toggle_done_item(
    conn: &libsql::Connection,
    id: &str,
    done: bool,
    modified_at: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE items SET done = ?1, modified_at = ?2 WHERE id = ?3",
        libsql::params![done, modified_at, id],
    )
    .await?;
    Ok(())
}
