use egui::{Color32, Pos2, Response, Sense, Ui, Vec2};
use crate::riemann::{non_trivial_zeros, trivial_zeros, zeta_derivative_magnitude_on_critical};

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

/// A point already projected to the 2D view plane (before screen fitting),
/// carrying its depth for painter-ordering and depth cueing.
#[derive(Clone, Copy)]
struct ViewPoint {
    vx: f32,
    vy: f32,
    depth: f32,
}

enum Drawable {
    Point {
        p: ViewPoint,
        color: Color32,
        radius: f32,
    },
    Segment {
        a: ViewPoint,
        b: ViewPoint,
        color: Color32,
        width: f32,
    },
}

impl Drawable {
    fn depth(&self) -> f32 {
        match self {
            Drawable::Point { p, .. } => p.depth,
            Drawable::Segment { a, b, .. } => (a.depth + b.depth) * 0.5,
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
    // Fill the remaining space so the view scales with the window.
    let height = ui.available_height().clamp(380.0, 900.0);
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click_and_drag());

    if response.dragged() {
        state.yaw += response.drag_delta().x * 0.01;
        state.pitch = (state.pitch + response.drag_delta().y * 0.01).clamp(-1.45, 1.45);
    }
    if response.double_clicked() {
        *state = Plot3DState::default();
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            state.zoom = (state.zoom * (1.0 + scroll * 0.0015)).clamp(0.4, 4.0);
        }
    }

    let trivial = trivial_zeros(nb_trivial);
    let non_trivial = non_trivial_zeros(im_min, im_max);

    // Data bounds across every plotted point, so the cube auto-fits whatever
    // Im range / rank count is currently selected.
    let mut re_lo = 0.5_f64;
    let mut re_hi = 0.5_f64;
    let mut im_lo = 0.0_f64;
    let mut im_hi = 0.0_f64;
    let mut rk_hi = 1.0_f64;
    for &t in &trivial {
        re_lo = re_lo.min(t);
        re_hi = re_hi.max(t);
    }
    for z in &non_trivial {
        re_lo = re_lo.min(z.re);
        re_hi = re_hi.max(z.re);
        im_lo = im_lo.min(z.im);
        im_hi = im_hi.max(z.im);
        rk_hi = rk_hi.max(z.rank as f64);
    }
    im_hi = im_hi.max(im_max).max(im_lo + 1.0);
    let re_span = (re_hi - re_lo).max(1e-6);
    let im_span = (im_hi - im_lo).max(1e-6);

    // Map data -> centred unit cube [-1, 1]^3 (independent per-axis normalisation
    // keeps the structure readable regardless of the raw ranges).
    let norm = |re: f64, im: f64, rank: f64| -> (f32, f32, f32) {
        let nx = ((re - re_lo) / re_span * 2.0 - 1.0) as f32;
        let ny = ((im - im_lo) / im_span * 2.0 - 1.0) as f32;
        let nz = (rank / rk_hi * 2.0 - 1.0) as f32;
        (nx, ny, nz)
    };

    let (cy, sy) = (state.yaw.cos(), state.yaw.sin());
    let (cp, sp) = (state.pitch.cos(), state.pitch.sin());
    // nx -> horizontal (Re), ny -> vertical (Im), nz -> depth axis (rank).
    let project = |nx: f32, ny: f32, nz: f32| -> ViewPoint {
        let x1 = nx * cy + nz * sy;
        let z1 = -nx * sy + nz * cy;
        let y1 = ny;
        let vy = y1 * cp - z1 * sp;
        let depth = y1 * sp + z1 * cp;
        ViewPoint { vx: x1, vy, depth }
    };

