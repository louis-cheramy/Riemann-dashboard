use std::f64::consts::PI;

use egui::{Color32, Pos2, Rect, Sense, Ui, Vec2};
use egui_plot::{Bar, BarChart, Legend, Line, Plot, PlotPoints, Points};
use crate::primes::PrimeStore;
use crate::riemann::{
    gue_wigner_pdf, logarithmic_integral, normalized_spacings, zero_count_asymptotic, NonTrivialZero,
    zeta_derivative_magnitude_on_critical, zeta_log_magnitude, zeta_phase,
};

#[derive(Clone, PartialEq)]
struct HeatmapKey {
    im_min: f64,
    im_max: f64,
    re_min: f64,
    re_max: f64,
    cols: u32,
    rows: u32,
}

pub struct HeatmapCache {
    key: HeatmapKey,
    colors: Vec<Color32>,
}

impl HeatmapCache {
    pub fn invalidate(&mut self) {
        self.key.cols = 0;
    }
}

fn phase_heatmap_color(sigma: f64, t: f64) -> Color32 {
    let phase = zeta_phase(sigma, t);
    let log_mag = zeta_log_magnitude(sigma, t);
    let hue = ((phase + PI) / (2.0 * PI)) as f32 * 360.0;
    let zero_strength = (-log_mag).clamp(0.0, 10.0) / 10.0;
    let value = (0.12 + 0.88 * zero_strength) as f32;
    hsv_to_rgb(hue, 0.92, value)
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

fn derivative_color(rank: u32, deriv: f64, max_deriv: f64) -> Color32 {
    let t = if max_deriv > 0.0 {
        (deriv / max_deriv).clamp(0.0, 1.0) as f32
    } else {
        0.5
    };
    let hue = 40.0 + (rank as f32 * 17.0) % 280.0;
    hsv_to_rgb(hue, 0.75, 0.35 + 0.65 * t)
}

pub fn heatmap_explorer(
    ui: &mut Ui,
    cache: &mut Option<HeatmapCache>,
    im_min: f64,
    im_max: f64,
    zeros: &[NonTrivialZero],
    animate_count: Option<usize>,
    show_labels: bool,
    color_by_derivative: bool,
) {
    let width = ui.available_width().min(720.0);
    let height = 360.0;
    let cols = 96u32;
    let rows = 64u32;
    let re_min = 0.05;
    let re_max = 0.95;

    let key = HeatmapKey {
        im_min,
        im_max,
        re_min,
        re_max,
        cols,
        rows,
    };

    let needs_rebuild = cache
        .as_ref()
        .map(|c| c.key != key)
        .unwrap_or(true);

    if needs_rebuild {
        let mut colors = Vec::with_capacity((cols * rows) as usize);
        for row in 0..rows {
            for col in 0..cols {
                let sigma = re_min + (re_max - re_min) * col as f64 / (cols - 1).max(1) as f64;
                let t = im_min + (im_max - im_min) * (rows - 1 - row) as f64 / (rows - 1).max(1) as f64;
                colors.push(phase_heatmap_color(sigma, t));
            }
        }
        *cache = Some(HeatmapCache { key, colors });
    }

    let colors = &cache.as_ref().unwrap().colors;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 12, 18));

    let cell_w = rect.width() / cols as f32;
    let cell_h = rect.height() / rows as f32;

    for row in 0..rows {
        for col in 0..cols {
            let idx = (row * cols + col) as usize;
            let r = Rect::from_min_size(
                Pos2::new(rect.min.x + col as f32 * cell_w, rect.min.y + row as f32 * cell_h),
                Vec2::new(cell_w + 0.5, cell_h + 0.5),
            );
            painter.rect_filled(r, 0.0, colors[idx]);
        }
    }

    let map_x = |sigma: f64| -> f32 {
        rect.min.x + ((sigma - re_min) / (re_max - re_min)) as f32 * rect.width()
    };
    let map_y = |t: f64| -> f32 {
        rect.max.y - ((t - im_min) / (im_max - im_min)) as f32 * rect.height()
    };

    let crit_x = map_x(0.5);
    painter.line_segment(
        [Pos2::new(crit_x, rect.min.y), Pos2::new(crit_x, rect.max.y)],
        egui::Stroke::new(2.5, Color32::from_rgba_unmultiplied(255, 255, 255, 210)),
    );

    let visible = animate_count.unwrap_or(zeros.len()).min(zeros.len());
    let max_deriv = zeros
        .iter()
        .take(visible)
        .map(|z| zeta_derivative_magnitude_on_critical(z.im))
        .fold(0.0_f64, f64::max);

    for z in zeros.iter().take(visible) {
        let p = Pos2::new(map_x(z.re), map_y(z.im));
        let color = if color_by_derivative {
            derivative_color(z.rank, zeta_derivative_magnitude_on_critical(z.im), max_deriv)
        } else {
            Color32::WHITE
        };
        painter.circle_filled(p, 5.0, Color32::BLACK);
        painter.circle_stroke(p, 5.0, egui::Stroke::new(2.0, color));
        if show_labels && z.rank <= 12 {
            painter.text(
                p + Vec2::new(7.0, -7.0),
                egui::Align2::LEFT_BOTTOM,
                format!("γ{}", subscript_num(z.rank)),
                egui::FontId::monospace(11.0),
                Color32::WHITE,
            );
        }
    }

    painter.text(
        rect.left_top() + Vec2::new(8.0, 6.0),
        egui::Align2::LEFT_TOP,
        "log|ζ(s)| — teinte = arg(ζ), luminosite = proximite d'un zero",
        egui::FontId::proportional(12.0),
        Color32::from_gray(220),
    );
    painter.text(
        Pos2::new(crit_x + 4.0, rect.min.y + 4.0),
        egui::Align2::LEFT_TOP,
        "Re(s)=1/2",
        egui::FontId::proportional(11.0),
        Color32::WHITE,
    );

    let _ = response;
}

