#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    use color_eyre::eyre::Result;

    use directories::ProjectDirs;

    use lazy_static::lazy_static;

    use std::{path::PathBuf, sync::Mutex};

    use tracing::Level;

    use tracing_subscriber::fmt::Subscriber;

    lazy_static! {
        pub static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
        pub static ref DATA_FOLDER: Option<PathBuf> =
            std::env::var(format!("{}_DATA", PROJECT_NAME.clone()))
                .ok()
                .map(PathBuf::from);
        pub static ref LOG_ENV: String = format!("{}_LOGLEVEL", PROJECT_NAME.clone());
        pub static ref LOG_FILE: String = format!("{}.log", env!("CARGO_PKG_NAME"));
    }

    fn project_directory() -> Option<ProjectDirs> {
        ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
    }

    pub fn get_data_dir() -> PathBuf {
        let directory = if let Some(s) = DATA_FOLDER.clone() {
            s
        } else if let Some(proj_dirs) = project_directory() {
            proj_dirs.data_local_dir().to_path_buf()
        } else {
            PathBuf::from(".").join(".data")
        };
        directory
    }

    pub fn initialize_logging() -> Result<()> {
        let dir = get_data_dir();
        let log_file = std::fs::File::create(dir.join("outlog.log"))?;
        Subscriber::builder()
            .with_writer(Mutex::new(log_file))
            .with_max_level(Level::DEBUG)
            .init();
        Ok(())
    }
}
#[cfg(target_arch = "wasm32")]
pub mod web {
    use color_eyre::eyre::Result;
    use tracing_subscriber::prelude::*;
    use tracing_web::MakeWebConsoleWriter;

    pub fn initialize_logging() -> Result<()> {
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false) // Only partially supported across browsers
            .without_time() // std::time is not available in browsers, see note below
            .with_writer(MakeWebConsoleWriter::new()); // write events to the console

        tracing_subscriber::registry().with(fmt_layer).init(); // Install these as subscribers to tracing event        Ok(())
        Ok(())
    }
}
