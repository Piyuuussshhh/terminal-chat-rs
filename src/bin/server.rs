use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use std::{
    error::Error,
    fs::File,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream, tcp::WriteHalf},
    signal,
    sync::broadcast::{self, Sender, error::RecvError},
};
use uuid::Uuid;

use terminal_chat::protocol::MessageProtocol;

const LOG_FILE: &str = "server.log";
const SERVER_CAPACITY: usize = 10;
const SERVER_SOCKET: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080);
const SERVER_ID: Uuid = Uuid::nil();
const SERVER_NAME: &str = "HouseChat";
const SERVER_ADDR: &str = "0.0.0.0:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match init_log() {
        Ok(_) => {}
        Err(e) => panic!("[ERROR] Could not create log file: {e}"),
    }

    let tcp_listener = TcpListener::bind(SERVER_ADDR).await?;
    log::info!("Server is ready to accept connections on {SERVER_ADDR}");

    let (tx, _) = broadcast::channel::<MessageProtocol>(SERVER_CAPACITY);

    loop {
        tokio::select! {
            // Event 1: A new client connects
            Ok((tcp_stream, client_addr)) = tcp_listener.accept() => {
                log::info!("Accepted new connection from {}", client_addr);
                let tx = tx.clone();
                tokio::spawn(async move {
                    match handle_client(tcp_stream, tx, client_addr).await {
                        Ok(_) => log::info!("Client {} handled successfully", client_addr),
                        Err(e) => {
                            log::error!("Client {client_addr} disconnected with an error: {e}");
                        },
                    }
                });
            }

            // Event 2: The Ctrl+C signal is received
            _ = signal::ctrl_c() => {
                log::info!("Shutdown signal received, terminating server.");
                break;
            }
        }
    }

    Ok(())
}

fn init_log() -> Result<(), Box<dyn Error>> {
    let file = File::create(LOG_FILE)?;
    WriteLogger::init(LevelFilter::Info, Config::default(), file)?;
    Ok(())
}

async fn handle_client(
    mut tcp_stream: TcpStream,
    tx: Sender<MessageProtocol>,
    client_addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    log::info!("Handling socket connection from client {}", client_addr);

    let id = Uuid::new_v4();
    let mut rx = tx.subscribe();

    let (reader, writer) = tcp_stream.split();
    let mut reader = BufReader::new(reader);
    let mut writer = BufWriter::new(writer);

    writer
        .write_all(b"Welcome to HouseChat! Please enter your username:\n")
        .await?;
    writer.flush().await?;
    let mut username = String::new();
    reader.read_line(&mut username).await?;
    let username = username.trim().to_string();
    let join_msg = format!("{} has joined the chat!", username);
    log::info!("{}", join_msg);
    if let Err(e) = tx.send(MessageProtocol::new(
        SERVER_SOCKET,
        SERVER_ID,
        SERVER_NAME.to_string(),
        join_msg,
    )) {
        log::warn!("Failed to broadcast join message: {}", e);
    }

    let mut incoming = String::new();

    loop {
        let tx = tx.clone();
        tokio::select! {
            // Either a client receives messages from other clients
            res = rx.recv() => {
                read_channel(res, &mut writer, &id).await?;
            }
            // Or the client sends a message themselves, or the client disconnects
            res = reader.read_line(&mut incoming) => {
                let num_bytes_read = res?;
                if num_bytes_read == 0 {
                    if let Err(e) = tx.send(
                        MessageProtocol::new(
                            SERVER_SOCKET,
                            SERVER_ID,
                            SERVER_NAME.to_string(),
                            format!("{} has left the chat!", username)
                        )
                    ) {
                        log::info!(
                            "Could not broadcast the message '{}': {}",
                            format!("{} has left the chat!", username),
                            e,
                        )
                    }
                    break;
                }
                handle_socket_read(&username, num_bytes_read, &id, &incoming, tx, client_addr).await?;
                incoming.clear();
            }
        }
    }

    Ok(())
}

async fn read_channel(
    res: Result<MessageProtocol, RecvError>,
    writer: &mut BufWriter<WriteHalf<'_>>,
    id: &Uuid,
) -> Result<(), Box<dyn Error>> {
    match res {
        Ok(msg) => {
            log::info!("[{}]: {:?}", id, msg);
            let json = msg.to_json()?;
            writer.write_all(json.as_bytes()).await?;
            writer.flush().await?;
        }
        Err(e) => {
            log::error!("{:?}", e)
        }
    }

    Ok(())
}

async fn handle_socket_read(
    username: &String,
    num_bytes_read: usize,
    id: &Uuid,
    incoming: &String,
    tx: Sender<MessageProtocol>,
    client_addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    log::info!(
        "[{id}]: incoming: {}: size, {num_bytes_read}",
        incoming.trim()
    );

    let outgoing = incoming.trim();

    let _ = tx.send(MessageProtocol::new(
        client_addr,
        id.to_owned(),
        username.to_owned(),
        outgoing.to_string(),
    ));

    log::info!(
        "[{id}]: outgoing: {}: size, {num_bytes_read}",
        outgoing.trim()
    );

    Ok(())
}
