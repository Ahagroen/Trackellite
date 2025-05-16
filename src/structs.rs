use chrono::DateTime;
use chrono::Utc;
use color_eyre::eyre::eyre;
use serde::Deserialize;
use serde::Serialize;
use sky_track::Pass;

use sky_track::GroundStation;

use sky_track::Satellite;

#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;

#[cfg(target_arch = "wasm32")]
use futures::executor;
#[cfg(target_arch = "wasm32")]
use ratzilla::web_sys;

use ratatui::widgets::ListState;

use serde_json::from_str;

use color_eyre::Result;

use ratatui::widgets::TableState;

#[cfg(not(target_arch = "wasm32"))]
use ratatui::crossterm::event::KeyCode;

#[cfg(target_arch = "wasm32")]
use ratzilla::event::KeyCode;

use tracing::debug;
use tracing::warn;

use crate::app::file_cache::cache::get_gs_cache;
use crate::app::file_cache::cache::get_sat_cache;

#[derive(Clone)]
pub enum ListMovement {
    Up,
    Down,
    Left,
    Right,
    Select,
}

#[derive(Clone)]
pub enum SatList {
    ListMovement(ListMovement),
    CopyTLE,
    FetchTLE,
    AddSatellite,
}

#[derive(Clone)]
pub enum AddSatMsg {
    ToggleEditing,
    StopEditing,
    ChangeSelection,
    LetterTyped(KeyCode),
    Backspace,
    PasteTLE,
}

#[derive(PartialEq, Debug)]
pub enum AddSatSel {
    NoradID,
    TLEBox,
}

pub struct AddSatState {
    pub selected: AddSatSel,
    pub text: String,
    pub editing: bool,
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

#[derive(Clone)]
pub enum Message {
    Close,
    ToggleSatConfig,
    SatListMessage(SatList),
    AddSatMessage(AddSatMsg),
    ToggleGSConfig,
    GSConfigMsg(GSConfigMsg),
    PropagatePasses,
}

#[derive(Clone)]
pub enum GSConfigMsg {
    ListMovement(ListMovement),
    Back,
    Backspace,
    StopEditing,
    LetterTyped(KeyCode),
}

pub struct CurrentMsg {
    pub error: bool,
    pub text: String,
}

impl CurrentMsg {
    pub fn error(msg: &str) -> CurrentMsg {
        return CurrentMsg {
            error: true,
            text: msg.to_string(),
        };
    }
    pub fn message(msg: &str) -> CurrentMsg {
        return CurrentMsg {
            error: false,
            text: msg.to_string(),
        };
    }
}

pub struct GSconfiguration {
    pub station_list: Vec<TLGroundStation>,
    pub table_state: TableState,
    pub editing: GSconfigState,
    pub current_msg: CurrentMsg,
    pub current_edit_buffer: String,
}

#[derive(PartialEq)]
pub enum GSconfigState {
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
    pub fn load() -> Result<Vec<TLGroundStation>> {
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

pub struct SatSelection {
    pub satellite_list: Vec<TLSatellite>,
    pub list_state: ListState,
    pub clipboard: TLClipboard,
    pub current_message: CurrentMsg,
    pub add_sat: AddSatState,
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
            clipboard: TLClipboard::new(),
            current_message,
            add_sat: AddSatState::default(),
        }
    }
}

impl SatSelection {
    pub fn load_sat_from_file() -> Result<Vec<TLSatellite>> {
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
pub enum AppState {
    Base,
    SatSelect,
    SatAddition,
    GSConfig,
}

pub struct Model {
    pub current_satellite: Option<TLSatellite>,
    pub station_config: GSconfiguration,
    pub sat_config: SatSelection,
    pub upcoming_passes: Vec<TLPass>,
    pub current_state: AppState,
    pub sub_point_range: i64,
    pub exit: bool,
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
pub struct TLSatellite {
    pub satellite: Satellite,
    pub metadata: MetaData,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct MetaData {
    pub owner: String,
    #[serde(with = "celestrak_date")]
    pub launch_date: DateTime<Utc>,
    pub object_id: String,
    pub inclination: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TLGroundStation {
    pub station: GroundStation,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct TLPass {
    pub pass: Pass,
    pub station: GroundStation,
}

pub mod celestrak_date {
    use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
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

pub struct TLClipboard {
    #[cfg(not(target_arch = "wasm32"))]
    clipboard: Clipboard,
    #[cfg(target_arch = "wasm32")]
    clipboard: WebClipboard,
}
impl TLClipboard {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> TLClipboard {
        TLClipboard {
            clipboard: Clipboard::new().expect("Could not open clipboard"),
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub fn new() -> TLClipboard {
        TLClipboard {
            clipboard: WebClipboard::default(),
        }
    }
    pub fn set_text(&mut self, text: &str) -> Result<()> {
        self.set_text_adjusted(text)
    }
    fn set_text_adjusted(&mut self, text: &str) -> Result<()> {
        self.clipboard
            .set_text(text)
            .map_err(|_| eyre!("Failed to set clipboard text"))
    }
    pub fn get_text(&mut self) -> String {
        self.clipboard.get_text().unwrap_or_default()
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct WebClipboard {}
#[cfg(target_arch = "wasm32")]
impl WebClipboard {
    pub fn set_text(&self, text: &str) -> Result<()> {
        Ok(executor::block_on(self.set_text_inner(text)))
    }
    pub fn get_text(&self) -> Result<String> {
        Ok(executor::block_on(self.get_text_inner()))
    }
    async fn set_text_inner(&self, text: &str) {
        let window = web_sys::window().unwrap();
        let nav = window.navigator().clipboard();
        let promise = nav.write_text(text);
        wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
    }
    async fn get_text_inner(&self) -> String {
        let window = web_sys::window().unwrap();
        let nav = window.navigator().clipboard();
        let promise = nav.read_text();
        let result = wasm_bindgen_futures::JsFuture::from(promise).await.unwrap();
        result.as_string().unwrap_or_default()
    }
}
