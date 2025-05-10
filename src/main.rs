use std::process::exit;

use app::{get_gs_cache, get_sat_cache, handle_event, update};
use arboard::Clipboard;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use color_eyre::{Result, eyre::Error, owo_colors::OwoColorize};
use ratatui::widgets::ListState;
use serde_json::{Map, Value};
use sky_track::{GroundStation, Satellite};
use ui::view;
use utils::initialize_logging;
mod app;
mod ui;
mod utils;
use tracing::{debug, info, warn};

fn main() -> Result<()> {
    initialize_logging()?;
    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let mut model = Model::default();
    info!("Loaded Model");
    while !model.exit {
        terminal.draw(|f| view(&mut model, f))?;
        let mut current_msg = handle_event(&model)?;
        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }
    ratatui::restore();
    Ok(())
}
enum SatList {
    Up,
    Down,
    Select,
    CopyTLE,
    FetchTLE,
    AddSatellite,
    UpdateMessage(CurrentMsgSatSel),
}
enum AddSatMsg {
    StartEditing,
    StopEditing,
    ChangeSelection,
    LetterTyped(String),
    Backspace,
    PasteTLE,
}
#[derive(PartialEq, Debug)]
enum AddSatSel {
    NoradID,
    TLEBox,
}
struct AddSatState {
    selected: AddSatSel,
    text: String,
    editing: bool,
}
impl Default for AddSatState {
    fn default() -> Self {
        AddSatState {
            selected: AddSatSel::NoradID,
            text: String::default(),
            editing: false,
        }
    }
}
enum Message {
    Close,
    ToggleSatConfig,
    SatListMessage(SatList),
    AddSatMessage(AddSatMsg),
    ToggleGSConfig,
}

struct CurrentMsgSatSel {
    error: bool,
    text: String,
}
impl CurrentMsgSatSel {
    fn error(msg: &str) -> Option<Message> {
        return Some(Message::SatListMessage(SatList::UpdateMessage(
            CurrentMsgSatSel {
                error: true,
                text: msg.to_string(),
            },
        )));
    }
    fn message(msg: &str) -> Option<Message> {
        return Some(Message::SatListMessage(SatList::UpdateMessage(
            CurrentMsgSatSel {
                error: false,
                text: msg.to_string(),
            },
        )));
    }
}
struct GSconfiguration {
    station_list: Vec<GroundStation>,
    list_state: ListState,
}
impl GSconfiguration {
    fn load() -> GSconfiguration {
        let cached_gs = get_gs_cache().unwrap();
        debug!("Cached GS': {:?}", cached_gs);
        let mut stations: Vec<GroundStation> = vec![];
        for (key, value) in cached_gs.iter() {
            let value_map = value.as_object().unwrap();
            stations.push(GroundStation {
                lat: value_map.get("lat").unwrap().as_f64().unwrap(),
                long: value_map.get("long").unwrap().as_f64().unwrap(),
                alt: value_map.get("alt").unwrap().as_f64().unwrap(),
                name: key.to_owned(),
            })
        }
        GSconfiguration {
            station_list: stations,
            list_state: ListState::default(),
        }
    }
}

struct SatSelection {
    satellite_list: Vec<TLSatellite>,
    list_state: ListState,
    clipboard: Clipboard,
    current_message: CurrentMsgSatSel,
    add_sat: AddSatState,
}
impl Default for SatSelection {
    fn default() -> Self {
        let sat_list = Self::load_sat_from_file();
        let current_message;
        let satellites;
        if sat_list.is_err() {
            warn!("Unable to Load satellites from file");
            current_message = CurrentMsgSatSel {
                error: true,
                text: sat_list.err().unwrap().to_string(),
            };
            satellites = vec![];
        } else {
            current_message = CurrentMsgSatSel {
                error: false,
                text: "".to_string(),
            };
            satellites = sat_list.unwrap();
        }
        SatSelection {
            satellite_list: satellites,
            list_state: ListState::default(),
            clipboard: Clipboard::new().expect("Unable to access clipboard"),
            current_message,
            add_sat: AddSatState::default(),
        }
    }
}
impl SatSelection {
    fn load_sat_from_file() -> Result<Vec<TLSatellite>> {
        let cached_tles = get_sat_cache().unwrap();
        debug!("Cached TLE's: {:?}", cached_tles);
        let mut satellites = vec![];
        for i in cached_tles.values() {
            let satellite = Satellite::new_from_tle(
                i.get("tle")
                    .and_then(|x| x.as_str())
                    .ok_or_else(|| Error::msg("Could Not parse tle section of sat.json"))?,
            );
            let metadata = MetaData::from_string(
                i.get("metadata")
                    .and_then(|x| x.as_object())
                    .ok_or_else(|| Error::msg("Could not parse metadata section of sat.json"))?,
            )?;
            satellites.push(TLSatellite {
                satellite,
                metadata,
            });
        }
        Ok(satellites)
    }
}
#[derive(Debug, PartialEq)]
enum AppState {
    Base,
    SatSelect,
    SatAddition,
    GSConfig,
}

struct Model {
    current_satellite: Option<TLSatellite>,
    station_config: GSconfiguration,
    sat_config: SatSelection,
    current_state: AppState,
    sub_point_range: i64,
    exit: bool,
}
impl Default for Model {
    fn default() -> Self {
        Model {
            station_config: GSconfiguration::load(),
            current_satellite: None,
            sub_point_range: 120 * 60,
            exit: false,
            sat_config: SatSelection::default(),
            current_state: AppState::Base,
        }
    }
}
#[derive(Clone)]
struct TLSatellite {
    satellite: Satellite,
    metadata: MetaData,
}
#[derive(Clone, Debug)]
struct MetaData {
    owner: String,
    launch_date: DateTime<Utc>,
    object_id: String,
    inclination: f64,
}
impl MetaData {
    fn from_string(sp_json: &Map<String, Value>) -> Result<MetaData> {
        let owner = sp_json.get("OWNER").and_then(|x| x.as_str());
        let object_id = sp_json.get("OBJECT_ID").and_then(|x| x.as_str());
        let launch_date_str = sp_json.get("LAUNCH_DATE").and_then(|x| x.as_str());
        let inclination = sp_json
            .get("INCLINATION")
            .and_then(|x| x.as_number().and_then(|y| y.as_f64()));
        if owner.is_none()
            || object_id.is_none()
            || launch_date_str.is_none()
            || inclination.is_none()
        {
            debug!(
                "owner: {:?}, id: {:?}, date: {:?}, inclination: {:?}",
                owner, object_id, launch_date_str, inclination
            );
            return Err(Error::msg("Unable to unpack response from celestrak"));
        } else {
            let launch_date = NaiveDate::parse_from_str(launch_date_str.unwrap(), "%Y-%m-%d")?;
            return Ok(MetaData {
                owner: owner.unwrap().to_string(),
                inclination: inclination.unwrap(),
                launch_date: launch_date
                    .and_time(NaiveTime::from_num_seconds_from_midnight_opt(0, 0).unwrap())
                    .and_utc(),
                object_id: object_id.unwrap().to_string(),
            });
        }
    }
    fn to_string(&self) -> Map<String, Value> {
        let mut carry: Map<String, Value> = Map::new();
        carry.insert("OWNER".to_string(), self.owner.clone().into());
        carry.insert("OBJECT_ID".to_string(), self.object_id.clone().into());
        carry.insert(
            "LAUNCH_DATE".to_string(),
            format!("{}", self.launch_date.date_naive().format("%Y-%m-%d")).into(),
        );
        carry.insert("INCLINATION".to_string(), self.inclination.into());
        carry
    }
}
