use std::net::SocketAddr;

use housechat::{client_model::Credentials, protocol::MessageProtocol};
use ratatui::crossterm::event::KeyEvent;

/// This enum defines all the events the networking task can send to the UI loop
#[derive(Debug)]
pub enum Event {
    Tick,
    KeyPress(KeyEvent),
    ServerFound(SocketAddr),
    // TODO maybe remove this
    ServerMessage(MessageProtocol),
    Connected(MessageProtocol),
    Error(String),
}

/// This enum defines all the events the UI can send to the networking task
pub enum Action {
    Connect {
        server_addr: SocketAddr,
        credentials: Credentials,
    },
    ClientMessage(String),
    Disconnect,
}