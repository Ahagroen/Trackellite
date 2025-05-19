use infobox::view_sat_data;
use popup::{view_popup_gs_config, view_popup_sat_config};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
};
use topbar::view_top_bar;
use track::view_ground_track;
mod infobox;
mod popup;
mod topbar;
mod track;
use crate::structs::{AppState, Model};

pub fn view(model: &Model, frame: &mut Frame) {
    {
        let [top_bar, core_bar, bottom_bar] = Layout::vertical([
            Constraint::Length(5),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());
        view_top_bar(model, frame, Some(top_bar));
        view_app_border(model, frame, Some(bottom_bar));
        let [ground_track_area, sat_stat_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(41)]).areas(core_bar);
        view_ground_track(model, frame, Some(ground_track_area));
        view_sat_data(model, frame, Some(sat_stat_area));
    }
    {
        if model.current_state == AppState::SatSelect
            || model.current_state == AppState::SatAddition
        {
            view_popup_sat_config(model, frame);
        } else if model.current_state == AppState::GSConfig {
            view_popup_gs_config(model, frame)
        }
    }
}

fn strf_seconds_small(seconds: i64) -> String {
    let working_seconds;
    if seconds < 0 {
        working_seconds = -seconds
    } else {
        working_seconds = seconds
    }
    let minutes = (working_seconds % 3600) / 60;
    let hours = (working_seconds) / 3600;
    let seconds_new = working_seconds % 60;
    if seconds < 0 {
        format!("-{:02}:{:02}:{:02}", hours, minutes, seconds_new)
    } else {
        format!("+{:02}:{:02}:{:02}", hours, minutes, seconds_new)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn view_app_border(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let instructions;
    match model.current_state {
        AppState::Base => {
            instructions = Line::from(vec![
                "Configure Ground Stations ".into(),
                "<g> ".blue().bold(),
                "Configure Satellites ".into(),
                "<s> ".blue().bold(),
                "Quit ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::SatSelect => {
            instructions = Line::from(vec![
                "Fetch TLE from Spacetrack ".into(),
                "<f> ".blue().bold(),
                "Copy TLE ".into(),
                "<c> ".blue().bold(),
                "Close Popup ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::SatAddition => {
            instructions = Line::from(vec![
                "Paste TLE ".into(),
                "<v> ".blue().bold(),
                "Close Popup ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::GSConfig => instructions = Line::from(vec!["".into(), "".blue().bold()]),
    }
    frame.render_widget(instructions.right_aligned(), draw_area);
}
#[cfg(target_arch = "wasm32")]
fn view_app_border(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let instructions;
    match model.current_state {
        AppState::Base => {
            instructions = Line::from(vec![
                "Configure Ground Stations ".into(),
                "<g> ".blue().bold(),
                "Configure Satellites ".into(),
                "<s> ".blue().bold(),
                "Quit ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::SatSelect => {
            instructions = Line::from(vec![
                "Fetch TLE from Spacetrack ".into(),
                "<f> ".blue().bold(),
                "Copy TLE ".into(),
                "<c> ".blue().bold(),
                "Close Popup ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::SatAddition => {
            instructions = Line::from(vec![
                "Paste TLE ".into(),
                "<v> ".blue().bold(),
                "Close Popup ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::GSConfig => instructions = Line::from(vec!["".into(), "".blue().bold()]),
        #[cfg(not(target_arch = "wasm32"))]
        AppState::SatWaitingFetch => instructions = Line::from(vec!["".into(), "".blue().bold()]),
    }
    frame.render_widget(instructions.right_aligned(), draw_area);
}
fn strf_seconds(seconds: i64) -> String {
    let working_seconds;
    if seconds < 0 {
        working_seconds = -seconds
    } else {
        working_seconds = seconds
    }
    let minutes = (working_seconds % 3600) / 60;
    let hours = (working_seconds % 86400) / 3600;
    let seconds_new = working_seconds % 60;
    let days = working_seconds / 86400;
    format!("{} day(s), {}h {}m {}s", days, hours, minutes, seconds_new)
}
