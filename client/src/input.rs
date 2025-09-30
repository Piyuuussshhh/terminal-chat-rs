use std::{error::Error, thread};

use ratatui::crossterm::event::{self, Event};
use tokio::sync::mpsc::Sender;
use super::ui::comms;

pub fn input_task(tx: Sender<comms::Event>) {
    thread::spawn(move || -> Result<(), Box<dyn Error + Send + Sync>> {
        loop {
            if let Event::Key(key_event) = event::read()? {
                if tx.blocking_send(comms::Event::KeyPress(key_event)).is_err() {
                    break Ok(());
                }
            }
        }
    });
}