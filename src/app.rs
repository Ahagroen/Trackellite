use crate::{
    AddSatMsg, AddSatSel, AppState, CurrentMsg, GSConfigMsg, GSconfigState, ListMovement, Message,
    MetaData, Model, SatList, TLGroundStation, TLPass, TLSatellite, utils::get_data_dir,
};
use chrono::{Days, TimeDelta, Utc};
use color_eyre::Result;
use ratatui::crossterm::event::{self, Event, KeyCode};
use serde_json::{from_reader, to_string, to_writer};
use sky_track::{GroundStation, Satellite, find_passes_datetime};
use std::{
    cell::Cell,
    collections::HashMap,
    fs::File,
    io::{BufReader, BufWriter},
    time::Duration,
};
use tracing::{debug, info, warn};
use ureq::get;
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

fn parse_gsconfig_msg(model: &mut Model, gsconfig_msg: GSConfigMsg) -> Option<Message> {
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
                        if let Some(index) = model.station_config.table_state.selected() {
                            if let Some(column) = model.station_config.table_state.selected_column()
                            {
                                match column {
                                    0 => {
                                        model
                                            .station_config
                                            .station_list
                                            .get_mut(index)
                                            .unwrap()
                                            .active = !model
                                            .station_config
                                            .station_list
                                            .get(index)
                                            .unwrap()
                                            .active
                                    }
                                    1..5 => {
                                        model.station_config.editing = GSconfigState::CellEdit;
                                    }
                                    _ => {
                                        warn!("Tried to edit a column out of range");
                                        model.station_config.table_state.select_first_column();
                                        return None;
                                    }
                                }
                            }
                        }
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
                None
            } else {
                None
            }
        }
        GSConfigMsg::StopEditing => {
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

                    None
                } else {
                    None
                }
            } else {
                None
            }
        }
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
                None
            } else {
                None
            }
        }
    }
}

fn parse_addsat_msg(model: &mut Model, add_sat_msg: AddSatMsg) -> Option<Message> {
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

fn parse_satlist_msg(model: &mut Model, x: SatList) -> Option<Message> {
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

pub fn handle_event(model: &Model) -> Result<Option<Message>> {
    if event::poll(Duration::from_millis(250))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match model.current_state {
                    AppState::Base => return Ok(handle_key_base(key)),
                    AppState::SatSelect => return Ok(handle_key_sat_config(key)),
                    AppState::SatAddition => return Ok(handle_key_sat_addition(key, &model)),
                    AppState::GSConfig => return Ok(handle_key_gs_config(key, &model)),
                }
            }
        }
    } else if let Some(x) = model.upcoming_passes.get(0) {
        if x.pass.get_los_datetime().signed_duration_since(Utc::now())
            < TimeDelta::new(-1800, 0).unwrap()
        {
            info!("Re-propagating to remove old pass");
            return Ok(Some(Message::PropagatePasses));
        }
    }
    Ok(None)
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

fn cache_gs(data: Vec<TLGroundStation>) -> Result<()> {
    let mut cached_data = get_gs_cache()?;
    for i in data {
        cached_data.insert(i.station.name.clone(), to_string(&i)?);
    }
    let mut gs_file = get_data_dir();
    gs_file.push("gs.json");
    let file = File::create(gs_file)?;
    let writer = BufWriter::new(file);
    info!("Writing TLE cache: {:?}", &cached_data);
    to_writer(writer, &cached_data)?;
    Ok(())
}

fn cache_tle(data: &Vec<TLSatellite>) -> Result<()> {
    let cache_result = get_sat_cache();
    let mut cache_data;
    if cache_result.is_err() {
        cache_data = HashMap::new();
    } else {
        cache_data = cache_result.unwrap();
    }
    for i in data {
        cache_data.insert(i.satellite.get_norad_id().to_string(), to_string(&i)?);
    }
    let mut tle_file = get_data_dir();
    tle_file.push("tle.json");
    let file = File::create(tle_file)?;
    let writer = BufWriter::new(file);
    info!("Writing TLE cache: {:?}", &cache_data);
    to_writer(writer, &cache_data)?;
    Ok(())
}

fn get_sup_data_spacetrack(norad_id: &str) -> Result<MetaData> {
    let response = get(format!(
        "https://celestrak.org/satcat/records.php?CATNR={}",
        norad_id
    ))
    .call()?;
    let response_lose: Vec<MetaData> = response.into_body().read_json()?;
    debug!("{:?}", response_lose);
    Ok(response_lose[0].clone())
}

fn get_tle_spacetrack(norad_id: u64) -> Result<String> {
    let response = get(format!(
        "https://celestrak.org/NORAD/elements/gp.php?CATNR={}&FORMAT=TLE",
        norad_id
    ))
    .call()?;
    Ok(response.into_body().read_to_string()?)
}

pub fn get_sat_cache() -> Result<HashMap<String, String>> {
    get_cache_file("tle.json")
}

pub fn get_gs_cache() -> Result<HashMap<String, String>> {
    get_cache_file("gs.json")
}

fn get_cache_file(filename: &str) -> Result<HashMap<String, String>> {
    let mut data_dir = get_data_dir();
    data_dir.push(filename);
    if data_dir.try_exists()? {
        let file = File::open(data_dir)?;
        let reader = BufReader::new(file);
        Ok(from_reader(reader)?)
    } else {
        let file = File::create_new(data_dir)?;
        let writer = BufWriter::new(file);
        to_writer(writer, &HashMap::<String, String>::new())?;
        Ok(HashMap::new())
    }
}
