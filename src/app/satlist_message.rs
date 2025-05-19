use crate::structs::{AppState, CurrentMsg, ListMovement, Message, Model, SatList};
#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::Sender;

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_satlist_msg(model: &mut Model, x: SatList) -> Option<Message> {
    use sky_track::Satellite;
    use tracing::{info, warn};

    use crate::{app::file_cache::cache::cache_tle, structs::TLSatellite};

    use super::file_cache::cache::get_tle_spacetrack;

    match x {
        SatList::AddSatellite => {
            model.current_state = AppState::SatAddition;
            model.sat_config.add_sat.text = "".to_string();
            model.sat_config.add_sat.editing = false;
            None
        }
        SatList::ListMovement(x) => match x {
            ListMovement::Up => {
                model.sat_config.list_state.scroll_up_by(1);
                None
            }
            ListMovement::Down => {
                model.sat_config.list_state.scroll_down_by(1);
                None
            }
            ListMovement::Select => {
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
            _ => None,
        },
        SatList::CopyTLE => {
            if let Some(index) = model.sat_config.list_state.selected() {
                if let Some(x) = model.sat_config.satellite_list.get(index) {
                    match model.sat_config.clipboard.set_text(x.satellite.get_tle()) {
                        Ok(_) => {
                            model.sat_config.current_message =
                                CurrentMsg::message("Copied TLE to clipboard")
                        }
                        Err(_) => {
                            model.sat_config.current_message =
                                CurrentMsg::error("Failed to copy to clipboard!");
                        }
                    }
                }
            }
            None
        }
        SatList::FetchTLE => {
            if let Some(index) = model.sat_config.list_state.selected() {
                if let Some(x) = model.sat_config.satellite_list.get(index) {
                    let new_tle = get_tle_spacetrack(x.satellite.get_norad_id());
                    match new_tle {
                        Ok(y) => {
                            info!("Got TLE from celestrak: {}", y.as_str());
                            let satellite = Satellite::new_from_tle(y.as_str());
                            model.sat_config.satellite_list[index] = TLSatellite {
                                satellite,
                                metadata: x.metadata.clone(),
                            };
                            if let Err(x) = cache_tle(&model.sat_config.satellite_list) {
                                warn!("{}", x);
                                model.sat_config.current_message =
                                    CurrentMsg::error("Failed to cache TLE");
                            }
                            model.current_satellite = None;
                            model.sat_config.current_message = CurrentMsg::message(&format!(
                                "Updated TLE for satellite: {}",
                                model.sat_config.satellite_list[index].satellite.get_name()
                            ));
                        }
                        Err(x) => {
                            warn! {"{}",x};
                            model.sat_config.current_message =
                                CurrentMsg::error("Failed to collect TLE from celestrak");
                        }
                    }
                }
            }
            None
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub fn parse_satlist_msg(
    model: &mut Model,
    x: SatList,
    tx_channel: Sender<Message>,
) -> Option<Message> {
    use ehttp::Request;

    match x {
        SatList::AddSatellite => {
            model.current_state = AppState::SatAddition;
            model.sat_config.add_sat.text = "".to_string();
            model.sat_config.add_sat.editing = false;
            None
        }
        SatList::ListMovement(x) => match x {
            ListMovement::Up => {
                model.sat_config.list_state.scroll_up_by(1);
                None
            }
            ListMovement::Down => {
                model.sat_config.list_state.scroll_down_by(1);
                None
            }
            ListMovement::Select => {
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
            _ => None,
        },
        SatList::CopyTLE => {
            if let Some(index) = model.sat_config.list_state.selected() {
                if let Some(x) = model.sat_config.satellite_list.get(index) {
                    match model.sat_config.clipboard.set_text(x.satellite.get_tle()) {
                        Ok(_) => {
                            model.sat_config.current_message =
                                CurrentMsg::message("Copied TLE to clipboard")
                        }
                        Err(_) => {
                            model.sat_config.current_message =
                                CurrentMsg::error("Failed to copy to clipboard!");
                        }
                    }
                }
            }
            None
        }
        SatList::FetchTLE => {
            if let Some(index) = model.sat_config.list_state.selected() {
                if let Some(x) = model.sat_config.satellite_list.get(index) {
                    let tle_sender = tx_channel.clone();
                    let norad_id_tle = x.satellite.get_norad_id().to_string();
                    ehttp::fetch(
                        Request::get(format!(
                            "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
                            x.satellite.get_norad_id()
                        )),
                        move |x| {
                            if let Ok(y) = x {
                                if y.ok {
                                    match y.text() {
                                        Some(x) => {
                                            tle_sender.send(Message::TLEResponse(
                                                norad_id_tle,
                                                x.to_string(),
                                            ));
                                        }
                                        None => {
                                            tle_sender.send(Message::FetchError(
                                                "Could not parse TLE".to_string(),
                                            ));
                                        }
                                    }
                                } else {
                                    tle_sender
                                        .send(Message::FetchError("Could not get TLE".to_string()));
                                }
                            } else {
                                tle_sender.send(Message::FetchError(
                                    "Could not Access the network".to_string(),
                                ));
                            }
                        },
                    );
                }
            }
            None
        }
    }
}
