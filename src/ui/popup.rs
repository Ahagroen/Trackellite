use std::time::{SystemTime, UNIX_EPOCH};

use ratatui::{
    Frame,
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, BorderType, Borders, Clear, List, Paragraph, Row, Table},
};
use tracing::warn;

use crate::structs::{AddSatSel, AppState, GSconfigState, Model};

use super::strf_seconds;

pub fn view_popup_gs_config(model: &mut Model, frame: &mut Frame<'_>) {
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

pub fn view_popup_sat_config(model: &mut Model, frame: &mut Frame) {
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
fn render_sat_list_details(model: &mut Model, frame: &mut Frame<'_>, detail_area: Rect) {
    let [text_area, tle_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Percentage(30)]).areas(detail_area);

    let details;
    let tle;
    let tle_block = Block::new()
        .borders(Borders::all())
        .title_top("TLE")
        .border_type(BorderType::Rounded)
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
                    "Satellite Norad ID: ".into(),
                    norad_id.into(),
                    open_norad_id.slow_blink(),
                ];
            } else {
                norad_id_render = vec![
                    "Satellite Norad ID: ".into(),
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
                    "Satellite Name: {}\nSatellite Norad ID: {}\nSatellite Catelog ID: {}\nSatellite Country: {}\nSatellite Launch Date: {}\nCurrent TLE age: {}\n",
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
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
