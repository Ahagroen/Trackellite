#[cfg(not(target_arch = "wasm32"))]
use crate::app::file_cache::cache::{cache_tle, get_sup_data_spacetrack, get_tle_spacetrack};
#[cfg(not(target_arch = "wasm32"))]
use crate::structs::TLSatellite;
use crate::structs::{AddSatMsg, AddSatSel, AppState, CurrentMsg, Message, MetaData, Model};
#[cfg(target_arch = "wasm32")]
use ehttp::Request;
#[cfg(not(target_arch = "wasm32"))]
use ratatui::crossterm::event::KeyCode;

#[cfg(target_arch = "wasm32")]
use ratzilla::event::KeyCode;

use sky_track::Satellite;
#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::Sender;

use tracing::warn;

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_addsat_msg(model: &mut Model, add_sat_msg: AddSatMsg) -> Option<Message> {
    use tracing::info;

    match add_sat_msg {
        AddSatMsg::ToggleEditing => {
            model.sat_config.add_sat.editing = !model.sat_config.add_sat.editing;
            model.sat_config.add_sat.text = "".to_string();
            None
        }
        #[cfg(not(target_arch = "wasm32"))]
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
                        warn!("Couldn't get sup data from spacetrack");
                        model.sat_config.current_message =
                            CurrentMsg::error("Failed to collect TLE from celestrak");
                        return None;
                    }
                } else {
                    model.sat_config.add_sat.editing = false;
                    model.sat_config.add_sat.text = "".to_string();
                    warn!("Couldn't read NORAD ID");
                    model.sat_config.current_message = CurrentMsg::error("Could not read NORAD ID");
                    return None;
                }
            } else {
                satellite = Satellite::new_from_tle(&model.sat_config.add_sat.text);
                let rs_metadata = get_sup_data_spacetrack(&satellite.get_norad_id().to_string());
                if rs_metadata.is_err() {
                    warn!("Couldn't get SUP data from celestrak");
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
                warn!("Unable to cache TLE data");
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
                let x = model.sat_config.clipboard.get_text();
                if x.lines().count() <= 3
                    && x.lines().count() > 0
                    && !x.lines().any(|y| y.len() < 70)
                {
                    model.sat_config.add_sat.editing = true;
                    model.sat_config.add_sat.text = x;
                    model.sat_config.current_message = CurrentMsg::message("Pasted TLE");
                }

                model.sat_config.current_message = CurrentMsg::error("Unable to paste TLE");
                None
            } else {
                None
            }
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub fn parse_addsat_msg(
    model: &mut Model,
    add_sat_msg: AddSatMsg,
    tx_channel: Sender<Message>,
) -> Option<Message> {
    match add_sat_msg {
        AddSatMsg::ToggleEditing => {
            model.sat_config.add_sat.editing = !model.sat_config.add_sat.editing;
            model.sat_config.add_sat.text = "".to_string();
            None
        }
        AddSatMsg::StopEditing => {
            if model.sat_config.add_sat.selected == AddSatSel::NoradID {
                if let Ok(norad_id) = model.sat_config.add_sat.text.parse::<u64>() {
                    let tle_sender = tx_channel.clone();
                    let norad_store = norad_id.to_string();
                    let norad_tle = norad_id.to_string();
                    let norad_mdata = norad_id.to_string();
                    ehttp::fetch(
                        Request::get(format!(
                            "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
                            norad_id
                        )),
                        move |x| {
                            if let Ok(y) = x {
                                if y.ok {
                                    match y.text() {
                                        Some(z) => {
                                            tle_sender
                                                .send(Message::TLEResponse(
                                                    norad_tle,
                                                    z.to_string(),
                                                ))
                                                .unwrap();
                                        }
                                        None => {
                                            tle_sender
                                                .send(Message::FetchError(
                                                    "Could not parse TLE".to_string(),
                                                ))
                                                .unwrap();
                                        }
                                    }
                                } else {
                                    tle_sender
                                        .send(Message::FetchError("Could not get TLE".to_string()))
                                        .unwrap();
                                }
                            } else {
                                tle_sender
                                    .send(Message::FetchError(
                                        "Could not Access the network".to_string(),
                                    ))
                                    .unwrap();
                            }
                        },
                    );
                    let sup_sender = tx_channel.clone();
                    ehttp::fetch(
                        Request::get(format!(
                            "https://celestrak.org/satcat/records.php?CATNR={}",
                            norad_store
                        )),
                        move |x| {
                            if let Ok(y) = x {
                                if y.ok {
                                    match y.json::<Vec<MetaData>>() {
                                        Ok(z) => {
                                            sup_sender
                                                .send(Message::SupDataResponse(
                                                    norad_mdata,
                                                    z[0].clone(),
                                                ))
                                                .unwrap();
                                        }
                                        Err(_) => {
                                            sup_sender
                                                .send(Message::FetchError(
                                                    "Could not parse SUP data".to_string(),
                                                ))
                                                .unwrap();
                                        }
                                    }
                                } else {
                                    sup_sender
                                        .send(Message::FetchError(
                                            "Could not get SUP data".to_string(),
                                        ))
                                        .unwrap();
                                }
                            } else {
                                sup_sender
                                    .send(Message::FetchError(
                                        "Could not Access the network".to_string(),
                                    ))
                                    .unwrap();
                            }
                        },
                    );
                    model.sat_config.add_sat.editing = false;
                } else {
                    model.sat_config.add_sat.editing = false;
                    model.sat_config.add_sat.text = "".to_string();
                    warn!("Couldn't read NORAD ID");
                    model.sat_config.current_message = CurrentMsg::error("Could not read NORAD ID");
                    return None;
                }
            } else {
                let satellite = Satellite::new_from_tle(&model.sat_config.add_sat.text);
                tx_channel
                    .send(Message::TLEResponse(
                        satellite.get_norad_id().to_string(),
                        model.sat_config.add_sat.text.clone(),
                    ))
                    .unwrap();
                let sup_sender = tx_channel.clone();
                let sup_norad = satellite.get_norad_id().to_string();
                ehttp::fetch(
                    Request::get(format!(
                        "https://celestrak.org/satcat/records.php?CATNR={}",
                        satellite.get_norad_id().to_string()
                    )),
                    move |x| {
                        if let Ok(y) = x {
                            if y.ok {
                                match y.json::<Vec<MetaData>>() {
                                    Ok(z) => {
                                        sup_sender
                                            .send(Message::SupDataResponse(sup_norad, z[0].clone()))
                                            .unwrap();
                                    }
                                    Err(_) => {
                                        sup_sender
                                            .send(Message::FetchError(
                                                "Could not parse SUP data".to_string(),
                                            ))
                                            .unwrap();
                                    }
                                }
                            } else {
                                sup_sender
                                    .send(Message::FetchError("Could not get SUP data".to_string()))
                                    .unwrap();
                            }
                        } else {
                            sup_sender
                                .send(Message::FetchError(
                                    "Could not Access the network".to_string(),
                                ))
                                .unwrap();
                        }
                    },
                );
            }
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
                let x = model.sat_config.clipboard.get_text();
                if x.lines().count() <= 3 && x.lines().count() > 0 {
                    if x.lines().find(|y| y.len() < 70).is_none() {
                        model.sat_config.add_sat.editing = true;
                        model.sat_config.add_sat.text = x;
                        model.sat_config.current_message = CurrentMsg::message("Pasted TLE");
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
