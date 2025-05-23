use color_eyre::Result;
use structs::Model;
mod app;
mod ui;
mod utils;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::io::Stdout;

    use color_eyre::Result;
    use ratatui::Terminal;
    use ratatui::prelude::CrosstermBackend;
    use ratatui::{init, restore};
    use tracing::info;

    use crate::app::key_handle_native::handle_event;
    use crate::app::update;
    use crate::structs::Model;
    use crate::ui::view;
    use crate::utils::native::initialize_logging;
    fn setup() -> Result<Terminal<CrosstermBackend<Stdout>>> {
        Ok(init())
    }
    pub fn run() -> Result<()> {
        initialize_logging()?;
        color_eyre::install()?;
        let mut terminal = setup()?;
        let mut model = Model::default();
        info!("Loaded Model");
        while !&model.exit {
            terminal.draw(|f| view(&model, f))?;
            let current_msg = handle_event(&model)?;
            if let Some(msg) = current_msg {
                update(&mut model, msg);
            }
        }
        restore();
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use std::{cell::RefCell, io, rc::Rc};

    use ratatui::Terminal;
    use ratzilla::{CanvasBackend, DomBackend, WebRenderer};
    use tracing::info;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::Closure;

    use crate::app::update;
    use crate::utils::web::initialize_logging;
    use crate::{app::key_handle_native::handle_event, structs::Model, ui::view};
    use color_eyre::Result;

    fn setup() -> Result<Terminal<CanvasBackend>, io::Error> {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        let backend = CanvasBackend::new()?;
        let mut terminal = Terminal::new(backend)?;
        terminal.autoresize()?;
        Ok(terminal)
    }
    pub fn run() -> Result<()> {
        initialize_logging()?;
        color_eyre::install()?;
        let terminal = setup()?;
        let (tx, rx) = std::sync::mpsc::channel();
        let model = Rc::new(RefCell::new(Model::default()));
        info!("Loaded Model");
        terminal.on_key_event({
            let key_tx = tx.clone();
            let model_state = model.clone();
            move |key_event| {
                let state = model_state.borrow();
                handle_event(&state, key_event, key_tx.clone());
            }
        });
        terminal.draw_web({
            let model_state = model.clone();
            move |frame| {
                let state = model_state.borrow();
                view(&state, frame);
            }
        });
        let model_state = model.clone();
        let callback = Closure::<dyn Fn()>::new(move || {
            if let Ok(x) = rx.try_recv() {
                let mut mut_model = model_state.borrow_mut();
                update(&mut mut_model, x, tx.clone());
            }
        });
        let _ = web_sys::window()
            .unwrap()
            .set_interval_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                50,
            );
        callback.forget(); //leaks memory
        Ok(())
    }
}

fn main() -> Result<()> {
    #[cfg(target_arch = "wasm32")]
    web::run()?;

    #[cfg(not(target_arch = "wasm32"))]
    native::run()?;

    Ok(())
}
mod structs;
