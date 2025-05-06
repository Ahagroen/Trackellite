use std::time::{SystemTime, UNIX_EPOCH};

use color_eyre::owo_colors::OwoColorize;
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style, Styled, Stylize},
    text::{Line, ToLine},
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, List, Paragraph,
        canvas::{Canvas, Map, MapResolution},
    },
};
use tracing::debug;

use crate::{AddSatSel, AppState, Model};

pub fn view(model: &mut Model, frame: &mut Frame) {
    let inner = view_app_border(model, frame, None);
    view_ground_track(model, frame, Some(inner));
    if model.current_state == AppState::SatSelect || model.current_state == AppState::SatAddition {
        view_popup(model, frame);
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

fn view_popup(model: &mut Model, frame: &mut Frame) {
    let area = popup_area(frame.area(), 70, 50);
    frame.render_widget(Clear, area);
    let outer_block = Block::new().title_top(Line::from("Satellite Configuration").centered());
    let left_side_block = Block::bordered().title("Current Satellites").on_dark_gray();
    let right_side_block = Block::bordered().title("Satellite Details").on_dark_gray();
    let [list_area, detail_side] = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)])
        .areas(outer_block.inner(area));
    frame.render_widget(outer_block, area);
    render_sat_list(model, frame, left_side_block, list_area);
    let [detail_area, message_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
            .areas(right_side_block.inner(detail_side));
    let [text_area, tle_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Percentage(30)]).areas(detail_area);
    frame.render_widget(right_side_block, detail_side);
    let details;
    let tle;
    let tle_block = Block::new()
        .borders(Borders::all())
        .title_top("TLE")
        .border_type(ratatui::widgets::BorderType::Rounded)
        .border_style(Style::new().fg(Color::Cyan));
    if model.current_state == AppState::SatAddition {
        if model.sat_config.add_sat.selected == AddSatSel::NoradID {
            let norad_id = model.sat_config.add_sat.text.clone();
            let mut open_norad_id = String::new();
            while norad_id.len() + open_norad_id.len() < 5 {
                open_norad_id.push_str("_");
            }
            let norad_id_render;
            if model.sat_config.add_sat.editing {
                norad_id_render = vec![
                    "Satellite Norad ID:".into(),
                    norad_id.into(),
                    open_norad_id.slow_blink(),
                ];
            } else {
                norad_id_render = vec![
                    "Satellite Norad ID:".into(),
                    norad_id.reversed(),
                    open_norad_id.reversed(),
                ];
            }
            details = Paragraph::new(vec![
                Line::from("Satellite Name: XXXXX"),
                Line::from(norad_id_render),
                Line::from("\nCurrent TLE age: 0 day(s), 0h 0m 0s\n"),
            ]);
            tle = Paragraph::new("").block(tle_block);
        } else {
            details = Paragraph::new(vec![
                "Satellite Name: XXXXX\nSatellite Norad ID:".into(),
                "_____".into(),
                "\nCurrent TLE age: 0 day(s), 0h 0m 0s\n".into(),
            ]);
            if model.sat_config.add_sat.editing {
                let current_text = model.sat_config.add_sat.text.clone();
                let mut y_offset: u16 = 1;
                let mut x_offset: u16 = 1;
                if current_text.len() > 0 {
                    let lines: Vec<String> = current_text.lines().map(|x| x.to_string()).collect();
                    y_offset = lines.len() as u16;
                    if let Some(x) = lines.last() {
                        x_offset = x.len() as u16 + 1;
                    }
                }
                frame.set_cursor_position(Position::new(
                    tle_area.x + x_offset,
                    tle_area.y + y_offset,
                ));
                tle = Paragraph::new(model.sat_config.add_sat.text.clone()).block(tle_block);
            } else {
                tle = Paragraph::new(model.sat_config.add_sat.text.clone())
                    .block(tle_block)
                    .reversed();
            }
        }
    } else {
        let mut current_sat = None;
        if let Some(index) = model.sat_config.list_state.selected() {
            if let Some(x) = model.sat_config.satellite_list.get(index) {
                current_sat = Some(x)
            };
        };
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
                ));
                tle = Paragraph::new(sat.get_tle()).block(tle_block);
            }
            None => {
                details = Paragraph::new(
                    "Satellite Name: _____\nSatellite Norad ID: _____\nCurrent TLE age: 0 day(s), 0h 0m 0s\n",
                );
                tle = Paragraph::new("").block(tle_block);
            }
        }
    }
    frame.render_widget(details, text_area);
    frame.render_widget(tle, tle_area);
    if model.sat_config.current_message.error {
        frame.render_widget(
            Line::from(model.sat_config.current_message.text.as_ref()).red(),
            message_area,
        );
    } else {
        frame.render_widget(
            Line::from(model.sat_config.current_message.text.as_ref()),
            message_area,
        );
    }
}

fn render_sat_list(
    model: &mut Model,
    frame: &mut Frame<'_>,
    left_side_block: Block<'_>,
    list_area: Rect,
) {
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
    frame.render_stateful_widget(list, list_area, &mut model.sat_config.list_state);
}

fn strf_seconds(seconds: i64) -> String {
    let minutes = (seconds % 3600) / 60;
    let hours = (seconds % 86400) / 3600;
    let seconds_new = seconds % 60;
    let days = seconds / 86400;
    format!("{} day(s), {}h {}m {}s", days, hours, minutes, seconds_new)
}

fn view_app_border(model: &Model, frame: &mut Frame, area: Option<Rect>) -> Rect {
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
                "Close Popup ".into(),
                "<c> ".blue().bold(),
                "Close Editor ".into(),
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
    render_background_map(frame, draw_area);
    if model.current_satellite.is_some() {
        render_tracks(model, frame, draw_area);
    } else {
        render_no_sat_text(frame, draw_area);
    }
}

fn render_no_sat_text(frame: &mut Frame<'_>, draw_area: Rect) {
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

fn render_tracks(model: &Model, frame: &mut Frame<'_>, draw_area: Rect) {
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
}

fn render_background_map(frame: &mut Frame<'_>, draw_area: Rect) {
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
}
