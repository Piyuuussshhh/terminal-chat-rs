use super::app::{ActiveDataField, App, CurrentScreen};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub fn ui(frame: &mut Frame, app: &App) {
    match app.current_screen {
        CurrentScreen::FindingServer => draw_finding_server_screen(frame, app),
        CurrentScreen::Signin => draw_signin_screen(frame, app),
        CurrentScreen::Chat => draw_chat_screen(frame, app),
    }
}

fn draw_finding_server_screen(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(3),
            Constraint::Percentage(45),
        ])
        .split(frame.area());

    let spinner = app.spinner[app.spinner_idx];

    let text = Paragraph::new(format!(
        "🔎 Searching for server on the network ... {spinner}"
    ))
    .block(Block::default().borders(Borders::ALL).title("Connecting"))
    .style(Style::default().fg(Color::Yellow));

    frame.render_widget(text, chunks[1]);
}

fn draw_signin_screen(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(35), // Title margin
            Constraint::Length(1),      // Title
            Constraint::Length(1),      // Spacer
            Constraint::Length(3),      // Username
            Constraint::Length(3),      // Password
            Constraint::Min(1),         // Spacer
            Constraint::Length(3),      // Error message
        ])
        .split(frame.area());

    let title = Paragraph::new(Text::from("Sign In").bold())
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));

    frame.render_widget(title, chunks[1]);

    let username_block = Block::default().borders(Borders::ALL).title("Username");
    let password_block = Block::default().borders(Borders::ALL).title("Password");
    let username_field = Paragraph::new(app.username_inp.as_str()).block(username_block.clone());
    let password_field =
        Paragraph::new("*".repeat(app.password_inp.len())).block(password_block.clone());

    let mut draw_fields =
        |focus: Paragraph, focus_idx: usize, non_focus: Paragraph, non_focus_idx: usize, inp: &str| {
            frame.render_widget(
                focus.style(Style::default().fg(Color::Yellow)),
                chunks[focus_idx],
            );
            frame.render_widget(non_focus, chunks[non_focus_idx]);
            frame.set_cursor_position((
                chunks[focus_idx].x + inp.len() as u16 + 1,
                chunks[focus_idx].y + 1,
            ));
        };

    // Highlighting the active block
    match app.active_data_field {
        ActiveDataField::Username => draw_fields(username_field, 3, password_field, 4, &app.username_inp),
        ActiveDataField::Password => draw_fields(password_field, 4, username_field, 3, &app.password_inp),
    }

    if let Some(error) = &app.error_msg {
        let error_widget = Paragraph::new(error.as_str())
            .block(Block::default().borders(Borders::ALL).title("Error"))
            .style(Style::default().fg(Color::Red));
        frame.render_widget(error_widget, chunks[6]);
    }
}

fn draw_chat_screen(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(frame.area());

    let msgs = app
        .chats
        .iter()
        .map(|msg| {
            Line::from(Span::raw(format!(
                "[{}]: {}",
                msg.sender_username, msg.payload
            )))
        })
        .collect::<Vec<Line>>();

    let msgs_list = Paragraph::new(msgs)
        .block(Block::default().borders(Borders::ALL).title("Chat"))
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    frame.render_widget(msgs_list, chunks[0]);

    let input_field = Paragraph::new(app.client_msg_input.as_str())
        .block(Block::default().borders(Borders::ALL).title("Chat"))
        .style(Style::default().fg(Color::White));
    frame.render_widget(input_field, chunks[1]);
    frame.set_cursor_position((
        chunks[1].x + app.client_msg_input.len() as u16 + 1,
        chunks[1].y + 1,
    ));
}
