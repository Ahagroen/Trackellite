use crate::app::file_cache::{cache_tle, get_sup_data_spacetrack, get_tle_spacetrack};
use crate::structs::{
    AddSatMsg, AddSatSel, AppState, CurrentMsg, Message, MetaData, Model, TLSatellite,
};
use ratatui::crossterm::event::KeyCode;
use sky_track::Satellite;
use tracing::{info, warn};

pub fn parse_addsat_msg(model: &mut Model, add_sat_msg: AddSatMsg) -> Option<Message> {
    match add_sat_msg {
        AddSatMsg::ToggleEditing => {
            model.sat_config.add_sat.editing = !model.sat_config.add_sat.editing;
            model.sat_config.add_sat.text = "".to_string();
            None
        }
        AddSatMsg::StopEditing => {
            let satellite: Satellite;
            let metadata: MetaData;
            if model.sat_config.add_sat.selected == AddSatSel::NoradID {
                if let Ok(x) = model.sat_config.add_sat.text.parse::<u64>() {
                    if let Ok(y) = get_tle_spacetrack(x) {
                        info!("Got TLE from spacetrack:{}", y.as_str());
                        let sup_data = get_sup_data_spacetrack(&x.to_string());
                        if let Ok(z) = sup_data {
                            info!("Got Sup data from spacetrack:{:?}", z);
                            metadata = z;
                        } else {
                            warn!("{}", sup_data.unwrap_err());
                            model.sat_config.current_message =
                                CurrentMsg::error("Failed to collect SUP Data from celestrak");
                            return None;
                        }
                        satellite = Satellite::new_from_tle(y.as_str())
                    } else {
                        model.sat_config.current_message =
                            CurrentMsg::error("Failed to collect TLE from celestrak");
                        return None;
                    }
                } else {
                    model.sat_config.add_sat.editing = false;
                    model.sat_config.add_sat.text = "".to_string();
                    model.sat_config.current_message = CurrentMsg::error("Could not read NORAD ID");
                    return None;
                }
            } else {
                satellite = Satellite::new_from_tle(&model.sat_config.add_sat.text);
                let rs_metadata = get_sup_data_spacetrack(&satellite.get_norad_id().to_string());
                if rs_metadata.is_err() {
                    model.sat_config.current_message =
                        CurrentMsg::error("Failed to collect SUP Data from celestrak");
                    return None;
                } else {
                    metadata = rs_metadata.unwrap();
                }
            }
            model.sat_config.satellite_list.push(TLSatellite {
                satellite,
                metadata,
            });
            if cache_tle(model.sat_config.satellite_list.as_ref()).is_err() {
                model.sat_config.current_message = CurrentMsg::error("Unable to cache TLE data");
                return None;
            }
            model.sat_config.add_sat.editing = false;
            model.current_state = AppState::SatSelect;
            model.sat_config.current_message = CurrentMsg::message("Loaded Satellite");
            None
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
        AddSatMsg::LetterTyped(letter) => match model.sat_config.add_sat.selected {
            AddSatSel::TLEBox => {
                if let KeyCode::Char(x) = letter {
                    model.sat_config.add_sat.text.push(x);
                    None
                } else {
                    None
                }
            }
            AddSatSel::NoradID => {
                if model.sat_config.add_sat.text.len() >= 5 {
                    return None;
                }
                if let KeyCode::Char(x) = letter {
                    if x.is_numeric() || x == '.' {
                        model.sat_config.add_sat.text.push(x);
                    }
                }
                None
            }
        },
        AddSatMsg::Backspace => {
            model.sat_config.add_sat.text.pop();
            None
        }
        AddSatMsg::PasteTLE => {
            if model.sat_config.add_sat.selected == AddSatSel::TLEBox {
                if let Ok(x) = model.sat_config.clipboard.get_text() {
                    if x.lines().count() <= 3 {
                        if x.lines().find(|y| y.len() < 70).is_none() {
                            model.sat_config.add_sat.editing = true;
                            model.sat_config.add_sat.text = x;
                            model.sat_config.current_message = CurrentMsg::message("Pasted TLE");
                        }
                    }
                }

                model.sat_config.current_message = CurrentMsg::error("Unable to paste TLE");
                None
            } else {
                None
            }
        }
    }
}
