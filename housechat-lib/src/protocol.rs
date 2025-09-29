use std::{error::Error, net::SocketAddr};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageProtocol {
    pub id: Uuid,
    pub sender_addr: SocketAddr,
    pub sender_username: String,
    pub payload: String,
}

impl MessageProtocol {
    pub fn new(id: Uuid, sender_addr: SocketAddr, sender_username: String, payload: String) -> Self {
        Self {
            id,
            sender_addr,
            sender_username,
            payload
        }
    }

    pub fn to_json(&self) -> Result<String, Box<dyn Error>> {
        let mut json = serde_json::to_string(self)?;
        json.push('\n');
        Ok(json)
    }
}

impl TryFrom<String> for MessageProtocol {
    type Error = serde_json::Error;

    fn try_from(json: String) -> Result<Self, Self::Error> {
        let msg = serde_json::from_str::<MessageProtocol>(&json)?;
        Ok(msg)
    }
}