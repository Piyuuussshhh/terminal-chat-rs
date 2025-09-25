use std::{error::Error, io, net::SocketAddr, time::Duration};
use tokio::net::{TcpStream, UdpSocket};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    terminal_chat::init_log(terminal_chat::CLIENT_LOG_FILE)?;

    let server_addr = match find_server().await {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("Error discovering server: {}", e);
            return Ok(());
        }
    };

    let stream = TcpStream::connect(server_addr).await?;

    Ok(())
}

/// Broadcast "I want to connect to the HouseChat server" and the server will reply with its address (ip + port).
async fn find_server() -> io::Result<SocketAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.set_broadcast(true)?;

    // Send discovery message
    let broadcast_addr = format!("255.255.255.255:{}", terminal_chat::DISCOVERY_PORT);
    socket
        .send_to(terminal_chat::DISCOVERY_MESSAGE, broadcast_addr)
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
