use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{Datelike, Local, TimeDelta, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, Gauge, List, Paragraph, Row, Table, Wrap,
        canvas::{Canvas, Map, MapResolution},
    },
};
use tracing::{debug, warn};

use crate::{AddSatSel, AppState, GSconfigState, Model};

pub fn view(model: &mut Model, frame: &mut Frame) {
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
    if model.current_state == AppState::SatSelect || model.current_state == AppState::SatAddition {
        view_popup_sat_config(model, frame);
    } else if model.current_state == AppState::GSConfig {
        view_popup_gs_config(model, frame)
    }
}

fn view_popup_gs_config(model: &mut Model, frame: &mut Frame<'_>) {
    let area = popup_area(frame.area(), 35, 50);
    frame.render_widget(Clear, area);
    let outer_block =
        Block::bordered().title_top(Line::from("Ground Station Configuration").centered());
    let [gs_area, message_area] = Layout::vertical([Constraint::Fill(1), Constraint::Length(1)])
        .areas(outer_block.inner(area));
    frame.render_widget(outer_block, area);
    let mut data: Vec<Row> = vec![];
    let header = Row::new(vec![
        "Active".to_string(),
        "Name".to_string(),
        "Lat".to_string(),
        "Long".to_string(),
        "Alt".to_string(),
    ]);
    for (index, i) in model.station_config.station_list.iter().enumerate() {
        let mut carry: Vec<String> = vec![];
        if i.active {
            carry.push("+".to_string())
        } else {
            carry.push(" ".to_string())
        }
        if model.station_config.editing == GSconfigState::CellEdit
            && model
                .station_config
                .table_state
                .selected()
                .is_some_and(|x| index == x)
        {
            match model.station_config.table_state.selected_column().unwrap() {
                1 => {
                    carry.append(&mut vec![
                        model.station_config.current_edit_buffer.clone(),
                        format!("{}", i.station.lat),
                        format!("{}", i.station.long),
                        format!("{}", i.station.alt),
                    ]);
                }
                2 => {
                    carry.append(&mut vec![
                        i.station.name.clone(),
                        model.station_config.current_edit_buffer.clone(),
                        format!("{}", i.station.long),
                        format!("{}", i.station.alt),
                    ]);
                }
                3 => {
                    carry.append(&mut vec![
                        i.station.name.clone(),
                        format!("{}", i.station.lat),
                        model.station_config.current_edit_buffer.clone(),
                        format!("{}", i.station.alt),
                    ]);
                }
                4 => {
                    carry.append(&mut vec![
                        i.station.name.clone(),
                        format!("{}", i.station.lat),
                        format!("{}", i.station.long),
                        model.station_config.current_edit_buffer.clone(),
                    ]);
                }
                _ => warn!("GS config index out of range"),
            }
            data.push(Row::new(carry))
        } else {
            carry.append(&mut vec![
                i.station.name.clone(),
                format!("{}", i.station.lat),
                format!("{}", i.station.long),
                format!("{}", i.station.alt),
            ]);
            data.push(Row::new(carry))
        }
    }
    data.push(Row::new(vec![
        " ".to_string(),
        "Add Station".to_string(),
        "0".to_string(),
        "0".to_string(),
        "0".to_string(),
    ]));
    let widths = [
        Constraint::Length(7),
        Constraint::Fill(2),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ];
    let table_widget: Table;
    match model.station_config.editing {
        GSconfigState::RowSelect => {
            table_widget = Table::new(data, widths)
                .highlight_symbol(">>")
                .header(header)
        }
        GSconfigState::CellSelect => {
            table_widget = Table::new(data, widths)
                .highlight_symbol(">>")
                .header(header)
                .cell_highlight_style(Style::new().reversed())
        }
        GSconfigState::CellEdit => {
            table_widget = Table::new(data, widths)
                .highlight_symbol(">>")
                .header(header)
                .cell_highlight_style(Style::new().underlined())
        }
    };
    frame.render_stateful_widget(table_widget, gs_area, &mut model.station_config.table_state);
    if model.station_config.current_msg.error {
        frame.render_widget(
            Line::from(model.station_config.current_msg.text.as_ref()).red(),
            message_area,
        );
    } else {
        frame.render_widget(
            Line::from(model.station_config.current_msg.text.as_ref()),
            message_area,
        );
    }
}

