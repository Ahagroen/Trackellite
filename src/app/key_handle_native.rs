#[cfg(not(target_arch = "wasm32"))]
use ratatui::crossterm::event::{self, Event, KeyCode};

#[cfg(target_arch = "wasm32")]
use ratzilla::event::{self, KeyCode, KeyEvent};

use crate::structs::AddSatMsg;
use crate::structs::AppState;
use crate::structs::GSConfigMsg;
use crate::structs::GSconfigState;
use crate::structs::ListMovement;
use crate::structs::Message;
use crate::structs::Model;
use crate::structs::SatList;
use color_eyre::Result;

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_event(model: &Model) -> Result<Option<Message>> {
    use chrono::Utc;

    use chrono::TimeDelta;
    use tracing::info;

    use std::time::Duration;

    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match model.current_state {
                    AppState::Base => return Ok(handle_key_base(key)),
                    AppState::SatSelect => return Ok(handle_key_sat_config(key)),
                    AppState::SatAddition => {
                        return Ok(handle_key_sat_addition(key, model));
                    }
                    AppState::GSConfig => return Ok(handle_key_gs_config(key, model)),
                }
            }
        }
    } else if let Some(x) = model.upcoming_passes.first() {
        if x.pass.get_los_datetime().signed_duration_since(Utc::now())
            < TimeDelta::new(-30, 0).unwrap()
        {
            info!("Re-propagating to remove old pass");
            return Ok(Some(Message::PropagatePasses));
        }
    }
    Ok(None)
}
#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::Sender;
#[cfg(target_arch = "wasm32")]
pub fn handle_event(model: &Model, key_event: KeyEvent, tx: Sender<Message>) {
    let message;
    match model.current_state {
        AppState::Base => message = handle_key_base(key_event),
        AppState::SatSelect => message = handle_key_sat_config(key_event),
        AppState::SatAddition => {
            message = handle_key_sat_addition(key_event, &model);
        }
        AppState::GSConfig => message = handle_key_gs_config(key_event, &model),
        AppState::SatWaitingFetch => message = None,
    }
    if let Some(x) = message {
        tx.send(x).unwrap()
    }
}

fn handle_key_gs_config(key: event::KeyEvent, model: &Model) -> Option<Message> {
    match model.station_config.editing {
        GSconfigState::CellSelect => match key.code {
            KeyCode::Left => Some(Message::GSConfigMsg(GSConfigMsg::ListMovement(
                ListMovement::Left,
            ))),
            KeyCode::Right => Some(Message::GSConfigMsg(GSConfigMsg::ListMovement(
                ListMovement::Right,
            ))),
            KeyCode::Enter => Some(Message::GSConfigMsg(GSConfigMsg::ListMovement(
                ListMovement::Select,
            ))),
            KeyCode::Esc | KeyCode::Char('q') => Some(Message::GSConfigMsg(GSConfigMsg::Back)),
            _ => None,
        },
        GSconfigState::RowSelect => match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::ToggleGSConfig),
            KeyCode::Up => Some(Message::GSConfigMsg(GSConfigMsg::ListMovement(
                ListMovement::Up,
            ))),
            KeyCode::Down => Some(Message::GSConfigMsg(GSConfigMsg::ListMovement(
                ListMovement::Down,
            ))),
            KeyCode::Enter => Some(Message::GSConfigMsg(GSConfigMsg::ListMovement(
                ListMovement::Select,
            ))),
            _ => None,
        },
        GSconfigState::CellEdit => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Some(Message::GSConfigMsg(GSConfigMsg::Back)),
            KeyCode::Backspace => Some(Message::GSConfigMsg(GSConfigMsg::Backspace)),
            KeyCode::Enter => Some(Message::GSConfigMsg(GSConfigMsg::StopEditing)),
            _ => Some(Message::GSConfigMsg(GSConfigMsg::LetterTyped(key.code))),
        },
    }
}

fn handle_key_sat_config(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Message::ToggleSatConfig),
        KeyCode::Char('c') => Some(Message::SatListMessage(SatList::CopyTLE)),
        KeyCode::Char('f') => Some(Message::SatListMessage(SatList::FetchTLE)),
        KeyCode::Up => Some(Message::SatListMessage(SatList::ListMovement(
            ListMovement::Up,
        ))),
        KeyCode::Down => Some(Message::SatListMessage(SatList::ListMovement(
            ListMovement::Down,
        ))),
        KeyCode::Enter => Some(Message::SatListMessage(SatList::ListMovement(
            ListMovement::Select,
        ))),
        _ => None,
    }
}

fn handle_key_base(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Message::Close),
        KeyCode::Char('s') => Some(Message::ToggleSatConfig),
        KeyCode::Char('g') => Some(Message::ToggleGSConfig),
        _ => None,
    }
}

fn handle_key_sat_addition(key: event::KeyEvent, model: &Model) -> Option<Message> {
    if !model.sat_config.add_sat.editing {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::ToggleSatConfig),
            KeyCode::Enter => Some(Message::AddSatMessage(AddSatMsg::ToggleEditing)),
            KeyCode::Up | KeyCode::Down => Some(Message::AddSatMessage(AddSatMsg::ChangeSelection)),
            KeyCode::Char('v') => Some(Message::AddSatMessage(AddSatMsg::PasteTLE)),
            _ => None,
        }
    } else {
        match key.code {
            KeyCode::Esc => Some(Message::AddSatMessage(AddSatMsg::ToggleEditing)),
            KeyCode::Backspace => Some(Message::AddSatMessage(AddSatMsg::Backspace)),
            KeyCode::Enter => Some(Message::AddSatMessage(AddSatMsg::StopEditing)),
            _ => Some(Message::AddSatMessage(AddSatMsg::LetterTyped(key.code))),
        }
    }
}
