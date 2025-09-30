mod input;
mod networking;
mod ui;

use ratatui::{
    Terminal,
    crossterm::{
        event::{DisableMouseCapture, EnableMouseCapture},
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::CrosstermBackend,
};
use std::{
    error::Error,
    io::{self, Stdout},
    time::Duration,
};
use tokio::sync::mpsc;

use crate::{
    input::input_task,
    networking::{discovery_task, network_task},
    ui::{
        app::{App, CurrentScreen},
        comms,
        screens::ui,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("\n[ERROR] Housechat exited with an error: {err}");
    }

    Ok(())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), Box<dyn Error>> {
    housechat::init_log(housechat::CLIENT_LOG_FILE)?;

    let mut app = App::new(None);
    let (event_tx, mut event_rx) = mpsc::channel::<comms::Event>(100);
    let (action_tx, action_rx) = mpsc::channel::<comms::Action>(100);
    let mut tick_interval = tokio::time::interval(Duration::from_millis(100));

    tokio::spawn(discovery_task(event_tx.clone()));
    input_task(event_tx.clone());
    tokio::spawn(network_task(action_rx, event_tx));

    // Main TUI loop
    loop {
        terminal.draw(|frame| ui(frame, &app))?;

        tokio::select! {
            Some(event) = event_rx.recv() => {
                match event {
                    comms::Event::ServerFound(socket_addr) => {
                        app.server_addr = Some(socket_addr);
                        app.current_screen = CurrentScreen::Signin;
                    },
                    comms::Event::Connected(server_response) => {
                        app.chats.push(server_response);
                        app.current_screen = CurrentScreen::Chat;
                    },
                    comms::Event::KeyPress(key_event) => app.handle_key_event(key_event, action_tx.clone()).await?,
                    comms::Event::ServerMessage(msg) => app.chats.push(msg),
                    comms::Event::Error(e) => app.error_msg = Some(e),
                    _ => {},
                }
            },
            _ = tick_interval.tick() => {
                app.tick();
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
