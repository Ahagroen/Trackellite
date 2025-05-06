use crate::{
    AddSatMsg, AddSatSel, AppState, CurrentMsgSatSel, Message, Model, SatList, utils::get_data_dir,
};
use color_eyre::{Result, owo_colors::OwoColorize};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers};
use serde_json::{Map, Value, from_reader, to_writer};
use sky_track::Satellite;
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    time::Duration,
};
use tracing::info;
use ureq::get;
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
        Message::SatListMessage(x) => match x {
            SatList::AddSatellite => {
                model.current_state = AppState::SatAddition;
                None
            }
            SatList::Up => {
                model.sat_config.list_state.scroll_up_by(1);
                None
            }
            SatList::Down => {
                model.sat_config.list_state.scroll_down_by(1);
                None
            }
            SatList::Select => {
                if let Some(index) = model.sat_config.list_state.selected() {
                    if index == model.sat_config.satellite_list.len() {
                        return Some(Message::SatListMessage(SatList::AddSatellite));
                    } else if let Some(x) = model.sat_config.satellite_list.get(index) {
                        model.current_satellite = Some(x.clone());
                        return Some(Message::ToggleSatConfig);
                    };
                };
                None
            }
            SatList::CopyTLE => {
                if let Some(index) = model.sat_config.list_state.selected() {
                    if let Some(x) = model.sat_config.satellite_list.get(index) {
                        match model.sat_config.clipboard.set_text(x.get_tle()) {
                            Ok(_) => {
                                return Some(Message::SatListMessage(SatList::UpdateMessage(
                                    CurrentMsgSatSel {
                                        error: false,
                                        text: "Copied TLE to clipboard".to_string(),
                                    },
                                )));
                            }
                            Err(_) => {
                                return Some(Message::SatListMessage(SatList::UpdateMessage(
                                    CurrentMsgSatSel {
                                        error: true,
                                        text: "Failed to copy to clipboard!".to_string(),
                                    },
                                )));
                            }
                        }
                    }
                }
                None
            }
            SatList::FetchTLE => {
                if let Some(index) = model.sat_config.list_state.selected() {
                    if let Some(x) = model.sat_config.satellite_list.get(index) {
                        let new_tle = get_tle_spacetrack(x.get_norad_id());
                        match new_tle {
                            Ok(y) => match cache_tle(x.get_norad_id().to_string(), y.as_str()) {
                                Ok(_) => {
                                    info!("Got TLE from celestrak: {}", y.as_str());
                                    let satellite = Satellite::new_from_tle(y.as_str());
                                    cache_tle(
                                        satellite.get_norad_id().to_string(),
                                        satellite.get_tle(),
                                    )
                                    .unwrap();
                                    model.sat_config.satellite_list[index] = satellite;
                                    return Some(Message::SatListMessage(SatList::UpdateMessage(
                                        CurrentMsgSatSel {
                                            error: false,
                                            text: format!(
                                                "Updated TLE for satellite: {}",
                                                model.sat_config.satellite_list[index].get_name()
                                            ),
                                        },
                                    )));
                                }
                                Err(_) => {
                                    return Some(Message::SatListMessage(SatList::UpdateMessage(
                                        CurrentMsgSatSel {
                                            error: true,
                                            text: "Failed to cache TLE".to_string(),
                                        },
                                    )));
                                }
                            },
                            Err(_) => {
                                return Some(Message::SatListMessage(SatList::UpdateMessage(
                                    CurrentMsgSatSel {
                                        error: true,
                                        text: "Failed to collect TLE from celestrak".to_string(),
                                    },
                                )));
                            }
                        }
                    }
                }
                None
            }
            SatList::UpdateMessage(x) => {
                model.sat_config.current_message = x;
                None
            }
        },
        Message::AddSatMessage(add_sat_msg) => match add_sat_msg {
            AddSatMsg::StartEditing => {
                model.sat_config.add_sat.editing = true;
                None
            }
            AddSatMsg::StopEditing => {
                let satellite: Satellite;
                if model.sat_config.add_sat.selected == AddSatSel::NoradID {
                    if let Ok(x) = model.sat_config.add_sat.text.parse::<u64>() {
                        if let Ok(y) = get_tle_spacetrack(x) {
                            info!("Got TLE from spacetrack:{}", y.as_str());
                            satellite = Satellite::new_from_tle(y.as_str())
                        } else {
                            return Some(Message::SatListMessage(SatList::UpdateMessage(
                                CurrentMsgSatSel {
                                    error: true,
                                    text: "Failed to collect TLE from celestrak".to_string(),
                                },
                            )));
                        }
                    } else {
                        model.sat_config.add_sat.editing = false;
                        model.sat_config.add_sat.text = "".to_string();
                        return Some(Message::SatListMessage(SatList::UpdateMessage(
                            CurrentMsgSatSel {
                                error: true,
                                text: "Could not read NORAD ID".to_string(),
                            },
                        )));
                    }
                } else {
                    satellite = Satellite::new_from_tle(&model.sat_config.add_sat.text);
                }
                model.sat_config.add_sat.editing = false;
                model.current_state = AppState::SatSelect;
                cache_tle(satellite.get_norad_id().to_string(), satellite.get_tle()).unwrap();
                model.sat_config.satellite_list.push(satellite);
                return Some(Message::SatListMessage(SatList::UpdateMessage(
                    CurrentMsgSatSel {
                        error: false,
                        text: "Loaded Satellite".to_string(),
                    },
                )));
            }
            AddSatMsg::ChangeSelection => match model.sat_config.add_sat.selected {
                AddSatSel::NoradID => {
                    model.sat_config.add_sat.selected = AddSatSel::TLEBox;
                    None
                }
                AddSatSel::TLEBox => {
                    model.sat_config.add_sat.selected = AddSatSel::NoradID;
                    None
                }
            },
            AddSatMsg::LetterTyped(x) => match model.sat_config.add_sat.selected {
                AddSatSel::TLEBox => {
                    model
                        .sat_config
                        .add_sat
                        .text
                        .push(x.chars().next().unwrap());
                    None
                }
                AddSatSel::NoradID => {
                    if model.sat_config.add_sat.text.len() >= 5 {
                        return None;
                    }
                    let char = x.chars().next().unwrap();
                    if char.is_numeric() {
                        model.sat_config.add_sat.text.push(char);
                    }
                    None
                }
            },
            AddSatMsg::Backspace => {
                model.sat_config.add_sat.text.pop();
                None
            }
        },
    }
}

