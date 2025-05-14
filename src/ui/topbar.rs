use chrono::{Datelike, Local, TimeDelta, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Gauge, List, Row, Table},
};

use crate::structs::Model;

use super::strf_seconds_small;

pub fn view_top_bar(model: &Model, frame: &mut Frame, area: Option<Rect>) {
    let draw_area = area.unwrap_or(frame.area());
    let [realtime, center, met_time] = Layout::horizontal([
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
                "Upcoming Pass".into(),
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
            let ratio;
            if current_progress_seconds >= pass_duration {
                ratio = 100.0
            } else {
                ratio = current_progress_seconds as f64 / pass_duration as f64
            }
            frame.render_widget(Table::new(pass_text, widths), table_space);
            frame.render_widget(Gauge::default().ratio(ratio), bar_space);
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
