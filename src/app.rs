use addsat_message::parse_addsat_msg;
use chrono::{Days, Utc};
use file_cache::cache::cache_gs;
use gs_message::parse_gsconfig_msg;
use satlist_message::parse_satlist_msg;
use sky_track::{GroundStation, find_passes_datetime};
use std::cell::Cell;
use tracing::{debug, info};
mod addsat_message;
mod gs_message;
pub mod key_handle_native;
mod satlist_message;
use crate::structs::{AppState, CurrentMsg, Message, Model, TLPass};
pub mod file_cache;
pub fn update(model: &mut Model, message: Message) {
    let message: Cell<Option<Message>> = Cell::new(Some(message));
    while let Some(msg) = message.take() {
        let msg_inner = msg.clone();
        drop(msg);
        match msg_inner {
            Message::Close => {
                model.exit = true;
            }
            Message::ToggleSatConfig => {
                if !(model.current_state == AppState::SatSelect) {
                    info!("Opening Satellite configuration picker");
                    model.current_state = AppState::SatSelect;
                } else {
                    info!("Closing Satellite configuration picker");
                    model.sat_config.current_message = CurrentMsg {
                        error: false,
                        text: "".to_string(),
                    };
                    model.current_state = AppState::Base;
                    message.set(Some(Message::PropagatePasses))
                }
            }
            Message::SatListMessage(x) => message.set(parse_satlist_msg(model, x)),
            Message::AddSatMessage(add_sat_msg) => {
                message.set(parse_addsat_msg(model, add_sat_msg))
            }
            Message::ToggleGSConfig => {
                if model.current_state != AppState::GSConfig {
                    model.current_state = AppState::GSConfig;
                } else {
                    if cache_gs(model.station_config.station_list.clone()).is_err() {
                        model.station_config.current_msg =
                            CurrentMsg::error("Unable to save Ground Stations");
                    } else {
                        model.current_state = AppState::Base;
                        message.set(Some(Message::PropagatePasses))
                    }
                }
            }
            Message::GSConfigMsg(gsconfig_msg) => {
                message.set(parse_gsconfig_msg(model, gsconfig_msg))
            }
            Message::PropagatePasses => {
                {
                    let current_stations: Vec<GroundStation> = model
                        .station_config
                        .station_list
                        .iter()
                        .filter(|x| x.active)
                        .map(|x| x.station.clone())
                        .collect();
                    let mut passes: Vec<TLPass> = vec![];
                    if current_stations.len() == 0 || model.current_satellite.is_none() {
                    } else {
                        for i in current_stations {
                            passes.append(
                                &mut find_passes_datetime(
                                    &model.current_satellite.as_ref().unwrap().satellite,
                                    &i,
                                    &Utc::now(),
                                    &Utc::now().checked_add_days(Days::new(3)).unwrap(),
                                )
                                .iter()
                                .map(|x| TLPass {
                                    pass: x.clone(),
                                    station: i.clone(),
                                })
                                .collect(),
                            ) //make configurable
                        }
                        for i in &passes {
                            debug!("{:?}", i)
                        }
                    }
                    passes.sort_by(|a, b| a.pass.get_aos().cmp(&b.pass.get_aos()));
                    info!("Updated Passes!");
                    model.upcoming_passes = passes;
                }
            }
        }
    }
}