pub fn handle_event(model: &Model) -> Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match model.current_state {
                    AppState::Base => return Ok(handle_key_base(key)),
                    AppState::SatSelect => return Ok(handle_key_sat_config(key)),
                    AppState::SatAddition => return Ok(handle_key_sat_addition(key, model)),
                }
            }
        }
    }
    Ok(None)
}

fn handle_key_sat_config(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Message::ToggleSatConfig),
        KeyCode::Char('c') => Some(Message::SatListMessage(SatList::CopyTLE)),
        KeyCode::Char('f') => Some(Message::SatListMessage(SatList::FetchTLE)),
        KeyCode::Up => Some(Message::SatListMessage(SatList::Up)),
        KeyCode::Down => Some(Message::SatListMessage(SatList::Down)),
        KeyCode::Enter => Some(Message::SatListMessage(SatList::Select)),
        _ => None,
    }
}
fn handle_key_base(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Some(Message::Close),
        KeyCode::Char('s') => Some(Message::ToggleSatConfig),
        _ => None,
    }
}

fn handle_key_sat_addition(key: event::KeyEvent, model: &Model) -> Option<Message> {
    if !model.sat_config.add_sat.editing {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::ToggleSatConfig),
            KeyCode::Enter => Some(Message::AddSatMessage(AddSatMsg::StartEditing)),
            KeyCode::Up | KeyCode::Down => Some(Message::AddSatMessage(AddSatMsg::ChangeSelection)),
            _ => None,
        }
    } else {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => Some(Message::ToggleSatConfig),
            KeyCode::Backspace => Some(Message::AddSatMessage(AddSatMsg::Backspace)),
            KeyCode::Enter => Some(Message::AddSatMessage(AddSatMsg::StopEditing)),
            _ => Some(Message::AddSatMessage(AddSatMsg::LetterTyped(
                key.code.to_string(),
            ))),
        }
    }
}

fn cache_tle(norad_id: String, tle: &str) -> Result<()> {
    let mut cache_data = get_tle_cache()?;
    let _ = cache_data.insert(norad_id, Value::String(tle.to_string()));
    let mut tle_file = get_data_dir();
    tle_file.push("tle.json");
    let file = File::create(tle_file)?;
    let writer = BufWriter::new(file);
    info!("Writing TLE cache: {:?}", &cache_data);
    to_writer(writer, &cache_data)?;
    Ok(())
}

fn get_tle_spacetrack(norad_id: u64) -> Result<String> {
    let response = get(format!(
        "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
        norad_id.to_string()
    ))
    .call()?;
    Ok(response.into_body().read_to_string()?)
}

pub fn get_tle_cache() -> Result<Map<String, Value>> {
    let mut data_dir = get_data_dir();
    data_dir.push("tle.json");
    if data_dir.try_exists()? {
        let file = File::open(data_dir)?;
        let reader = BufReader::new(file);
        let json: Value = from_reader(reader)?;
        Ok(json.as_object().unwrap().clone())
    } else {
        let file = File::create_new(data_dir)?;
        let writer = BufWriter::new(file);
        to_writer(writer, &Map::new())?;
        Ok(Map::new())
    }
}