fn subscript_num(n: u32) -> String {
    const SUB: &[char] = &['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉'];
    n.to_string()
        .chars()
        .map(|c| SUB[c.to_digit(10).unwrap_or(0) as usize])
        .collect()
}

pub fn spacing_gue_plot(ui: &mut Ui, zeros: &[NonTrivialZero]) {
    let gammas: Vec<f64> = zeros.iter().map(|z| z.im).collect();
    let spacings = normalized_spacings(&gammas);

    if spacings.is_empty() {
        ui.label("Au moins 2 zeros necessaires pour les espacements.");
        return;
    }

    let bins = 16;
    let max_s = spacings.iter().copied().fold(0.0_f64, f64::max).max(3.0);
    let mut counts = vec![0u32; bins];
    for &s in &spacings {
        let idx = ((s / max_s) * (bins - 1) as f64).round() as usize;
        counts[idx.min(bins - 1)] += 1;
    }
    let bar_width = max_s / bins as f64;
    let bars: Vec<Bar> = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| Bar::new((i as f64 + 0.5) * bar_width, c as f64).width(bar_width * 0.88))
        .collect();

    let gue_line: PlotPoints = (0..80)
        .map(|i| {
            let s = max_s * i as f64 / 79.0;
            let scale = spacings.len() as f64 * bar_width;
            [s, gue_wigner_pdf(s) * scale]
        })
        .collect();

    Plot::new("gue_spacing")
        .height(220.0)
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.bar_chart(BarChart::new(bars).name("Espacements normalises"));
            plot_ui.line(
                Line::new(gue_line)
                    .color(Color32::from_rgb(250, 204, 21))
                    .width(2.5)
                    .name("GUE (Wigner)"),
            );
        });
}

