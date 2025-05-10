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
    let log_file = std::fs::File::create("./log.log")?;
    Subscriber::builder()
        .with_writer(Mutex::new(log_file))
        .with_max_level(Level::DEBUG)
        .init();
    Ok(())
}
// pub fn get_config_dir() -> eyre::Result<PathBuf> {
//   let directory = if let Ok(s) = std::env::var("RATATUI_TEMPLATE_CONFIG") {
//     PathBuf::from(s)
//   } else if let Some(proj_dirs) = ProjectDirs::from("com", "kdheepak", "ratatui-template") {
//     proj_dirs.config_local_dir().to_path_buf()
//   } else {
//     return Err(eyre::eyre!("Unable to find config directory for ratatui-template"));
//   };
//   Ok(directory)
// }
// pub fn version() -> String {
//     let author = clap::crate_authors!();

//     let commit_hash = env!("RATATUI_TEMPLATE_GIT_INFO");

//     // let current_exe_path = PathBuf::from(clap::crate_name!()).display().to_string();
//     let config_dir_path = get_config_dir().unwrap().display().to_string();
//     let data_dir_path = get_data_dir().unwrap().display().to_string();

//     format!(
//         "\
// {commit_hash}

// Authors: {author}

// Config directory: {config_dir_path}
// Data directory: {data_dir_path}"
//     )
// }
