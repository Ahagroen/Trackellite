use app::{handle_event, update};
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
    model
        .sat_config
        .satellite_list
        .push(Satellite::new_from_tle(
            "ISS (ZARYA)
1 25544U 98067A   25124.17583429  .00010980  00000+0  20479-3 0  9995
2 25544  51.6364 165.0572 0002347  78.0135  27.5001 15.49334428508330",
        ));
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
enum Message {
    Close,
    ToggleSatConfig,
    AddSatellite,  //Only in SatConfig
    SatListUp,     //Only in SatConfig
    SatListDown,   //Only in SatConfig
    SatListSelect, //Only in SatConfig
}
#[derive(Default)]
struct SatSelection {
    satellite_list: Vec<Satellite>,
    list_state: ListState,
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