fn view_popup_sat_config(model: &mut Model, frame: &mut Frame) {
    let area = popup_area(frame.area(), 65, 50);
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
    render_sat_list_details(model, frame, detail_area);
    frame.render_widget(right_side_block, detail_side);
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
fn strf_seconds_small(seconds: i64) -> String {
    let working_seconds;
    if seconds < 0 {
        working_seconds = seconds * -1
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
fn view_top_bar(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let [met_time, center, realtime] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Percentage(50),
        Constraint::Fill(1),
    ])
    .areas(draw_area);

    let rt_frame = Block::bordered();
    let mut rt_text = vec![
        Line::from(format!("  UTC: {}", Utc::now().format("%Y-%m-%d %H:%M:%S"))).centered(),
        Line::from(format!(
            "LOCAL: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S")
        ))
        .centered(),
    ];
    if let Some(x) = model.current_satellite.as_ref() {
        rt_text.push(
            Line::from(format!(
                "  MET: {} days",
                (Utc::now().num_days_from_ce() - x.metadata.launch_date.num_days_from_ce())
            ))
            .centered(),
        );
    }
    let rt_inner = rt_frame.inner(realtime);
    frame.render_widget(rt_frame, realtime);
    frame.render_widget(List::new(rt_text), rt_inner);

    let track_frame = Block::bordered();
    let center_text = vec![Line::from(""), Line::from("Trackellite").centered()];

    let center_inner = track_frame.inner(center);
    frame.render_widget(track_frame, center);
    frame.render_widget(List::new(center_text), center_inner);

    let met_frame = Block::bordered();
    let met_inner = met_frame.inner(met_time);
    frame.render_widget(met_frame, met_time);
    if model.upcoming_passes.len() == 0 {
        let met_text;
        met_text = vec![
            Line::from(""),
            Line::from("Please select a satellite and").centered(),
            Line::from("Ground Station for next pass data").centered(),
        ];
        frame.render_widget(List::new(met_text), met_inner);
    } else {
        let pass = model.upcoming_passes[0].clone();
        let widths = vec![Constraint::Fill(1), Constraint::Fill(1)];
        let aos_time_till = Utc::now().signed_duration_since(pass.pass.get_aos_datetime());
        let los_time_till = Utc::now().signed_duration_since(pass.pass.get_los_datetime());
        let mut pass_text = vec![
            Row::new(vec![
                format!("Station: {}", pass.station),
                format!(
                    "Time to AOS: T{}",
                    strf_seconds_small(aos_time_till.num_seconds())
                ),
            ]),
            Row::new(vec![
                format!(
                    "Time to TME: T{}",
                    strf_seconds_small(
                        Utc::now()
                            .signed_duration_since(pass.pass.get_tme_datetime())
                            .num_seconds()
                    )
                ),
                format!(
                    "Time to LOS: T{}",
                    strf_seconds_small(los_time_till.num_seconds())
                ),
            ]),
        ];
        if Utc::now().signed_duration_since(pass.pass.get_aos_datetime()) > TimeDelta::zero() {
            let [table_space, bar_space] =
                Layout::vertical([Constraint::Length(2), Constraint::Length(1)]).areas(met_inner);
            let pass_duration =
                (pass.pass.get_los_datetime() - pass.pass.get_aos_datetime()).num_seconds();
            let current_progress_seconds = Utc::now()
                .signed_duration_since(pass.pass.get_aos_datetime())
                .num_seconds();
            frame.render_widget(Table::new(pass_text, widths), table_space);
            frame.render_widget(
                Gauge::default().ratio(current_progress_seconds as f64 / pass_duration as f64),
                bar_space,
            );
        } else {
            pass_text.push(Row::new(vec![
                format!("Max. Elevation: {:.2}deg", pass.pass.get_max_elevation()),
                format!(
                    "Duration: {}sec",
                    (pass.pass.get_los_datetime() - pass.pass.get_aos_datetime()).num_seconds()
                ),
            ]));
            frame.render_widget(Table::new(pass_text, widths), met_inner);
        }
    }
}

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

fn view_ground_track(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let gt_frame = Block::bordered();
    let internal_area = gt_frame.inner(draw_area);
    frame.render_widget(gt_frame, draw_area);
    render_background_map(frame, internal_area);
    if model.current_satellite.is_some() {
        render_tracks(model, frame, internal_area);
    } else {
        render_no_sat_text(frame, internal_area);
    }
}

fn view_sat_data(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let [sat_stats, pass_stats] =
        Layout::vertical([Constraint::Fill(1), Constraint::Percentage(40)]).areas(draw_area);
    render_sat_block(model, frame, sat_stats);
    render_pass_block(model, frame, pass_stats);
}

fn render_pass_block(model: &Model, frame: &mut Frame, draw_area: Rect) {
    let pass_stat_block = Block::bordered();
    let inner_area = pass_stat_block.inner(draw_area);
    frame.render_widget(pass_stat_block, draw_area);
    let satellite = model.current_satellite.as_ref();
    let stations = &model.upcoming_passes;
    if satellite.is_some() && stations.len() > 0 {
    } else {
        let [_, text_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Fill(1),
        ])
        .areas(inner_area);
        frame.render_widget(
            Paragraph::new("Ensure Satellite is selected and Ground Stations loaded to display upcoming pass data")
                .wrap(Wrap { trim: false }),
            text_area,
        );
    }
}

