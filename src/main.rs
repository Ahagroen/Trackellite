use app::{get_gs_cache, get_sat_cache, handle_event, update};
use arboard::Clipboard;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use color_eyre::{Result, eyre::Error, owo_colors::OwoColorize};
use ratatui::{
    crossterm::event::KeyCode,
    init, restore,
    widgets::{ListState, TableState},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, from_str};
use sky_track::{GroundStation, Pass, Satellite};
use ui::view;
use utils::initialize_logging;
mod app;
mod ui;
mod utils;
use tracing::{debug, info, warn};

fn main() -> Result<()> {
    initialize_logging()?;
    color_eyre::install()?;
    let mut terminal = init();
    let mut model = Model::default();
    info!("Loaded Model");
    while !model.exit {
        terminal.draw(|f| view(&mut model, f))?;
        let mut current_msg = handle_event(&model)?;
        while current_msg.is_some() {
            current_msg = update(&mut model, current_msg.unwrap());
        }
    }
    restore();
    Ok(())
}
enum ListMovement {
    Up,
    Down,
    Left,
    Right,
    Select,
}
enum SatList {
    ListMovement(ListMovement),
    CopyTLE,
    FetchTLE,
    AddSatellite,
}
enum AddSatMsg {
    ToggleEditing,
    StopEditing,
    ChangeSelection,
    LetterTyped(KeyCode),
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
    GSConfigMsg(GSConfigMsg),
    PropagatePasses,
}
enum GSConfigMsg {
    ListMovement(ListMovement),
    Back,
    Backspace,
    StopEditing,
    LetterTyped(KeyCode),
}
struct CurrentMsg {
    error: bool,
    text: String,
}
impl CurrentMsg {
    fn error(msg: &str) -> CurrentMsg {
        return CurrentMsg {
            error: true,
            text: msg.to_string(),
        };
    }
    fn message(msg: &str) -> CurrentMsg {
        return CurrentMsg {
            error: false,
            text: msg.to_string(),
        };
    }
}
struct GSconfiguration {
    station_list: Vec<TLGroundStation>,
    table_state: TableState,
    editing: GSconfigState,
    current_msg: CurrentMsg,
    current_edit_buffer: String,
}
#[derive(PartialEq)]
enum GSconfigState {
    RowSelect,
    CellSelect,
    CellEdit,
}
impl Default for GSconfiguration {
    fn default() -> Self {
        let stations = GSconfiguration::load();
        let current_message;
        let station_list;
        if stations.is_err() {
            warn!("Unable to Load ground stations from file");
            current_message = CurrentMsg {
                error: true,
                text: stations.err().unwrap().to_string(),
            };
            station_list = vec![];
        } else {
            station_list = stations.unwrap();
            current_message = CurrentMsg::message("")
        }
        GSconfiguration {
            station_list,
            table_state: TableState::default(),
            editing: GSconfigState::RowSelect,
            current_msg: current_message,
            current_edit_buffer: "".to_string(),
        }
    }
}
impl GSconfiguration {
    fn load() -> Result<Vec<TLGroundStation>> {
        let cached_gs = get_gs_cache()?;
        debug!("Cached GS': {:?}", cached_gs);
        let mut stations: Vec<TLGroundStation> = vec![];
        for key in cached_gs.values() {
            let value_map: TLGroundStation = from_str(key).unwrap();
            stations.push(value_map)
        }
        Ok(stations)
    }
}

struct SatSelection {
    satellite_list: Vec<TLSatellite>,
    list_state: ListState,
    clipboard: Clipboard,
    current_message: CurrentMsg,
    add_sat: AddSatState,
}
impl Default for SatSelection {
    fn default() -> Self {
        let sat_list = Self::load_sat_from_file();
        let current_message;
        let satellites;
        if sat_list.is_err() {
            warn!("Unable to Load satellites from file");
            current_message = CurrentMsg {
                error: true,
                text: sat_list.err().unwrap().to_string(),
            };
            satellites = vec![];
        } else {
            current_message = CurrentMsg {
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
        let cached_tles = get_sat_cache()?;
        debug!("Cached TLE's: {:?}", cached_tles);
        let mut satellites = vec![];
        for i in cached_tles.values() {
            let data: TLSatellite = from_str(&i).unwrap();
            satellites.push(data)
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
    upcoming_passes: Vec<TLPass>,
    current_state: AppState,
    sub_point_range: i64,
    exit: bool,
}
impl Default for Model {
    fn default() -> Self {
        Model {
            station_config: GSconfiguration::default(),
            current_satellite: None,
            upcoming_passes: vec![],
            sub_point_range: 120 * 60,
            exit: false,
            sat_config: SatSelection::default(),
            current_state: AppState::Base,
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
struct TLSatellite {
    satellite: Satellite,
    metadata: MetaData,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
struct MetaData {
    owner: String,
    #[serde(with = "celestrak_date")]
    launch_date: DateTime<Utc>,
    object_id: String,
    inclination: f64,
}
#[derive(Clone, Serialize, Deserialize)]
struct TLGroundStation {
    station: GroundStation,
    active: bool,
}
#[derive(Debug, Clone)]
struct TLPass {
    pass: Pass,
    station: String,
}
mod celestrak_date {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer, de::Error};

    const FORMAT: &'static str = "%Y-%m-%d";

    // The signature of a serialize_with function must follow the pattern:
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let dt = NaiveDate::parse_from_str(&s, FORMAT)
            .map_err(Error::custom)?
            .and_time(NaiveTime::from_num_seconds_from_midnight_opt(0, 0).unwrap());
        Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    }
}
