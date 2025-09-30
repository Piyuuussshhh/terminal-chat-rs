use housechat::protocol::MessageProtocol;
use std::{error::Error, io, net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpStream, UdpSocket},
    sync::mpsc::{Receiver, Sender},
};

use super::ui::comms;

pub async fn discovery_task(tx: Sender<comms::Event>) -> Result<(), Box<dyn Error + Send + Sync>> {
    match find_server().await {
        Ok(addr) => {
            tx.send(comms::Event::ServerFound(addr)).await?;
        }
        Err(e) => {
            tx.send(comms::Event::Error(format!(
                "Server discovery failed: {}",
                e.to_string()
            )))
            .await?;
        }
    }
    Ok(())
}

/// Broadcast "I want to connect to the HouseChat server" and the server will reply with its address (ip + port).
async fn find_server() -> io::Result<SocketAddr> {
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

/// Connects to the TCP server, then relays user action from the TUI to the server AND messages sent by the server to the TUI.
pub async fn network_task(
    mut action_rx: Receiver<comms::Action>,
    event_tx: Sender<comms::Event>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(comms::Action::Connect {
        server_addr,
        credentials,
    }) = action_rx.recv().await
    {
        let stream = match TcpStream::connect(server_addr).await {
            Ok(stream) => stream,
            Err(e) => {
                event_tx.send(comms::Event::Error(e.to_string())).await?;
                return Ok(());
            }
        };

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
        let server_response = serde_json::from_str::<MessageProtocol>(&server_response)?;
        event_tx
            .send(comms::Event::Connected(server_response))
            .await?;

        let mut network_buffer = String::new();
        loop {
            tokio::select! {
                // Handle incoming messages from the server
                res = reader.read_line(&mut network_buffer) => {
                    match res {
                        Ok(0) => {
                            event_tx.send(comms::Event::Error("Server closed connection".to_string())).await?;
                            break;
                        },
                        Ok(_) => {
                            if let Ok(msg) = serde_json::from_str::<MessageProtocol>(&network_buffer) {
                                event_tx.send(comms::Event::ServerMessage(msg)).await?;
                            }
                            break;
                        },
                        Err(e) => {
                            event_tx.send(comms::Event::Error(e.to_string())).await?;
                        },
                    }
                },
                // Handle actions sent by the TUI (sending client's own messages & disconnection)
                Some(action) = action_rx.recv() => {
                    match action {
                        comms::Action::ClientMessage(mut msg) => {
                            msg.push('\n');
                            if writer.write_all(msg.as_bytes()).await.is_err() {
                                event_tx.send(comms::Event::Error("Failed to send message".to_string())).await?;
                                break;
                            }
                        },
                        comms::Action::Disconnect => {
                            break;
                        },
                        _ => {},
                    }
                }
            }
        }
    }
    Ok(())
}
