// SPDX-License-Identifier: MIT
use crate::models::Item;

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

    let items = load_items(&db).await?;
    Ok((db, items))
}

async fn load_items(db: &libsql::Database) -> anyhow::Result<Vec<Item>> {
    let conn = db.connect()?;
    let mut rows = conn
        .query(
            "SELECT id, name, where_to_buy, description, created_at, modified_at \
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
            created_at: row.get(4)?,
            modified_at: row.get(5)?,
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
        "INSERT INTO items (id, name, where_to_buy, description, created_at, modified_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        libsql::params![
            id.clone(),
            name.clone(),
            wb.clone(),
            desc.clone(),
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
