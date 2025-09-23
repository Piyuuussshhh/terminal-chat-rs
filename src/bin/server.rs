use local_ip_address::local_ip;
use log::LevelFilter;
use simplelog::{Config, WriteLogger};
use std::{
    fs::File,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream, UdpSocket, tcp::WriteHalf},
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

#[tokio::main]
async fn main() -> terminal_chat::HouseChatResult<()> {
    match init_log() {
        Ok(_) => {}
        Err(e) => panic!("[ERROR] Could not create log file: {e}"),
    }

    // Run the discovery server, so that clients running on different devices in the home network can find the server
    let discovery_handle = tokio::spawn(run_discovery_server());

    let tcp_listener = TcpListener::bind(SERVER_SOCKET).await?;
    log::info!("Server is ready to accept connections on {SERVER_SOCKET}");

    // This only executes if the discovery server crashed
    if discovery_handle.is_finished() {
        // I've made it so that if the discovery server crashes, the whole server crashes
        discovery_handle.await??;
    }

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

fn init_log() -> terminal_chat::HouseChatResult<()> {
    let file = File::create(LOG_FILE)?;
    WriteLogger::init(LevelFilter::Info, Config::default(), file)?;
    Ok(())
}

async fn handle_client(
    mut tcp_stream: TcpStream,
    tx: Sender<MessageProtocol>,
    client_addr: SocketAddr,
) -> terminal_chat::HouseChatResult<()> {
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
) -> terminal_chat::HouseChatResult<()> {
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
) -> terminal_chat::HouseChatResult<()> {
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

async fn run_discovery_server() -> io::Result<()> {
    let socket = UdpSocket::bind(("0:0:0:0:{}", terminal_chat::DISCOVERY_PORT)).await?;
    log::info!(
        "Discovery service listening on port {}",
        terminal_chat::DISCOVERY_PORT
    );

    let server_ip_addr = match local_ip() {
        Ok(addr) => addr,
        Err(e) => {
            log::error!("Failed to get local IP: {}", e);
            return Err(io::Error::new(io::ErrorKind::AddrNotAvailable, e));
        },
    };
    let port = SERVER_SOCKET.port();
    let server_tcp_addr = format!("{}:{}", server_ip_addr, port);

    let mut buf = [0; 1024];

    loop {
        let (len, client_addr) = socket.recv_from(&mut buf).await?;

        if &buf[..len] == terminal_chat::DISCOVERY_MESSAGE {
            log::info!("Replying to discovery message from {}", client_addr);
            socket.send_to(server_tcp_addr.as_bytes(), client_addr).await?;
        }
    }
}
