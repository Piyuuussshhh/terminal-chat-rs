use std::{error::Error, net::SocketAddr};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageProtocol {
    pub sender_addr: SocketAddr,
    pub id: Uuid,
    pub username: String,
    pub payload: String,
}

impl MessageProtocol {
    pub fn new(sender_addr: SocketAddr, id: Uuid, username: String, payload: String) -> Self {
        Self {
            sender_addr,
            id,
            username,
            payload
        }
    }

    pub fn to_json(&self) -> Result<String, Box<dyn Error>> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}