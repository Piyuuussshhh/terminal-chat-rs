mod networking;
mod ui;

use std::error::Error;
use housechat::{client_model::Credentials};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    housechat::init_log(housechat::CLIENT_LOG_FILE)?;

    let server_addr = match networking::find_server().await {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Error discovering server: {}", e);
            return Ok(());
        }
    };

    println!("Please enter your username:");
    let mut username = String::new();
    std::io::stdin().read_line(&mut username)?;
    let username = username.trim().to_string();

    println!("Please enter your password:");
    let mut password = String::new();
    std::io::stdin().read_line(&mut password)?;
    let password = password.trim().to_string();

    let credentials = Credentials::new(username, password);

    if let Err(e) = networking::chat(server_addr, credentials).await {
        eprintln!("[ERROR] Chat session ended with an error: {}", e);
    }

    Ok(())
}

