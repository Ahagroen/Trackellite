use chrono::Utc;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Stylize},
    symbols::Marker,
    widgets::{
        Axis, Block, Chart, Dataset, GraphType,
        canvas::{Canvas, Map, MapResolution},
    },
};
use tracing::debug;

use crate::structs::Model;

pub fn view_ground_track(model: &Model, frame: &mut Frame, area: Option<Rect>) {
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
    let current_time = Utc::now().timestamp();
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
    let direction;
    let points_less = points
        .iter()
        .enumerate()
        .filter(|(a, b)| {
            let next_point = points.get(*a + 1);
            if let Some(pt) = next_point {
                b.0 < pt.0
            } else {
                false
            }
        })
        .count();
    let points_more = points
        .iter()
        .enumerate()
        .filter(|(a, b)| {
            let next_point = points.get(*a + 1);
            if let Some(pt) = next_point {
                b.0 > pt.0
            } else {
                false
            }
        })
        .count();
    if points_less > points_more {
        direction = true;
        debug!("Direction: {direction}");
    } else {
        direction = false;
        debug!("Direction: {direction}: points_less: {points_less}, points_more: {points_more}");
    }
    for i in &points {
        if direction {
            if prev.is_some() && prev.unwrap() > i.0 {
                let dataset = Dataset::default()
                    .name(working_satellites.satellite.get_name())
                    .marker(Marker::Braille)
                    .graph_type(GraphType::Line)
                    .cyan()
                    .data(&points[current_start..current_end]);
                paths_list.push(dataset);
                current_start = current_end + 1;
            }
        } else if prev.is_some() && prev.unwrap() < i.0 {
            let dataset = Dataset::default()
                .name(working_satellites.satellite.get_name())
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .cyan()
                .data(&points[current_start..current_end]);
            paths_list.push(dataset);
            current_start = current_end + 1;
        }
        // debug!("current x: {}, last x: {:?}", i.0, prev);
        current_end += 1;
        prev = Some(i.0);
    }
    let dataset = Dataset::default()
        .name(working_satellites.satellite.get_name())
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
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
