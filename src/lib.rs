
pub mod protocol;

use std::error::Error;
pub type HouseChatResult<T> = Result<T, Box<dyn Error>>;

pub const DISCOVERY_PORT: u16 = 8081;
pub const DISCOVERY_MESSAGE: &[u8] = b"HOUSE_CHAT_SERVER_DISCOVERY";