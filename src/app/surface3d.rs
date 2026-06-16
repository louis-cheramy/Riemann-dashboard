use egui::{Color32, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2};

use crate::app::plots::Plot3DState;
use crate::riemann::{non_trivial_zeros, zeta_complex};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SurfaceField {
    LogMagnitude,
    Real,
    Imag,
    RealAndImag,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SurfaceColor {
    Argument,
    Height,
}

/// One mesh vertex already mapped into the centred unit cube [-1,1]^3.
#[derive(Clone, Copy)]
struct Vert {
    nx: f32,
    ny: f32,
    nz: f32,
    color: Color32,
}

#[derive(Clone, PartialEq)]
struct SurfaceKey {
    smin: i64,
    smax: i64,
    tmin: i64,
    tmax: i64,
    res: usize,
    field: u8,
    color: u8,
}

pub struct SurfaceCache {
    key: SurfaceKey,
    cols: usize,
    rows: usize,
    smin: f64,
    smax: f64,
    tmin: f64,
    tmax: f64,
    z_min: f64,
    z_max: f64,
    /// One or two surfaces (RealAndImag has two), each (cols+1)*(rows+1) verts.
    sheets: Vec<Vec<Vert>>,
}

fn quant(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color32 {
    let c = v * s;
    let hp = (h / 60.0).rem_euclid(6.0);
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

fn lerp_rgb(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// Diverging blue -> white -> red for signed values; `v` in [-1, 1].
fn diverging(v: f32) -> (f32, f32, f32) {
    let blue = (37.0, 99.0, 235.0);
    let white = (244.0, 244.0, 250.0);
    let red = (220.0, 38.0, 38.0);
    if v < 0.0 {
        lerp_rgb(blue, white, (v + 1.0).clamp(0.0, 1.0))
    } else {
        lerp_rgb(white, red, v.clamp(0.0, 1.0))
    }
}

/// Magma-like ramp for magnitudes; `t` in [0, 1].
fn magma(t: f32) -> (f32, f32, f32) {
    let stops = [
        (8.0, 6.0, 30.0),
        (60.0, 15.0, 90.0),
        (140.0, 30.0, 120.0),
        (220.0, 55.0, 95.0),
        (250.0, 130.0, 60.0),
        (252.0, 220.0, 130.0),
    ];
    let t = t.clamp(0.0, 1.0) * (stops.len() - 1) as f32;
    let i = t.floor() as usize;
    let f = t - i as f32;
    if i + 1 < stops.len() {
        lerp_rgb(stops[i], stops[i + 1], f)
    } else {
        stops[stops.len() - 1]
    }
}

fn shade(rgb: (f32, f32, f32), s: f32) -> Color32 {
    let s = s.clamp(0.0, 1.2);
    Color32::from_rgb(
        (rgb.0 * s).clamp(0.0, 255.0) as u8,
        (rgb.1 * s).clamp(0.0, 255.0) as u8,
        (rgb.2 * s).clamp(0.0, 255.0) as u8,
    )
}

#[allow(clippy::too_many_arguments)]
fn rebuild_cache(
    smin: f64,
    smax: f64,
    tmin: f64,
    tmax: f64,
    res: usize,
    field: SurfaceField,
    color: SurfaceColor,
    key: SurfaceKey,
) -> SurfaceCache {
    let cols = res;
    let rows = res;
    let nverts = (cols + 1) * (rows + 1);

    let two_surfaces = matches!(field, SurfaceField::RealAndImag);
    // Raw scalar values per vertex for the primary (and optional secondary) sheet.
    let mut raw1 = vec![0.0f64; nverts];
    let mut raw2 = vec![0.0f64; nverts];
    let mut phase = vec![0.0f64; nverts];

    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;

    for j in 0..=rows {
        let t = tmin + (tmax - tmin) * j as f64 / rows as f64;
        for i in 0..=cols {
            let sigma = smin + (smax - smin) * i as f64 / cols as f64;
            let (mut re, mut im) = zeta_complex(sigma, t);
            // ζ has a pole at s = 1; grid points landing on/near it return
            // non-finite values that otherwise produce wild flickering polygons.
            if !re.is_finite() {
                re = 0.0;
            }
            if !im.is_finite() {
                im = 0.0;
            }
            let idx = j * (cols + 1) + i;
            phase[idx] = im.atan2(re);

            let (mut v1, mut v2) = match field {
                SurfaceField::LogMagnitude => {
                    let mag = (re * re + im * im).sqrt();
                    ((mag + 1e-10).ln().clamp(-30.0, 14.0), 0.0)
                }
                SurfaceField::Real => (re.clamp(-1e6, 1e6), 0.0),
                SurfaceField::Imag => (im.clamp(-1e6, 1e6), 0.0),
                SurfaceField::RealAndImag => (re.clamp(-1e6, 1e6), im.clamp(-1e6, 1e6)),
            };
            if !v1.is_finite() {
                v1 = 0.0;
            }
            if !v2.is_finite() {
                v2 = 0.0;
            }
            raw1[idx] = v1;
            raw2[idx] = v2;
            z_min = z_min.min(v1);
            z_max = z_max.max(v1);
            if two_surfaces {
                z_min = z_min.min(v2);
                z_max = z_max.max(v2);
            }
        }
    }

    // Clamp magnitude/value outliers so a single spike does not flatten the rest.
    if matches!(field, SurfaceField::LogMagnitude) {
        z_min = z_min.max(-12.0);
    } else {
        let bound = z_max.abs().max(z_min.abs()).min(40.0);
        z_min = -bound;
        z_max = bound;
    }
    if z_max <= z_min {
        z_max = z_min + 1.0;
    }
    let z_span = z_max - z_min;

    let norm_z = |v: f64| -> f32 { ((v - z_min) / z_span * 2.0 - 1.0).clamp(-1.0, 1.0) as f32 };
    let norm_x = |i: usize| -> f32 { (i as f64 / cols as f64 * 2.0 - 1.0) as f32 };
    let norm_t = |j: usize| -> f32 { (j as f64 / rows as f64 * 2.0 - 1.0) as f32 };

    let light = {
        let l = (0.45f32, 0.85, 0.30);
        let n = (l.0 * l.0 + l.1 * l.1 + l.2 * l.2).sqrt();
        (l.0 / n, l.1 / n, l.2 / n)
    };

    let make_sheet = |raw: &[f64], alpha: u8, signed: bool| -> Vec<Vert> {
        let mut verts = Vec::with_capacity(nverts);
        for j in 0..=rows {
            for i in 0..=cols {
                let idx = j * (cols + 1) + i;
                let nx = norm_x(i);
                let nz = norm_t(j);
                let ny = norm_z(raw[idx]);

                // Surface normal via finite differences on the value grid.
                let ip = (i + 1).min(cols);
                let im_ = i.saturating_sub(1);
                let jp = (j + 1).min(rows);
                let jm = j.saturating_sub(1);
                let dzdx = norm_z(raw[j * (cols + 1) + ip]) - norm_z(raw[j * (cols + 1) + im_]);
                let dx = norm_x(ip) - norm_x(im_);
                let dzdt = norm_z(raw[jp * (cols + 1) + i]) - norm_z(raw[jm * (cols + 1) + i]);
                let dt = norm_t(jp) - norm_t(jm);
                // tangents: (dx, dzdx, 0) and (0, dzdt, dt) -> normal via cross product
                let nrm = {
                    let a = (dx, dzdx, 0.0f32);
                    let b = (0.0f32, dzdt, dt);
                    let cx = a.1 * b.2 - a.2 * b.1;
                    let cy = a.2 * b.0 - a.0 * b.2;
                    let cz = a.0 * b.1 - a.1 * b.0;
                    let len = (cx * cx + cy * cy + cz * cz).sqrt().max(1e-6);
                    (cx / len, cy / len, cz / len)
                };
                let diff = (nrm.0 * light.0 + nrm.1 * light.1 + nrm.2 * light.2).abs();
                let lum = 0.45 + 0.75 * diff;

                let base = match color {
                    SurfaceColor::Argument => {
                        let hue = ((phase[idx] + std::f64::consts::PI)
                            / (2.0 * std::f64::consts::PI)) as f32
                            * 360.0;
                        let c = hsv_to_rgb(hue, 0.9, 1.0);
                        (c.r() as f32, c.g() as f32, c.b() as f32)
                    }
                    SurfaceColor::Height => {
                        if signed {
                            let v = (raw[idx] / z_max.abs().max(1e-6)) as f32;
                            diverging(v)
                        } else {
                            magma((ny + 1.0) * 0.5)
                        }
                    }
                };
                let mut col = shade(base, lum);
                col = Color32::from_rgba_unmultiplied(col.r(), col.g(), col.b(), alpha);
                verts.push(Vert { nx, ny, nz, color: col });
            }
        }
        verts
    };

    let mut sheets = Vec::new();
    if two_surfaces {
        sheets.push(make_sheet(&raw1, 165, true));
        sheets.push(make_sheet(&raw2, 165, true));
    } else {
        let signed = !matches!(field, SurfaceField::LogMagnitude);
        sheets.push(make_sheet(&raw1, 235, signed));
    }

    SurfaceCache {
        key,
        cols,
        rows,
        smin,
        smax,
        tmin,
        tmax,
        z_min,
        z_max,
        sheets,
    }
}

#[allow(clippy::too_many_arguments)]
pub fn zeta_surface_3d(
    ui: &mut Ui,
    state: &mut Plot3DState,
    cache: &mut Option<SurfaceCache>,
    smin: f64,
    smax: f64,
    tmin: f64,
    tmax: f64,
    res: usize,
    field: SurfaceField,
    color: SurfaceColor,
    show_critical: bool,
) -> Response {
    let height = ui.available_height().clamp(420.0, 1000.0);
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

    let key = SurfaceKey {
        smin: quant(smin),
        smax: quant(smax),
        tmin: quant(tmin),
        tmax: quant(tmax),
        res,
        field: field as u8,
        color: color as u8,
    };
    let needs_rebuild = cache.as_ref().map(|c| c.key != key).unwrap_or(true);
    if needs_rebuild {
        *cache = Some(rebuild_cache(smin, smax, tmin, tmax, res, field, color, key));
    }
    let c = cache.as_ref().unwrap();

    // --- Projection (centred unit cube -> screen, auto-fit) -------------------
    let (cy, sy) = (state.yaw.cos(), state.yaw.sin());
    let (cp, sp) = (state.pitch.cos(), state.pitch.sin());
    let project = |nx: f32, ny: f32, nz: f32| -> (f32, f32, f32) {
        let x1 = nx * cy + nz * sy;
        let z1 = -nx * sy + nz * cy;
        let vy = ny * cp - z1 * sp;
        let depth = ny * sp + z1 * cp;
        (x1, vy, depth)
    };

    let mut min_vx = f32::INFINITY;
    let mut max_vx = f32::NEG_INFINITY;
    let mut min_vy = f32::INFINITY;
    let mut max_vy = f32::NEG_INFINITY;
    for &sxp in &[-1.0f32, 1.0] {
        for &syp in &[-1.0f32, 1.0] {
            for &szp in &[-1.0f32, 1.0] {
                let v = project(sxp, syp, szp);
                min_vx = min_vx.min(v.0);
                max_vx = max_vx.max(v.0);
                min_vy = min_vy.min(v.1);
                max_vy = max_vy.max(v.1);
            }
        }
    }
    let view_w = (max_vx - min_vx).max(1e-3);
    let view_h = (max_vy - min_vy).max(1e-3);
    let margin = 60.0f32;
    let scale = ((rect.width() - margin).max(10.0) / view_w)
        .min((rect.height() - margin).max(10.0) / view_h)
        * state.zoom;
    let view_cx = (min_vx + max_vx) * 0.5;
    let view_cy = (min_vy + max_vy) * 0.5;
    let center = rect.center();
    let to_screen = |v: (f32, f32, f32)| -> Pos2 {
        Pos2::new(
            center.x + (v.0 - view_cx) * scale,
            center.y - (v.1 - view_cy) * scale,
        )
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, Color32::from_rgb(10, 11, 18));

    // --- Wireframe cube -------------------------------------------------------
    let corner = |sx: f32, sy_: f32, sz: f32| to_screen(project(sx, sy_, sz));
    let edge_col = Color32::from_rgb(52, 56, 78);
    for (a, b) in [
        ((-1., -1., -1.), (1., -1., -1.)),
        ((1., -1., -1.), (1., -1., 1.)),
        ((1., -1., 1.), (-1., -1., 1.)),
        ((-1., -1., 1.), (-1., -1., -1.)),
        ((-1., 1., -1.), (1., 1., -1.)),
        ((1., 1., -1.), (1., 1., 1.)),
        ((1., 1., 1.), (-1., 1., 1.)),
        ((-1., 1., 1.), (-1., 1., -1.)),
        ((-1., -1., -1.), (-1., 1., -1.)),
        ((1., -1., -1.), (1., 1., -1.)),
        ((1., -1., 1.), (1., 1., 1.)),
        ((-1., -1., 1.), (-1., 1., 1.)),
    ] {
        painter.line_segment(
            [corner(a.0, a.1, a.2), corner(b.0, b.1, b.2)],
            Stroke::new(1.0, edge_col),
        );
    }

    // --- Surface quads, depth sorted ------------------------------------------
    let cols = c.cols;
    let rows = c.rows;
    struct Cell {
        depth: f32,
        pts: [Pos2; 4],
        color: Color32,
    }
    let mut cells: Vec<Cell> = Vec::with_capacity(cols * rows * c.sheets.len());

    for sheet in &c.sheets {
        for j in 0..rows {
            for i in 0..cols {
                let i00 = j * (cols + 1) + i;
                let i10 = j * (cols + 1) + i + 1;
                let i11 = (j + 1) * (cols + 1) + i + 1;
                let i01 = (j + 1) * (cols + 1) + i;
                let v00 = sheet[i00];
                let v10 = sheet[i10];
                let v11 = sheet[i11];
                let v01 = sheet[i01];

                let p00 = project(v00.nx, v00.ny, v00.nz);
                let p10 = project(v10.nx, v10.ny, v10.nz);
                let p11 = project(v11.nx, v11.ny, v11.nz);
                let p01 = project(v01.nx, v01.ny, v01.nz);
                let depth = (p00.2 + p10.2 + p11.2 + p01.2) * 0.25;

                let s00 = to_screen(p00);
                let s10 = to_screen(p10);
                let s11 = to_screen(p11);
                let s01 = to_screen(p01);
                // Skip any degenerate cell that would otherwise draw a stray streak.
                if ![s00, s10, s11, s01]
                    .iter()
                    .all(|p| p.x.is_finite() && p.y.is_finite())
                {
                    continue;
                }

                // Average the 4 corner colors (alpha preserved).
                let r = (v00.color.r() as u16 + v10.color.r() as u16 + v11.color.r() as u16 + v01.color.r() as u16) / 4;
                let g = (v00.color.g() as u16 + v10.color.g() as u16 + v11.color.g() as u16 + v01.color.g() as u16) / 4;
                let b = (v00.color.b() as u16 + v10.color.b() as u16 + v11.color.b() as u16 + v01.color.b() as u16) / 4;
                let a = v00.color.a();
                cells.push(Cell {
                    depth,
                    pts: [s00, s10, s11, s01],
                    color: Color32::from_rgba_unmultiplied(r as u8, g as u8, b as u8, a),
                });
            }
        }
    }

    cells.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal));
    for cell in &cells {
        painter.add(Shape::convex_polygon(
            cell.pts.to_vec(),
            cell.color,
            Stroke::NONE,
        ));
    }

    // --- Critical line σ = 1/2 + zero markers ---------------------------------
    if show_critical && c.smin <= 0.5 && c.smax >= 0.5 {
        let sx = ((0.5 - c.smin) / (c.smax - c.smin) * 2.0 - 1.0) as f32;
        let a = project(sx, -1.0, -1.0);
        let b = project(sx, 1.2, -1.0);
        // laser plane edge along the back, then a bright vertical marker
        painter.line_segment(
            [to_screen(project(sx, -1.0, -1.0)), to_screen(project(sx, -1.0, 1.0))],
            Stroke::new(1.5, Color32::from_rgba_unmultiplied(0, 230, 255, 160)),
        );
        painter.line_segment(
            [to_screen(a), to_screen(b)],
            Stroke::new(2.0, Color32::from_rgba_unmultiplied(0, 230, 255, 220)),
        );

        let z_span = (c.z_max - c.z_min).max(1e-6);
        for z in non_trivial_zeros(c.tmin.max(0.0), c.tmax) {
            if z.im < c.tmin || z.im > c.tmax {
                continue;
            }
            let nz = (z.im - c.tmin) / (c.tmax - c.tmin) * 2.0 - 1.0;
            // place marker near the floor of the well / on the critical line
            let val = match field {
                SurfaceField::LogMagnitude => c.z_min,
                _ => 0.0,
            };
            let ny = ((val - c.z_min) / z_span * 2.0 - 1.0) as f32;
            let p = to_screen(project(sx, ny, nz as f32));
            painter.circle_filled(p, 5.0, Color32::from_rgb(10, 11, 18));
            painter.circle_filled(p, 3.6, Color32::from_rgb(255, 244, 150));
            painter.circle_stroke(p, 5.5, Stroke::new(1.4, Color32::from_rgba_unmultiplied(255, 230, 120, 180)));
        }
    }

    // --- Axis labels + ticks --------------------------------------------------
    let label = egui::FontId::proportional(12.0);
    let axis_col = Color32::from_gray(205);
    painter.text(
        to_screen(project(0.0, -1.0, -1.0)) + Vec2::new(0.0, 18.0),
        egui::Align2::CENTER_CENTER,
        "σ = Re(s)",
        label.clone(),
        axis_col,
    );
    painter.text(
        to_screen(project(1.0, -1.0, 0.0)) + Vec2::new(22.0, 6.0),
        egui::Align2::CENTER_CENTER,
        "t = Im(s)",
        label.clone(),
        axis_col,
    );
    let z_name = match field {
        SurfaceField::LogMagnitude => "log|ζ|",
        SurfaceField::Real => "Re ζ",
        SurfaceField::Imag => "Im ζ",
        SurfaceField::RealAndImag => "Re ζ / Im ζ",
    };
    painter.text(
        to_screen(project(-1.0, 1.0, -1.0)) + Vec2::new(-18.0, -4.0),
        egui::Align2::RIGHT_BOTTOM,
        z_name,
        label.clone(),
        axis_col,
    );

    let tick = egui::FontId::monospace(10.0);
    let tick_col = Color32::from_gray(150);
    painter.text(
        to_screen(project(-1.0, -1.0, -1.0)) + Vec2::new(-2.0, 12.0),
        egui::Align2::RIGHT_CENTER,
        format!("{:.1}", c.smin),
        tick.clone(),
        tick_col,
    );
    painter.text(
        to_screen(project(1.0, -1.0, -1.0)) + Vec2::new(2.0, 12.0),
        egui::Align2::LEFT_CENTER,
        format!("{:.1}", c.smax),
        tick.clone(),
        tick_col,
    );
    painter.text(
        to_screen(project(1.0, -1.0, 1.0)) + Vec2::new(6.0, 0.0),
        egui::Align2::LEFT_CENTER,
        format!("t={:.0}", c.tmax),
        tick.clone(),
        tick_col,
    );

    // --- Overlay help ---------------------------------------------------------
    painter.text(
        rect.left_top() + Vec2::new(10.0, 8.0),
        egui::Align2::LEFT_TOP,
        "Glisser = pivoter · molette = zoom · double-clic = reset · survol = infos",
        egui::FontId::proportional(12.0),
        Color32::from_gray(170),
    );

    // --- Hover: nearest grid sample -> info tooltip ---------------------------
    if let Some(ptr) = response.hover_pos() {
        let sheet = &c.sheets[0];
        let mut best: Option<(f32, usize, usize)> = None;
        for j in 0..=rows {
            for i in 0..=cols {
                let v = sheet[j * (cols + 1) + i];
                let sp = to_screen(project(v.nx, v.ny, v.nz));
                if !sp.x.is_finite() || !sp.y.is_finite() {
                    continue;
                }
                let d = (sp - ptr).length();
                if best.map(|(bd, _, _)| d < bd).unwrap_or(true) {
                    best = Some((d, i, j));
                }
            }
        }
        if let Some((d, i, j)) = best {
            if d < 16.0 {
                let sigma = c.smin + (c.smax - c.smin) * i as f64 / cols as f64;
                let t = c.tmin + (c.tmax - c.tmin) * j as f64 / rows as f64;
                let (re, im) = zeta_complex(sigma, t);
                let (re, im) = (
                    if re.is_finite() { re } else { 0.0 },
                    if im.is_finite() { im } else { 0.0 },
                );
                let mag = (re * re + im * im).sqrt();
                let arg = im.atan2(re);
                let marker = to_screen(project(
                    sheet[j * (cols + 1) + i].nx,
                    sheet[j * (cols + 1) + i].ny,
                    sheet[j * (cols + 1) + i].nz,
                ));
                painter.circle_stroke(marker, 5.0, Stroke::new(1.6, Color32::WHITE));
                let info = format!(
                    "σ = {sigma:.4}\nt = {t:.4}\nζ(s) = {re:.4} {}{:.4}i\n|ζ(s)| = {mag:.4}\narg(ζ) = {arg:.4} rad\nlog|ζ| = {:.4}",
                    if im >= 0.0 { "+ " } else { "- " },
                    im.abs(),
                    (mag + 1e-10).ln(),
                );
                response.clone().on_hover_text(info);
            }
        }
    }

    response
}
