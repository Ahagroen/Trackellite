use app::{key_handle_native::handle_event, update};
use color_eyre::Result;
use ratatui::{init, restore};
use structs::Model;
use ui::view;
use utils::initialize_logging;
mod app;
mod ui;
mod utils;
use tracing::info;

fn main() -> Result<()> {
    initialize_logging()?;
    color_eyre::install()?;
    let mut terminal = init();
    let mut model = Model::default();
    info!("Loaded Model");
    while !&model.exit {
        terminal.draw(|f| view(&mut model, f))?;
        let current_msg = handle_event(&model)?;
        if current_msg.is_some() {
            update(&mut model, current_msg.unwrap());
        }
    }
    restore();
    Ok(())
}
mod structs;
