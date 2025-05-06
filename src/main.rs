use app::{get_tle_cache, handle_event, update};
use arboard::Clipboard;
use color_eyre::Result;
use ratatui::widgets::ListState;
use sky_track::Satellite;
use ui::view;
use utils::initialize_logging;
mod app;
mod ui;
mod utils;
use tracing::{debug, info};

fn main() -> Result<()> {
    initialize_logging()?;
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
}

struct CurrentMsgSatSel {
    error: bool,
    text: String,
}

struct SatSelection {
    satellite_list: Vec<Satellite>,
    list_state: ListState,
    clipboard: Clipboard,
    current_message: CurrentMsgSatSel,
    add_sat: AddSatState,
}
impl Default for SatSelection {
    fn default() -> Self {
        let cached_tles = get_tle_cache().unwrap();
        debug!("Cached TLE's: {:?}", cached_tles);
        let mut satellites = vec![];
        for i in cached_tles.values() {
            satellites.push(Satellite::new_from_tle(i.as_str().unwrap()));
        }
        SatSelection {
            satellite_list: satellites,
            list_state: ListState::default(),
            clipboard: Clipboard::new().expect("Unable to access clipboard"),
            current_message: CurrentMsgSatSel {
                error: false,
                text: "".to_string(),
            },
            add_sat: AddSatState::default(),
        }
    }
}
#[derive(Debug, PartialEq)]
enum AppState {
    Base,
    SatSelect,
    SatAddition,
}

struct Model {
    current_satellite: Option<Satellite>,
    sat_config: SatSelection,
    current_state: AppState,
    sub_point_range: i64,
    exit: bool,
}
impl Default for Model {
    fn default() -> Self {
        Model {
            current_satellite: None,
            sub_point_range: 120 * 60,
            exit: false,
            sat_config: SatSelection::default(),
            current_state: AppState::Base,
        }
    }
}