pub fn zero_density_plot(ui: &mut Ui, zeros: &[NonTrivialZero], im_max: f64) {
    if zeros.is_empty() {
        ui.label("Aucun zero a afficher.");
        return;
    }

    let actual: PlotPoints = zeros
        .iter()
        .map(|z| [z.im, z.rank as f64])
        .collect();

    let asymptotic: PlotPoints = (0..100)
        .map(|i| {
            let t = zeros.first().map(|z| z.im).unwrap_or(10.0)
                + (im_max - zeros.first().map(|z| z.im).unwrap_or(10.0)) * i as f64 / 99.0;
            [t, zero_count_asymptotic(t)]
        })
        .collect();

    Plot::new("zero_density")
        .height(220.0)
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.points(
                Points::new(actual)
                    .radius(4.0)
                    .color(Color32::from_rgb(220, 38, 38))
                    .name("N reels"),
            );
            plot_ui.line(
                Line::new(asymptotic)
                    .color(Color32::from_rgb(96, 165, 250))
                    .width(2.0)
                    .style(egui_plot::LineStyle::Dashed { length: 5.0 })
                    .name("N(T) ~ T/2pi ln(T/2pi) - T/2pi"),
            );
        });
}

pub fn spacing_sequence_plot(ui: &mut Ui, zeros: &[NonTrivialZero]) {
    if zeros.len() < 2 {
        return;
    }
    let pts: PlotPoints = zeros
        .windows(2)
        .enumerate()
        .map(|(i, w)| [i as f64 + 1.0, w[1].im - w[0].im])
        .collect();

    Plot::new("gap_sequence")
        .height(200.0)
        .show(ui, |plot_ui| {
            plot_ui.line(
                Line::new(pts)
                    .color(Color32::from_rgb(168, 85, 247))
                    .width(1.5)
                    .name("gamma_{n+1} - gamma_n"),
            );
        });
}

pub fn primes_connection_plot(ui: &mut Ui, store: &PrimeStore, max_x: u64) {
    let max_x = max_x.clamp(100, 50_000);
    let step = (max_x / 200).max(1);

    let mut pi_pts = Vec::new();
    let mut li_pts = Vec::new();
    let mut osc_pts = Vec::new();

    let mut x = 2u64;
    while x <= max_x {
        let xf = x as f64;
        let pi = store.range_len(2, x) as f64;
        let li = logarithmic_integral(xf);
        pi_pts.push([xf, pi]);
        li_pts.push([xf, li]);
        osc_pts.push([xf, pi - li]);
        x += step;
    }

    Plot::new("primes_link")
        .height(240.0)
        .legend(Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.line(
                Line::new(PlotPoints::new(pi_pts))
                    .color(Color32::from_rgb(220, 38, 38))
                    .width(2.0)
                    .name("pi(x)"),
            );
            plot_ui.line(
                Line::new(PlotPoints::new(li_pts))
                    .color(Color32::from_rgb(59, 130, 246))
                    .width(2.0)
                    .style(egui_plot::LineStyle::Dashed { length: 6.0 })
                    .name("Li(x)"),
            );
        });

    Plot::new("primes_osc")
        .height(160.0)
        .show(ui, |plot_ui| {
            plot_ui.line(
                Line::new(PlotPoints::new(osc_pts))
                    .color(Color32::from_rgb(34, 197, 94))
                    .width(1.5)
                    .name("pi(x) - Li(x)"),
            );
            plot_ui.hline(egui_plot::HLine::new(0.0).color(Color32::GRAY));
        });
}

pub fn zeros_table(ui: &mut Ui, zeros: &[NonTrivialZero], visible: usize) {
    egui::Grid::new("zeros_grid")
        .num_columns(4)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(egui::RichText::new("n").strong());
            ui.label(egui::RichText::new("gamma_n").strong());
            ui.label(egui::RichText::new("|zeta'|").strong());
            ui.label(egui::RichText::new("espacement").strong());
            ui.end_row();
            for (i, z) in zeros.iter().take(visible).enumerate() {
                let deriv = zeta_derivative_magnitude_on_critical(z.im);
                let gap = if i > 0 {
                    format!("{:.4}", z.im - zeros[i - 1].im)
                } else {
                    "—".into()
                };
                ui.monospace(format!("{}", z.rank));
                ui.monospace(format!("{:.6}", z.im));
                ui.monospace(format!("{:.4}", deriv));
                ui.monospace(gap);
                ui.end_row();
            }
        });
}
