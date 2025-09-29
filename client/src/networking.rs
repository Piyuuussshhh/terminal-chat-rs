use std::{error::Error, io, net::SocketAddr, time::Duration};
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, net::{TcpStream, UdpSocket}};
use housechat::{client_model::Credentials, protocol::MessageProtocol};

/// Broadcast "I want to connect to the HouseChat server" and the server will reply with its address (ip + port).
pub async fn find_server() -> io::Result<SocketAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;

    // Send discovery message
    let broadcast_addr = format!("255.255.255.255:{}", housechat::DISCOVERY_PORT);
    socket
        .send_to(housechat::DISCOVERY_MESSAGE, broadcast_addr)
        .await?;
    log::info!("Discovery message broadcasted!");

    let mut buf = [0; 1024];

    let wait_time = Duration::from_secs(5);
    let res = tokio::time::timeout(wait_time, socket.recv_from(&mut buf)).await;

    match res {
        // The address returned by socket.recv_from() is of the UDP server, therefore we cannot use that.
        Ok(Ok((len, _))) => {
            let server_addr_str = String::from_utf8_lossy(&buf[..len]);
            let server_addr: SocketAddr = server_addr_str
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            Ok(server_addr)
        }
        // Network error
        Ok(Err(e)) => Err(e),
        // Timeout
        Err(e) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("Server discovery timed out: {e}"),
        )),
    }
}

pub async fn chat(
    server_addr: SocketAddr, 
    credentials: Credentials
) -> Result<(), Box<dyn Error>> {
    let stream = TcpStream::connect(server_addr).await?;
    log::info!("Connected to server at {}.", server_addr);
    let (reader_half, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader_half);

    let mut cred_json = serde_json::to_string(&credentials)?;
    // Pushed '\n' so that server's reader.read_line() works correctly
    cred_json.push('\n');
    writer.write_all(cred_json.as_bytes()).await?;
    writer.flush().await?;
    log::info!("Sent client credentials to the server.");

    let mut server_response = String::new();
    reader.read_line(&mut server_response).await?;
    let message = MessageProtocol::try_from(server_response)?;
    print!("[{}]: {}", message.sender_username, message.payload);

    println!("--- You are now in the chat! ---");

    let mut network_buffer = String::new();
    let mut user_input_buffer = String::new();
    let tokio_stdin = tokio::io::stdin();
    let mut tokio_stdin_reader = BufReader::new(tokio_stdin);
    loop {
        tokio::select! {
            // A message is received from the server
            res = reader.read_line(&mut network_buffer) => {
                match res {
                    Ok(0) => {
                        log::info!("Server has closed the connection. Exiting the application...");
                        break Ok(());
                    },
                    Ok(_) => {
                        if let Ok(msg) = MessageProtocol::try_from(network_buffer.clone()) {
                            if msg.sender_username == credentials.username {
                                println!("[Me]: {}", msg.payload);
                            } else {
                                println!("[{}]: {}", msg.sender_username, msg.payload);
                            }
                        }
                        network_buffer.clear();
                    },
                    Err(e) => {
                        log::error!("Error reading from the server: {e}");
                        break Err(Box::new(e));
                    },
                }
            }
            // The client has sent a message to the server
            res = tokio_stdin_reader.read_line(&mut user_input_buffer) => {
                match res {
                    Ok(_) => {
                        let input = user_input_buffer.trim();
                        if input == ":q" {
                            println!("Disconnecting...");
                            log::info!("{} has decided to stop chatting.", credentials.username);
                            break Ok(());
                        }
                        // Send the original buffer before being trimmed to the server because the server expects the \n
                        writer.write_all(user_input_buffer.as_bytes()).await?;
                        writer.flush().await?;
                        user_input_buffer.clear();
                    },
                    Err(e) => {
                        log::error!("Error reading from stdin: {e}");
                        break Err(Box::new(e));
                    },
                }
            }
        }
    }
}
