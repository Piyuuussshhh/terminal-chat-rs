pub mod protocol;
pub mod client_model;

use std::{error::Error, fs::OpenOptions};
use log::LevelFilter;
use simplelog::{format_description, ConfigBuilder, WriteLogger};
use uuid::Uuid;

pub const DISCOVERY_PORT: u16 = 8081;
pub const DISCOVERY_MESSAGE: &[u8] = b"HOUSE_CHAT_SERVER_DISCOVERY";
pub const CLIENT_LOG_FILE: &str = "client.log";
pub const SERVER_LOG_FILE: &str = "server.log";

pub const SERVER_ID: Uuid = Uuid::nil();
pub const SERVER_NAME: &str = "HouseChat";

pub fn init_log(log_file: &str) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file)?;

    let mut builder = ConfigBuilder::new();
    builder.set_time_format_custom(format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second]"
    ));
    if builder.set_time_offset_to_local().is_err() {
        log::warn!("Could not determine local time zone. Logging in UTC.");
    }

    WriteLogger::init(LevelFilter::Info, builder.build(), file)?;

    Ok(())
}