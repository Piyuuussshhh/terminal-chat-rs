use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::net::SocketAddr;
use tokio::sync::mpsc::{self, error::SendError};

use super::comms::Action;
use housechat::{client_model::Credentials, protocol::MessageProtocol};

#[derive(PartialEq)]
pub enum CurrentScreen {
    FindingServer,
    Signin,
    Chat,
}

#[derive(PartialEq)]
pub enum ActiveDataField {
    Username,
    Password,
}

pub struct App {
    pub server_addr: Option<SocketAddr>,
    pub client_msg_input: String,
    pub chats: Vec<MessageProtocol>,
    pub current_screen: CurrentScreen,

    // State required during server finding
    pub spinner: Vec<char>,
    pub spinner_idx: usize,

    // State required during login and register
    pub active_data_field: ActiveDataField,
    pub username_inp: String,
    pub password_inp: String,
    pub error_msg: Option<String>,

    // Flag set if user inputs Ctrl + C
    pub should_quit: bool,
}

impl App {
    pub fn new(server_addr: Option<SocketAddr>) -> Self {
        Self {
            server_addr: server_addr,
            client_msg_input: String::new(),
            chats: Vec::new(),
            current_screen: CurrentScreen::FindingServer,
            spinner: vec!['\\', '|', '/', '-'],
            spinner_idx: 0,
            active_data_field: ActiveDataField::Username,
            username_inp: String::new(),
            password_inp: String::new(),
            error_msg: None,
            should_quit: false,
        }
    }

    pub fn tick(&mut self) {
        self.spinner_idx = (self.spinner_idx + 1) % (self.spinner.len())
    }

    pub async fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
        action_tx: mpsc::Sender<Action>,
    ) -> Result<(), SendError<Action>> {
        // To not false input a character twice
        if key_event.kind != KeyEventKind::Press {
            return Ok(());
        }

        // if input = CTRL + c, then quit
        let to_quit =
            key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('c');
        if to_quit {
            self.should_quit = true;
            action_tx.send(Action::Disconnect).await?;
            return Ok(());
        }

        match self.current_screen {
            CurrentScreen::Signin => self.handle_signin_input(key_event, action_tx).await,
            CurrentScreen::Chat => self.handle_chat_input(key_event, action_tx).await,
            _ => {}
        }

        Ok(())
    }

    pub async fn handle_signin_input(&mut self, key_event: KeyEvent, action_tx: mpsc::Sender<Action>) {
        match key_event.code {
            KeyCode::Enter => {
                // TODO Input validation

                let credentials = Credentials::new(self.username_inp.clone(), self.password_inp.clone());
                // By now, the server address should be found.
                let server_addr = self.server_addr.unwrap();
                if action_tx.send(Action::Connect { server_addr, credentials }).await.is_err() {
                    self.error_msg = Some(String::from("Failed to send connection action to the network task."));
                }
            },
            KeyCode::Char(c) => {
                match self.active_data_field {
                    ActiveDataField::Username => self.username_inp.push(c),
                    ActiveDataField::Password => self.password_inp.push(c),
                }
            },
            KeyCode::Backspace => {
                match self.active_data_field {
                    ActiveDataField::Username => {self.username_inp.pop();},
                    ActiveDataField::Password => {self.password_inp.pop();},
                }
            },
            // Switch between Username and Password fields
            KeyCode::Tab => {
                self.active_data_field = match self.active_data_field {
                    ActiveDataField::Username => ActiveDataField::Password,
                    ActiveDataField::Password => ActiveDataField::Username,
                }
            }
            _ => {},
        }
    }
    
    pub async fn handle_chat_input(&mut self, key_event: KeyEvent, action_tx: mpsc::Sender<Action>) {
        match key_event.code {
            KeyCode::Enter => {
                if !self.client_msg_input.is_empty() {
                    let msg = self.client_msg_input.drain(..).collect::<String>();
                    if action_tx.send(Action::ClientMessage(msg)).await.is_err() {
                        self.error_msg = Some(String::from("Failed to send message."));
                    }
                }
            },
            KeyCode::Char(c) => self.client_msg_input.push(c),
            KeyCode::Backspace => {self.client_msg_input.pop();},
            _ => {},
        }
    }
}
