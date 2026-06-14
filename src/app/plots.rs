use egui::{Color32, Pos2, Response, Sense, Ui, Vec2};
use crate::riemann::{non_trivial_zeros, trivial_zeros};

pub struct Plot3DState {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}

impl Default for Plot3DState {
    fn default() -> Self {
        Self {
            yaw: 0.6,
            pitch: 0.35,
            zoom: 1.0,
        }
    }
}

pub fn riemann_plot_3d(
    ui: &mut Ui,
    state: &mut Plot3DState,
    im_min: f64,
    im_max: f64,
    nb_trivial: u32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 420.0), Sense::drag());
    if response.dragged() {
        state.yaw += response.drag_delta().x * 0.01;
        state.pitch = (state.pitch + response.drag_delta().y * 0.01).clamp(-1.2, 1.2);
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            state.zoom = (state.zoom * (1.0 - scroll * 0.001)).clamp(0.4, 3.0);
        }
    }

    let trivial = trivial_zeros(nb_trivial);
    let non_trivial = non_trivial_zeros(im_min, im_max);
    let max_rank = non_trivial.len().max(5) as f64;

    let to_screen = |x: f64, y: f64, z: f64| -> Pos2 {
        let (cy, sy) = (state.yaw.cos(), state.yaw.sin());
        let (cp, sp) = (state.pitch.cos(), state.pitch.sin());
        let x1 = x * cy as f64 - z * sy as f64;
        let z1 = x * sy as f64 + z * cy as f64;
        let y1 = y * cp as f64 - z1 * sp as f64;
        let _depth = y * sp as f64 + z1 * cp as f64;

        let scale = (rect.width().min(rect.height()) * 0.003 * state.zoom) as f64;
        let cx = rect.center().x as f64;
        let cy = rect.center().y as f64;
        Pos2::new(
            (cx + x1 * scale) as f32,
            (cy - y1 * scale) as f32,
        )
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(247, 247, 250));

    // axes labels
    painter.text(
        rect.left_top() + Vec2::new(8.0, 8.0),
        egui::Align2::LEFT_TOP,
        "3D: Re(s), Im(s), rang n — glisser pour pivoter, molette pour zoom",
        egui::FontId::proportional(13.0),
        Color32::DARK_GRAY,
    );

    // critical line
    let p0 = to_screen(0.5, im_min, 0.0);
    let p1 = to_screen(0.5, im_max, max_rank);
    painter.line_segment([p0, p1], egui::Stroke::new(1.5, Color32::GRAY));

    for &t in &trivial {
        let p = to_screen(t, 0.0, 0.0);
        painter.circle_filled(p, 5.0, Color32::from_rgb(37, 99, 235));
    }

    let mut prev: Option<Pos2> = None;
    for z in &non_trivial {
        let p = to_screen(z.re, z.im, z.rank as f64);
        let color = rank_color(z.rank, non_trivial.len() as u32);
        painter.circle_filled(p, 4.5, color);
        if let Some(prev_p) = prev {
            painter.line_segment([prev_p, p], egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(220, 38, 38, 120)));
        }
        prev = Some(p);
    }

    response
}

fn rank_color(rank: u32, total: u32) -> Color32 {
    let t = if total <= 1 {
        0.5
    } else {
        (rank - 1) as f32 / (total - 1) as f32
    };
    let hue = t * 280.0;
    hsv_to_rgb(hue, 0.85, 0.95)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let hp = (h / 60.0) % 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

pub fn riemann_plot_2d(
    ui: &mut Ui,
    im_min: f64,
    im_max: f64,
    nb_trivial: u32,
    animate_index: Option<usize>,
) {
    use egui_plot::{Legend, Line, Plot, PlotPoints, Points};

    let trivial = trivial_zeros(nb_trivial);
    let mut non_trivial = non_trivial_zeros(im_min, im_max);
    if let Some(n) = animate_index {
        non_trivial.truncate(n.min(non_trivial.len()));
    }

    Plot::new("riemann_2d")
        .legend(Legend::default())
        .height(420.0)
        .show(ui, |plot_ui| {
            plot_ui.line(
                Line::new(PlotPoints::new(vec![[0.5, im_min], [0.5, im_max]]))
                    .color(Color32::GRAY)
                    .style(egui_plot::LineStyle::Dashed { length: 6.0 })
                    .name("Droite critique"),
            );

            let trivial_pts: PlotPoints = trivial.iter().map(|&x| [x, 0.0]).collect();
            plot_ui.points(
                Points::new(trivial_pts)
                    .radius(6.0)
                    .color(Color32::from_rgb(37, 99, 235))
                    .name("Triviaux"),
            );

            if !non_trivial.is_empty() {
                let line_pts: PlotPoints = non_trivial.iter().map(|z| [z.re, z.im]).collect();
                let point_pts: PlotPoints = non_trivial.iter().map(|z| [z.re, z.im]).collect();
                plot_ui.line(
                    Line::new(line_pts)
                        .color(Color32::from_rgba_unmultiplied(220, 38, 38, 90))
                        .width(1.5)
                        .name("Non triviaux"),
                );
                plot_ui.points(
                    Points::new(point_pts)
                        .radius(5.0)
                        .color(Color32::from_rgb(220, 38, 38))
                        .name("Zeros"),
                );
            }
        });
}

pub fn histogram_bars(values: &[u64], bins: usize) -> (Vec<f64>, Vec<f64>) {
    if values.is_empty() || bins == 0 {
        return (Vec::new(), Vec::new());
    }
    let min = *values.iter().min().unwrap();
    let max = *values.iter().max().unwrap();
    if min == max {
        return (vec![min as f64], vec![values.len() as f64]);
    }
    let mut counts = vec![0u64; bins];
    let range = (max - min) as f64;
    for &v in values {
        let idx = (((v - min) as f64 / range) * (bins - 1) as f64).round() as usize;
        counts[idx.min(bins - 1)] += 1;
    }
    let centers: Vec<f64> = (0..bins)
        .map(|i| min as f64 + (i as f64 + 0.5) * range / bins as f64)
        .collect();
    let heights: Vec<f64> = counts.iter().map(|&c| c as f64).collect();
    (centers, heights)
}

pub fn spacing_histogram(values: &[u64], bins: usize) -> (Vec<f64>, Vec<f64>) {
    if values.len() < 2 {
        return (Vec::new(), Vec::new());
    }
    let gaps: Vec<u64> = values.windows(2).map(|w| w[1] - w[0]).collect();
    histogram_bars(&gaps, bins)
}
