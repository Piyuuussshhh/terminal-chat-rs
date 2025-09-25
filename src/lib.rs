use std::{error::Error, fs::OpenOptions};

use log::LevelFilter;
use simplelog::{format_description, ConfigBuilder, WriteLogger};

pub mod protocol;

pub const DISCOVERY_PORT: u16 = 8081;
pub const DISCOVERY_MESSAGE: &[u8] = b"HOUSE_CHAT_SERVER_DISCOVERY";
pub const CLIENT_LOG_FILE: &str = "client.log";
pub const SERVER_LOG_FILE: &str = "server.log";

pub fn init_log(log_file: &str) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file)?;

    let config = ConfigBuilder::new()
        .set_time_format_custom(format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
        ))
        .build();

    WriteLogger::init(LevelFilter::Info, config, file)?;

    Ok(())
}