fn render_sat_block(model: &Model, frame: &mut Frame<'_>, draw_area: Rect) {
    let sat_stat_block = Block::bordered();
    let inner_area = sat_stat_block.inner(draw_area);
    frame.render_widget(sat_stat_block, draw_area);
    if let Some(x) = model.current_satellite.as_ref() {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let base_offset = current_time - x.satellite.get_epoch().timestamp();
        let lla = x.satellite.get_sub_point(base_offset);
        let apo_peri = x.satellite.get_apogee_perigee();
        let text = vec![
            Line::from(format!("Satellite Name: {}", x.satellite.get_name())),
            Line::from(format!("Latitude: {:.2} deg", lla.lat)),
            Line::from(format!("Longitude: {:.2} deg", lla.long)),
            Line::from(format!("Local Altitude: {:.2} km", lla.alt)),
            Line::from(format!(
                "Speed: {:.2} Km/s",
                x.satellite.get_speed(base_offset)
            )),
            Line::from(""),
            Line::from(format!("Apogee: {:.2} km", apo_peri.0)),
            Line::from(format!("Perigee: {:.2} km", apo_peri.1)),
            Line::from(format!("Inclination: {:.2} deg", x.metadata.inclination)),
            Line::from(format!(
                "Orbital Period: {:.2} minutes",
                x.satellite.get_period() / 60.0
            )),
            Line::from(""),
            Line::from("Satellite in Sunlight"),
            Line::from("Time to eclipse: XX:XX:XX"),
        ];
        frame.render_widget(List::new(text), inner_area)
    } else {
        let [_, text_area, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(4),
            Constraint::Fill(1),
        ])
        .areas(inner_area);
        frame.render_widget(
            Paragraph::new("Select Satellite to display Telemetry").centered(),
            text_area,
        );
    }
}

fn render_sat_list_details(model: &mut Model, frame: &mut Frame<'_>, detail_area: Rect) {
    let [text_area, tle_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Percentage(30)]).areas(detail_area);

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
                open_norad_id.push('_');
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
                Line::from("Satellite Name: XXXXX"),
                Line::from("Satellite Norad ID:_____"),
                Line::from("Current TLE age: 0 day(s), 0h 0m 0s"),
            ]);
            if model.sat_config.add_sat.editing {
                let current_text = model.sat_config.add_sat.text.clone();
                let mut y_offset: u16 = 1;
                let mut x_offset: u16 = 1;
                if !current_text.is_empty() {
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
                let base_offset = current_time - sat.satellite.get_epoch().timestamp();
                details = Paragraph::new(format!(
                    "Satellite Name: {}\nSatellite Norad ID: {}\nSatellite Catelog ID: {}\nSatellite Registered Owner:{}\nSatellite Launch Date:{}\nCurrent TLE age: {}\n",
                    sat.satellite.get_name(),
                    sat.satellite.get_norad_id(),
                    sat.metadata.object_id,
                    sat.metadata.owner,
                    sat.metadata.launch_date.format("%Y-%m-%d"),
                    strf_seconds(base_offset)
                ));
                tle = Paragraph::new(sat.satellite.get_tle()).block(tle_block);
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
            if model
                .current_satellite
                .as_ref()
                .is_some_and(|y| y.satellite == x.satellite)
            {
                format!("*{}", x.satellite.get_name())
            } else {
                x.satellite.get_name()
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
    let working_seconds;
    if seconds < 0 {
        working_seconds = seconds * -1
    } else {
        working_seconds = seconds
    }
    let minutes = (working_seconds % 3600) / 60;
    let hours = (working_seconds % 86400) / 3600;
    let seconds_new = working_seconds % 60;
    let days = working_seconds / 86400;
    return format!("{} day(s), {}h {}m {}s", days, hours, minutes, seconds_new);
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
    let base_offset = current_time - working_satellites.satellite.get_epoch().timestamp();
    let current_pos = working_satellites.satellite.get_sub_point(base_offset);
    let points: Vec<(f64, f64)> = ((base_offset - 300)..(base_offset + model.sub_point_range))
        .map(|x| {
            let sub_point = working_satellites.satellite.get_sub_point(x);
            (sub_point.long, sub_point.lat)
        })
        .collect();
    let mut prev: Option<f64> = None;
    let mut paths_list: Vec<Dataset> = vec![];
    let mut current_start: usize = 0;
    let mut current_end: usize = 0;
    for i in &points {
        if prev.is_some() && prev.unwrap() > i.0 {
            let dataset = Dataset::default()
                .name(working_satellites.satellite.get_name())
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
        // debug!("current x: {}, last x: {:?}", i.0, prev);
        current_end += 1;
        prev = Some(i.0);
    }
    let dataset = Dataset::default()
        .name(working_satellites.satellite.get_name())
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
    for i in model
        .station_config
        .station_list
        .iter()
        .filter(|x| x.active)
    {
        frame.render_widget(
            Canvas::default()
                .paint(|ctx| {
                    ctx.print(
                        i.station.long,
                        i.station.lat,
                        "+".yellow().into_centered_line(),
                    )
                })
                .x_bounds([-180.0, 180.0])
                .y_bounds([-90.0, 90.0]),
            draw_area,
        );
    }
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
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
