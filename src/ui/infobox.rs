use chrono::Utc;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::Line,
    widgets::{Block, List, Paragraph, Wrap},
};

use crate::Model;

pub fn view_sat_data(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let [sat_stats, pass_stats] =
        Layout::vertical([Constraint::Fill(1), Constraint::Percentage(60)]).areas(draw_area);
    render_sat_block(model, frame, sat_stats);
    render_pass_block(model, frame, pass_stats);
}

fn render_pass_block(model: &Model, frame: &mut Frame, draw_area: Rect) {
    let pass_stat_block = Block::bordered();
    let inner_area = pass_stat_block.inner(draw_area);
    frame.render_widget(pass_stat_block, draw_area);
    let satellite = model.current_satellite.as_ref();
    let stations = &model.upcoming_passes;
    if satellite.is_some() && !stations.is_empty() {
        let upcoming_pass = &model.upcoming_passes[0];
        let pointing = satellite.unwrap().satellite.get_look_angle(
            &upcoming_pass.station,
            satellite
                .unwrap()
                .satellite
                .seconds_since_epoch(&Utc::now()),
        );
        let mut list_text = vec![
            Line::from(format!("Next Pass Station: {}", upcoming_pass.station.name)),
            Line::from(format!("Elevation: {:.2}deg", pointing.elevation)),
            Line::from(format!("Azimuth: {:.2}deg", pointing.azimuth)),
            Line::from(format!("Range: {:.2}km", pointing.range)),
            Line::from(""), //WIll be local time at Ground station
            Line::from(""),
            Line::from("Upcoming Passes").centered().underlined(),
        ];
        for i in model.upcoming_passes.iter().take(5) {
            list_text.push(Line::from(format!(
                "{}: AOS {}(UTC)",
                i.station.name,
                i.pass.get_aos_datetime().format("%y-%m-%d %H:%M")
            )));
            list_text.push(Line::from(format!(
                "    Max. El: {:.1}deg, Duration: {}sec",
                i.pass.get_max_elevation(),
                i.pass.get_duration_sec()
            )));
            list_text.push("".into())
        }

        frame.render_widget(List::new(list_text), inner_area);
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
        let current_time = Utc::now().timestamp();
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
            // Line::from("Satellite in Sunlight"),
            // Line::from("Time to eclipse: XX:XX:XX"),
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