    // Auto-fit: project the 8 cube corners, then scale so they fill the rect.
    let mut min_vx = f32::INFINITY;
    let mut max_vx = f32::NEG_INFINITY;
    let mut min_vy = f32::INFINITY;
    let mut max_vy = f32::NEG_INFINITY;
    let mut min_d = f32::INFINITY;
    let mut max_d = f32::NEG_INFINITY;
    for &sxp in &[-1.0_f32, 1.0] {
        for &syp in &[-1.0_f32, 1.0] {
            for &szp in &[-1.0_f32, 1.0] {
                let v = project(sxp, syp, szp);
                min_vx = min_vx.min(v.vx);
                max_vx = max_vx.max(v.vx);
                min_vy = min_vy.min(v.vy);
                max_vy = max_vy.max(v.vy);
                min_d = min_d.min(v.depth);
                max_d = max_d.max(v.depth);
            }
        }
    }
    let view_w = (max_vx - min_vx).max(1e-3);
    let view_h = (max_vy - min_vy).max(1e-3);
    let margin = 56.0_f32;
    let avail_w = (rect.width() - margin).max(10.0);
    let avail_h = (rect.height() - margin).max(10.0);
    let scale = (avail_w / view_w).min(avail_h / view_h) * state.zoom;
    let view_cx = (min_vx + max_vx) * 0.5;
    let view_cy = (min_vy + max_vy) * 0.5;
    let center = rect.center();
    let to_screen = |v: ViewPoint| -> Pos2 {
        Pos2::new(
            center.x + (v.vx - view_cx) * scale,
            center.y - (v.vy - view_cy) * scale,
        )
    };
    let depth_span = (max_d - min_d).max(1e-3);
    // 0 = far, 1 = near.
    let nearness = |d: f32| -> f32 { ((d - min_d) / depth_span).clamp(0.0, 1.0) };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, Color32::from_rgb(15, 16, 24));

    // --- Wireframe cube + axis grid -------------------------------------------
    let corner = |sx: f32, sy_: f32, sz: f32| to_screen(project(sx, sy_, sz));
    let edge_col = Color32::from_rgb(60, 64, 86);
    let grid_col = Color32::from_rgba_unmultiplied(90, 96, 124, 90);
    let edges = [
        // bottom face
        ((-1., -1., -1.), (1., -1., -1.)),
        ((1., -1., -1.), (1., -1., 1.)),
        ((1., -1., 1.), (-1., -1., 1.)),
        ((-1., -1., 1.), (-1., -1., -1.)),
        // top face
        ((-1., 1., -1.), (1., 1., -1.)),
        ((1., 1., -1.), (1., 1., 1.)),
        ((1., 1., 1.), (-1., 1., 1.)),
        ((-1., 1., 1.), (-1., 1., -1.)),
        // verticals
        ((-1., -1., -1.), (-1., 1., -1.)),
        ((1., -1., -1.), (1., 1., -1.)),
        ((1., -1., 1.), (1., 1., 1.)),
        ((-1., -1., 1.), (-1., 1., 1.)),
    ];
    for (a, b) in edges {
        painter.line_segment(
            [corner(a.0, a.1, a.2), corner(b.0, b.1, b.2)],
            egui::Stroke::new(1.0, edge_col),
        );
    }
    // Floor grid lines (Re x rank plane at the bottom).
    let grid_n = 4;
    for i in 1..grid_n {
        let f = i as f32 / grid_n as f32 * 2.0 - 1.0;
        painter.line_segment(
            [corner(f, -1., -1.), corner(f, -1., 1.)],
            egui::Stroke::new(0.6, grid_col),
        );
        painter.line_segment(
            [corner(-1., -1., f), corner(1., -1., f)],
            egui::Stroke::new(0.6, grid_col),
        );
    }

    // --- Critical line (Re = 1/2) ---------------------------------------------
    {
        let (cx, _, _) = norm(0.5, im_lo, 0.0);
        let a = project(cx, -1.0, 1.0);
        let b = project(cx, 1.0, 1.0);
        painter.line_segment(
            [to_screen(a), to_screen(b)],
            egui::Stroke::new(1.6, Color32::from_rgba_unmultiplied(255, 255, 255, 150)),
        );
    }

    // --- Collect zeros as depth-sorted drawables ------------------------------
    let mut items: Vec<Drawable> = Vec::new();
    // Screen positions + tooltip text for hover queries.
    let mut hover_points: Vec<(Pos2, String)> = Vec::new();

    for (i, &t) in trivial.iter().enumerate() {
        let (nx, ny, nz) = norm(t, 0.0, 0.0);
        let vp = project(nx, ny, nz);
        items.push(Drawable::Point {
            p: vp,
            color: Color32::from_rgb(56, 132, 255),
            radius: 4.5,
        });
        hover_points.push((
            to_screen(vp),
            format!("Zero trivial #{}\ns = {:.0}\nζ(s) = 0", i + 1, t),
        ));
    }

    let total = non_trivial.len() as u32;
    let mut prev: Option<ViewPoint> = None;
    for z in &non_trivial {
        let (nx, ny, nz) = norm(z.re, z.im, z.rank as f64);
        let vp = project(nx, ny, nz);
        if let Some(prev_p) = prev {
            items.push(Drawable::Segment {
                a: prev_p,
                b: vp,
                color: Color32::from_rgba_unmultiplied(220, 60, 60, 140),
                width: 1.4,
            });
        }
        items.push(Drawable::Point {
            p: vp,
            color: rank_color(z.rank, total),
            radius: 5.0,
        });
        let deriv = zeta_derivative_magnitude_on_critical(z.im);
        hover_points.push((
            to_screen(vp),
            format!(
                "Zero non trivial γ{}\nRe(s) = {:.1}\nIm(s) = {:.6}\n|ζ'(½+iγ)| = {:.4}",
                z.rank, z.re, z.im, deriv
            ),
        ));
        prev = Some(vp);
    }

    // Painter's algorithm: far elements first.
    items.sort_by(|a, b| a.depth().partial_cmp(&b.depth()).unwrap_or(std::cmp::Ordering::Equal));

    for item in &items {
        match *item {
            Drawable::Segment { a, b, color, width } => {
                let near = nearness((a.depth + b.depth) * 0.5);
                let w = width * (0.6 + 0.7 * near);
                painter.line_segment([to_screen(a), to_screen(b)], egui::Stroke::new(w, color));
            }
            Drawable::Point { p, color, radius } => {
                let near = nearness(p.depth);
                // Depth cueing: nearer points are larger, brighter and on top.
                let r = radius * (0.6 + 0.7 * near);
                let dimmed = blend_to_bg(color, 0.35 + 0.65 * near);
                let sp_ = to_screen(p);
                painter.circle_filled(sp_, r + 1.3, Color32::from_rgb(15, 16, 24));
                painter.circle_filled(sp_, r, dimmed);
            }
        }
    }

    // --- Axis labels ----------------------------------------------------------
    let label_font = egui::FontId::proportional(12.0);
    let axis_col = Color32::from_gray(210);
    let re_anchor = to_screen(project(0.0, -1.0, -1.0)) + Vec2::new(0.0, 18.0);
    painter.text(re_anchor, egui::Align2::CENTER_CENTER, "Re(s)", label_font.clone(), axis_col);
    let im_anchor = to_screen(project(-1.0, 0.0, -1.0)) + Vec2::new(-20.0, 0.0);
    painter.text(im_anchor, egui::Align2::CENTER_CENTER, "Im(s)", label_font.clone(), axis_col);
    let rk_anchor = to_screen(project(1.0, -1.0, 0.0)) + Vec2::new(18.0, 8.0);
    painter.text(rk_anchor, egui::Align2::CENTER_CENTER, "rang n", label_font.clone(), axis_col);

    // Tick values at the cube extremities.
    let tick_font = egui::FontId::monospace(10.0);
    let tick_col = Color32::from_gray(150);
    painter.text(
        to_screen(project(-1.0, -1.0, -1.0)) + Vec2::new(-2.0, 12.0),
        egui::Align2::RIGHT_CENTER,
        format!("{:.0}", re_lo),
        tick_font.clone(),
        tick_col,
    );
    painter.text(
        to_screen(project(1.0, -1.0, -1.0)) + Vec2::new(2.0, 12.0),
        egui::Align2::LEFT_CENTER,
        format!("{:.1}", re_hi),
        tick_font.clone(),
        tick_col,
    );
    painter.text(
        to_screen(project(-1.0, 1.0, -1.0)) + Vec2::new(-6.0, -2.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{:.0}", im_hi),
        tick_font.clone(),
        tick_col,
    );
    painter.text(
        to_screen(project(-1.0, -1.0, -1.0)) + Vec2::new(-6.0, 2.0),
        egui::Align2::RIGHT_TOP,
        format!("{:.0}", im_lo),
        tick_font.clone(),
        tick_col,
    );

    // --- Overlay help / status ------------------------------------------------
    painter.text(
        rect.left_top() + Vec2::new(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "Glisser = pivoter · molette = zoom · double-clic = reset",
        egui::FontId::proportional(12.0),
        Color32::from_gray(170),
    );
    painter.text(
        rect.right_top() + Vec2::new(-10.0, 8.0),
        egui::Align2::RIGHT_TOP,
        format!("zoom ×{:.2} · {} zeros", state.zoom, non_trivial.len()),
        egui::FontId::monospace(11.0),
        Color32::from_gray(140),
    );

    // --- Hover: nearest zero -> info tooltip ----------------------------------
    if let Some(ptr) = response.hover_pos() {
        let mut best: Option<(f32, &str)> = None;
        for (pos, info) in &hover_points {
            let d = (*pos - ptr).length();
            if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                best = Some((d, info.as_str()));
            }
        }
        if let Some((d, info)) = best {
            if d < 12.0 {
                response.clone().on_hover_text(info);
            }
        }
    }

    response
}

/// Blend a colour toward the dark background; `f` in [0,1], 1 = full colour.
fn blend_to_bg(c: Color32, f: f32) -> Color32 {
    let f = f.clamp(0.0, 1.0);
    let bg = (15.0, 16.0, 24.0);
    Color32::from_rgb(
        (bg.0 + (c.r() as f32 - bg.0) * f) as u8,
        (bg.1 + (c.g() as f32 - bg.1) * f) as u8,
        (bg.2 + (c.b() as f32 - bg.2) * f) as u8,
    )
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
