use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Stylize},
    text::Line,
    widgets::{
        Axis, Block, Chart, Clear, Dataset, List, Paragraph,
        canvas::{Canvas, Map, MapResolution},
    },
};
use tracing::debug;

use crate::{AppState, Model};

pub fn view(model: &mut Model, frame: &mut Frame) {
    let inner = app_border(model, frame, None);
    view_ground_track(model, frame, Some(inner));
    if model.current_state == AppState::SatSelect || model.current_state == AppState::SatAddition {
        draw_popup(model, frame);
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn draw_popup(model: &mut Model, frame: &mut Frame) {
    let area = popup_area(frame.area(), 50, 50);
    frame.render_widget(Clear, area);
    let outer_block = Block::new().title_top(Line::from("Satellite Configuration").centered());
    let left_side_block = Block::bordered().title("Current Satellites").on_dark_gray();
    let right_side_block = Block::bordered().title("Satellite Details").on_dark_gray();
    let [list_area, detail_area] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
        .areas(outer_block.inner(area));
    frame.render_widget(outer_block, area);
    let mut items: Vec<String> = model
        .sat_config
        .satellite_list
        .clone()
        .into_iter()
        .map(|x| {
            if model.current_satellite.as_ref().is_some_and(|y| y == &x) {
                format!("*{}", x.get_name())
            } else {
                x.get_name()
            }
        })
        .collect();
    items.push("Add Satellite".to_string());
    let list = List::new(items)
        .block(left_side_block)
        .highlight_symbol(">>");
    let mut current_sat = None;
    if let Some(index) = model.sat_config.list_state.selected() {
        if let Some(x) = model.sat_config.satellite_list.get(index) {
            current_sat = Some(x)
        };
    };
    frame.render_stateful_widget(list, list_area, &mut model.sat_config.list_state);
    if model.current_state == AppState::SatAddition {
    } else {
        let details;
        match current_sat {
            Some(sat) => {
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                let base_offset = current_time - sat.get_epoch().timestamp();
                details = Paragraph::new(format!(
                    "Satellite Name: {}\nSatellite Norad ID: {}\nCurrent TLE age: {}\n",
                    sat.get_name(),
                    sat.get_norad_id(),
                    strf_seconds(base_offset)
                ))
                .block(right_side_block)
            }
            None => details = Paragraph::new("").block(right_side_block),
        }
        frame.render_widget(details, detail_area);
    }
}

fn strf_seconds(seconds: i64) -> String {
    let minutes = (seconds % 3600) / 60;
    let hours = (seconds % 86400) / 3600;
    let seconds_new = seconds % 60;
    let days = seconds / 86400;
    format!("{} days, {}h {}m {}s", days, hours, minutes, seconds_new)
}

fn app_border(model: &Model, frame: &mut Frame, area: Option<Rect>) -> Rect {
    let draw_area = area.unwrap_or(frame.area());
    let title = Line::from("Trackellite".bold());
    let center_title;
    if let Some(x) = model.current_satellite.as_ref() {
        center_title = Line::from(x.get_name())
    } else {
        center_title = Line::from("");
    }
    let instructions;
    match model.current_state {
        AppState::Base => {
            instructions = Line::from(vec![
                "Configure Satellites ".into(),
                "<s> ".blue().bold(),
                "Quit ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::SatSelect => {
            instructions = Line::from(vec![
                "Close Popup ".into(),
                "<c> ".blue().bold(),
                "Quit ".into(),
                "<q> ".blue().bold(),
            ])
        }
        AppState::SatAddition => {
            instructions = Line::from(vec![
                "Close Popup ".into(),
                "<c> ".blue().bold(),
                "Quit ".into(),
                "<q> ".blue().bold(),
            ])
        }
    }
    let block = Block::new()
        .title_top(title.left_aligned())
        .title_top(center_title.centered())
        .title_bottom(instructions.right_aligned());
    let data = block.inner(draw_area);
    frame.render_widget(block, draw_area);
    data
}
fn view_ground_track(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let base_map = Canvas::default()
        .paint(|ctx| {
            ctx.draw(&Map {
                color: Color::Green,
                resolution: MapResolution::High,
            });
        })
        .x_bounds([-180.0, 180.0])
        .y_bounds([-90.0, 90.0]);
    frame.render_widget(base_map, draw_area);
    if model.current_satellite.is_some() {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let working_satellites = model.current_satellite.as_ref().unwrap();
        let base_offset = current_time - working_satellites.get_epoch().timestamp();
        let current_pos = working_satellites.get_sub_point(base_offset);
        let points: Vec<(f64, f64)> = ((base_offset - 300)..(base_offset + model.sub_point_range))
            .map(|x| {
                let sub_point = working_satellites.get_sub_point(x);
                (sub_point.long, sub_point.lat)
            })
            .collect();
        let mut prev: Option<f64> = None;
        let mut paths_list: Vec<Dataset> = vec![];
        let mut current_start: usize = 0;
        let mut current_end: usize = 0;
        for i in &points {
            if prev.is_some() {
                if prev.unwrap() < i.0 {
                    let dataset = Dataset::default()
                        .name(working_satellites.get_name())
                        .marker(ratatui::symbols::Marker::Braille)
                        .graph_type(ratatui::widgets::GraphType::Line)
                        .cyan()
                        .data(&points[current_start..current_end]);
                    paths_list.push(dataset);
                    debug!(
                        "End of Orbit! start_index: {}, end_index: {}",
                        current_start, current_end
                    );
                    current_start = current_end + 1;
                }
            }
            // debug!("current x: {}, last x: {:?}", i.0, prev);
            current_end += 1;
            prev = Some(i.0);
        }
        let dataset = Dataset::default()
            .name(working_satellites.get_name())
            .marker(ratatui::symbols::Marker::Braille)
            .graph_type(ratatui::widgets::GraphType::Line)
            .data(&points[current_start..current_end])
            .cyan();
        paths_list.push(dataset);
        paths_list.reverse();
        let x_axis = Axis::default().bounds([-180.0, 180.0]);
        let y_axis = Axis::default().bounds([-90.0, 90.0]);
        frame.render_widget(
            Chart::new(paths_list)
                .x_axis(x_axis)
                .y_axis(y_axis)
                .legend_position(None),
            draw_area,
        );
        frame.render_widget(
            Canvas::default()
                .paint(|ctx| {
                    ctx.print(
                        current_pos.long,
                        current_pos.lat,
                        "#".red().into_centered_line(),
                    );
                    debug!(?current_pos);
                })
                .x_bounds([-180.0, 180.0])
                .y_bounds([-90.0, 90.0]),
            draw_area,
        );
    } else {
        frame.render_widget(
            Canvas::default()
                .paint(|ctx| {
                    ctx.print(
                        -0.15,
                        0.0,
                        "Add a satellite to begin tracking"
                            .yellow()
                            .into_centered_line(),
                    )
                })
                .x_bounds([-1.0, 1.0])
                .y_bounds([-1.0, 1.0]),
            draw_area,
        );
    }
}
