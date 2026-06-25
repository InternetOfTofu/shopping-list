// SPDX-License-Identifier: MIT
use std::sync::Arc;

use tokio::sync::{broadcast, RwLock};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(rename = "whereToBuy")]
    pub where_to_buy: String,
    pub description: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "modifiedAt")]
    pub modified_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "add")]
    Add {
        id: String,
        name: String,
        #[serde(default)]
        #[serde(rename = "whereToBuy")]
        where_to_buy: String,
    },
    #[serde(rename = "update")]
    Update {
        id: String,
        changes: serde_json::Value,
    },
    #[serde(rename = "delete")]
    Delete { id: String },
}

#[derive(Debug, Serialize)]
pub struct FullSyncMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub items: Vec<Item>,
}

pub struct AppState {
    pub items: RwLock<Vec<Item>>,
    pub db: libsql::Database,
    pub tx: broadcast::Sender<String>,
}

pub type SharedState = Arc<AppState>;
