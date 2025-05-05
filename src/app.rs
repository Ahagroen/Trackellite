use std::time::Duration;

use crate::{AppState, Message, Model};
use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode};
use tracing::info;
pub fn update(model: &mut Model, message: Message) -> Option<Message> {
    match message {
        Message::Close => {
            model.exit = true;
            None
        }
        Message::ToggleSatConfig => {
            if !(model.current_state == AppState::SatSelect) {
                info!("Opening Satellite configuration picker");
                model.current_state = AppState::SatSelect;
            } else {
                info!("Closing Satellite configuration picker");
                model.current_state = AppState::Base;
            }
            None
        }
        Message::AddSatellite => todo!(),
        Message::SatListUp => {
            model.sat_config.list_state.scroll_up_by(1);
            None
        }
        Message::SatListDown => {
            model.sat_config.list_state.scroll_down_by(1);
            None
        }
        Message::SatListSelect => {
            if let Some(index) = model.sat_config.list_state.selected() {
                if index == model.sat_config.satellite_list.len() {
                    return Some(Message::AddSatellite);
                } else if let Some(x) = model.sat_config.satellite_list.get(index) {
                    model.current_satellite = Some(x.clone())
                };
            };
            None
        }
    }
    //Only updates are Adding/Removing satellites or ground stations, everything else is derived from the view and rendered on demand?
}

pub fn handle_event(model: &Model) -> Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match model.current_state {
                    AppState::Base => return Ok(handle_key_base(key)),
                    AppState::SatSelect => return Ok(handle_key_sat_config(key)),
                    AppState::SatAddition => todo!(),
                }
            }
        }
    }
    Ok(None)
}

fn handle_key_sat_config(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Close),
        KeyCode::Char('c') => Some(Message::ToggleSatConfig),
        KeyCode::Char('a') => Some(Message::AddSatellite),
        KeyCode::Up => Some(Message::SatListUp),
        KeyCode::Down => Some(Message::SatListDown),
        KeyCode::Enter => Some(Message::SatListSelect),
        _ => None,
    }
}
fn handle_key_base(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') => Some(Message::Close),
        KeyCode::Char('s') => Some(Message::ToggleSatConfig),
        _ => None,
    }
}
