use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl Credentials {
    pub fn new(username: String, password: String) -> Self {
        Self {
            username,
            password
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    pub id: Uuid,
    pub credentials: Credentials
}

impl Client {
    pub fn new(username: String, password: String) -> Self {
        let id = Uuid::new_v4();
        let credentials = Credentials::new(username, password);
        Self {
            id,
            credentials
        }
    }
}

impl TryFrom<String> for Client {
    type Error = serde_json::Error;

    fn try_from(credentials: String) -> Result<Self, Self::Error> {
        let id = Uuid::new_v4();
        let credentials= serde_json::from_str::<Credentials>(&credentials)?;
        Ok(Self {
            id,
            credentials
        })
    }
}
