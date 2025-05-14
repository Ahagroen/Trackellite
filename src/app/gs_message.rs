use ratatui::crossterm::event::KeyCode;
use sky_track::GroundStation;
use tracing::warn;

use crate::structs::{
    CurrentMsg, GSConfigMsg, GSconfigState, ListMovement, Message, Model, TLGroundStation,
};

pub fn parse_gsconfig_msg(model: &mut Model, gsconfig_msg: GSConfigMsg) -> Option<Message> {
    match gsconfig_msg {
        GSConfigMsg::ListMovement(list_movement) => match list_movement {
            ListMovement::Up => {
                model.station_config.table_state.scroll_up_by(1);
                None
            }
            ListMovement::Down => {
                model.station_config.table_state.scroll_down_by(1);
                None
            }
            ListMovement::Select => {
                match model.station_config.editing {
                    GSconfigState::RowSelect => {
                        if let Some(index) = model.station_config.table_state.selected() {
                            if index == model.station_config.station_list.len() {
                                model.station_config.editing = GSconfigState::CellSelect;
                                model.station_config.station_list.push(TLGroundStation {
                                    station: GroundStation::new([0.0, 0.0, 0.0], ""),
                                    active: false,
                                });
                                model.station_config.table_state.select_first_column();
                                return None; //Maybe a popup to edit? or edit live in the line. worth considering the options.
                            } else if let Some(_) = model.station_config.station_list.get(index) {
                                model.station_config.editing = GSconfigState::CellSelect;
                                model.station_config.table_state.select_first_column();
                                return None;
                            };
                        };
                        None
                    }
                    GSconfigState::CellSelect => {
                        handle_cell_select(model);
                        None
                    }
                    GSconfigState::CellEdit => None,
                }
            }
            ListMovement::Left => {
                model.station_config.table_state.scroll_left_by(1);
                None
            }
            ListMovement::Right => {
                model.station_config.table_state.scroll_right_by(1);
                None
            }
        },
        GSConfigMsg::Back => match model.station_config.editing {
            GSconfigState::RowSelect => Some(Message::ToggleGSConfig),
            GSconfigState::CellSelect => {
                model.station_config.editing = GSconfigState::RowSelect;
                model.station_config.table_state.select_column(None);
                None
            }
            GSconfigState::CellEdit => {
                model.station_config.editing = GSconfigState::CellSelect;
                model.station_config.current_edit_buffer = "".to_string();
                None
            }
        },
        GSConfigMsg::Backspace => {
            if model.station_config.editing == GSconfigState::CellEdit {
                model.station_config.current_edit_buffer.pop();
            }
            None
        }
        GSConfigMsg::StopEditing => handle_stop_editing(model),
        GSConfigMsg::LetterTyped(letter) => {
            if model.station_config.editing == GSconfigState::CellEdit {
                match model.station_config.table_state.selected_column().unwrap() {
                    1 => {
                        if let KeyCode::Char(x) = letter {
                            model.station_config.current_edit_buffer.push(x)
                        }
                    }
                    2..5 => {
                        if let KeyCode::Char(x) = letter {
                            if x.is_numeric() || x == '.' {
                                model.station_config.current_edit_buffer.push(x);
                            } else if x == '-' {
                                if model.station_config.current_edit_buffer.starts_with("-") {
                                    model.station_config.current_edit_buffer.remove(0);
                                } else {
                                    model.station_config.current_edit_buffer.insert(0, '-');
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            None
        }
    }
}

fn handle_stop_editing(model: &mut Model) -> Option<Message> {
    if model.station_config.editing == GSconfigState::CellEdit {
        if let Some((x, y)) = model.station_config.table_state.selected_cell() {
            if y == 1 {
                //editing the name field
                model
                    .station_config
                    .station_list
                    .get_mut(x)
                    .unwrap()
                    .station
                    .name = model.station_config.current_edit_buffer.clone();
                model.station_config.editing = GSconfigState::CellSelect;
                model.station_config.current_edit_buffer = "".to_string();
            } else {
                let value_test = model.station_config.current_edit_buffer.parse::<f64>();
                let value;
                match value_test {
                    Ok(x) => value = x,
                    Err(_) => {
                        model.station_config.current_msg =
                            CurrentMsg::error("Unable to parse value");
                        return None;
                    }
                }
                if y == 2 && (value > 90.0 || value < -90.0) {
                    model.station_config.current_msg =
                        CurrentMsg::error("Latitude Value out of range");
                    return None;
                } else if y == 3 && (value > 180.0 || value < -180.0) {
                    model.station_config.current_msg =
                        CurrentMsg::error("Longitude value out of range");
                    return None;
                }
                match y {
                    2 => {
                        model
                            .station_config
                            .station_list
                            .get_mut(x)
                            .unwrap()
                            .station
                            .lat = value
                    }
                    3 => {
                        model
                            .station_config
                            .station_list
                            .get_mut(x)
                            .unwrap()
                            .station
                            .long = value
                    }
                    4 => {
                        model
                            .station_config
                            .station_list
                            .get_mut(x)
                            .unwrap()
                            .station
                            .alt = value
                    }
                    _ => {}
                };
            }
            if model.station_config.table_state.selected_column().unwrap() < 4 {
                model.station_config.table_state.scroll_right_by(1);
                model.station_config.current_edit_buffer = "".to_string();
            } else {
                model.station_config.editing = GSconfigState::RowSelect;
                model.station_config.current_edit_buffer = "".to_string();
            }
        }
    }
    None
}

fn handle_cell_select(model: &mut Model) {
    if let Some(index) = model.station_config.table_state.selected() {
        if let Some(column) = model.station_config.table_state.selected_column() {
            match column {
                0 => {
                    model
                        .station_config
                        .station_list
                        .get_mut(index)
                        .unwrap()
                        .active = !model.station_config.station_list.get(index).unwrap().active
                }
                1..5 => {
                    model.station_config.editing = GSconfigState::CellEdit;
                }
                _ => {
                    warn!("Tried to edit a column out of range");
                    model.station_config.table_state.select_first_column();
                }
            }
        }
    }
}
