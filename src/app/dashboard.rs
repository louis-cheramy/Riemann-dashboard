use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use egui_plot::{Bar, BarChart, Legend, Plot};
use egui::{Color32, RichText};

use crate::app::plots::{self, Plot3DState};
use crate::app::riemann_viz::{self, HeatmapCache};
use crate::primes::{generate_primes, PrimeStore, SegmentProgress, DEFAULT_PRIME_FILE, resolve_prime_path};
use crate::riemann::{default_im_range, non_trivial_zeros};

#[derive(Clone, Copy, PartialEq, Eq)]
enum GraphKind {
    Histogram,
    Spacing,
    Riemann,
    IntegerDisplay,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RiemannView {
    Explorer,
    Spacings,
    Density,
    Primes,
    Classic2d,
    Classic3d,
}

enum GenerateMsg {
    Progress(SegmentProgress),
    Done(Result<u64, String>),
}

pub struct DashboardApp {
    prime_path: PathBuf,
    store: Option<PrimeStore>,
    load_error: Option<String>,

    range_min: u64,
    range_max: u64,
    graph: GraphKind,
    bins: usize,
    spacing_bins: usize,

    // Riemann
    im_min: f64,
    im_max: f64,
    nb_trivial: u32,
    riemann_view: RiemannView,
    animate_2d: bool,
    anim_index: usize,
    anim_last: Instant,
    plot3d: Plot3DState,
    heatmap_cache: Option<HeatmapCache>,
    show_zero_labels: bool,
    color_zeros_by_derivative: bool,
    primes_link_max: u64,

    // Integer display
    display_min: u64,
    display_max: u64,

    // Generation
    gen_limit_str: String,
    gen_running: bool,
    gen_log: Vec<String>,
    gen_rx: Option<Receiver<GenerateMsg>>,
}

impl DashboardApp {
    pub fn new() -> Self {
        let prime_path = resolve_prime_path();
        let (store, load_error) = match PrimeStore::open(&prime_path) {
            Ok(s) => (Some(s), None),
            Err(e) => (None, Some(e.to_string())),
        };

        let (range_min, range_max, display_min, display_max) = if let Some(ref s) = store {
            let first = s.first().unwrap_or(2);
            let last = s.last().unwrap_or(first);
            (
                first,
                (first + 100_000).min(last),
                first,
                (first + 100).min(last),
            )
        } else {
            (2, 100_002, 2, 102)
        };

        let (default_im_min, default_im_max) = default_im_range();
        Self {
            prime_path,
            store,
            load_error,
            range_min,
            range_max,
            graph: GraphKind::Histogram,
            bins: 50,
            spacing_bins: 30,
            im_min: default_im_min,
            im_max: default_im_max,
            nb_trivial: 20,
            riemann_view: RiemannView::Explorer,
            animate_2d: false,
            anim_index: 0,
            anim_last: Instant::now(),
            plot3d: Plot3DState::default(),
            heatmap_cache: None,
            show_zero_labels: true,
            color_zeros_by_derivative: true,
            primes_link_max: 10_000,
            display_min,
            display_max,
            gen_limit_str: "1000000".into(),
            gen_running: false,
            gen_log: Vec::new(),
            gen_rx: None,
        }
    }

    fn reload_store(&mut self) {
        match PrimeStore::open(&self.prime_path) {
            Ok(s) => {
                self.load_error = None;
                if let (Some(first), Some(last)) = (s.first(), s.last()) {
                    self.range_min = first;
                    self.range_max = (first + 100_000).min(last);
                    self.display_min = first;
                    self.display_max = (first + 100).min(last);
                }
                self.store = Some(s);
            }
            Err(e) => {
                self.load_error = Some(e.to_string());
                self.store = None;
            }
        }
    }

    fn poll_generation(&mut self) {
        let mut done = false;
        let mut reload = false;
        let mut messages = Vec::new();

        if let Some(rx) = &self.gen_rx {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
        }

        for msg in messages {
            match msg {
                GenerateMsg::Progress(p) => {
                    self.gen_log.push(format!(
                        "Segment {} : {} -> {} ({} premiers)",
                        p.segment,
                        format_num(p.start),
                        format_num(p.end),
                        format_num(p.count),
                    ));
                }
                GenerateMsg::Done(result) => {
                    done = true;
                    match result {
                        Ok(total) => {
                            self.gen_log.push(format!(
                                "Termine : {} nombres premiers enregistres.",
                                format_num(total)
                            ));
                            reload = true;
                        }
                        Err(e) => self.gen_log.push(format!("Erreur : {e}")),
                    }
                }
            }
        }

        if reload {
            self.reload_store();
        }
        if done {
            self.gen_running = false;
            self.gen_rx = None;
        }
    }

    fn start_generation(&mut self) {
        let limit: u64 = match self.gen_limit_str.replace('_', "").replace(' ', "").parse() {
            Ok(n) if n >= 2 => n,
            _ => {
                self.gen_log.push("Borne invalide (entier >= 2 requis).".into());
                return;
            }
        };

        self.gen_running = true;
        self.gen_log.clear();
        self.prime_path = resolve_prime_path();
        self.gen_log.push(format!(
            "Generation des premiers jusqu'a {}...",
            format_num(limit)
        ));
        self.gen_log.push(format!("Fichier : {}", self.prime_path.display()));

        // Liberer le mmap sinon Windows bloque l'ecriture du fichier
        self.store = None;
        self.load_error = None;

        let path = self.prime_path.clone();
        let (tx, rx) = mpsc::channel();
        self.gen_rx = Some(rx);

        thread::spawn(move || {
            let tx_progress = tx.clone();
            let result = generate_primes(&path, limit, |p| {
                let _ = tx_progress.send(GenerateMsg::Progress(p));
            })
            .map_err(|e| format!("{e:#}"));
            let _ = tx.send(GenerateMsg::Done(result));
        });
    }

    fn show_histogram(&mut self, ui: &mut egui::Ui) {
        let Some(store) = &self.store else { return };
        let lo = self.range_min.min(self.range_max);
        let hi = self.range_min.max(self.range_max);
        let values = store.collect_range(lo, hi, 500_000);
        let (centers, heights) = plots::histogram_bars(&values, self.bins);

        if centers.is_empty() {
            ui.label("Aucune donnee dans l'intervalle.");
            return;
        }

        let bar_width = if centers.len() > 1 {
            (centers[1] - centers[0]) * 0.9
        } else {
            1.0
        };
        let bars: Vec<Bar> = centers
            .iter()
            .zip(heights.iter())
            .map(|(&x, &h)| Bar::new(x, h).width(bar_width))
            .collect();

        Plot::new("hist")
            .legend(Legend::default())
            .height(360.0)
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(BarChart::new(bars).name("Repartition"));
            });
    }

    fn show_spacing(&mut self, ui: &mut egui::Ui) {
        let Some(store) = &self.store else { return };
        let lo = self.range_min.min(self.range_max);
        let hi = self.range_min.max(self.range_max);
        let count = store.range_len(lo, hi);
        if count < 2 {
            ui.colored_label(Color32::YELLOW, "Intervalle trop petit (au moins 2 premiers).");
            return;
        }
        let values = store.collect_range(lo, hi, 500_000);
        let (centers, heights) = plots::spacing_histogram(&values, self.spacing_bins);
        let width = if centers.len() > 1 {
            centers[1] - centers[0]
        } else {
            1.0
        };
        let bars: Vec<Bar> = centers
            .iter()
            .zip(heights.iter())
            .map(|(&x, &h)| Bar::new(x, h).width(width * 0.9))
            .collect();

        Plot::new("spacing")
            .height(360.0)
            .show(ui, |plot_ui| {
                plot_ui.bar_chart(BarChart::new(bars).name("Espacements"));
            });
    }

    fn show_riemann(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Fonction zeta de Riemann : zeros triviaux (entiers pairs negatifs) et non triviaux sur Re(s)=1/2.",
        );
        ui.label(
            egui::RichText::new("Hypothese de Riemann : tous les zeros non triviaux ont Re(s) = 1/2.")
                .italics()
                .color(Color32::from_rgb(147, 197, 253)),
        );

        let prev_im = (self.im_min, self.im_max);
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.im_min).speed(0.5).prefix("Im min: "));
            ui.add(egui::DragValue::new(&mut self.im_max).speed(0.5).prefix("Im max: "));
            ui.add(egui::Slider::new(&mut self.nb_trivial, 5..=30).text("Triviaux"));
        });
        if (self.im_min, self.im_max) != prev_im {
            if let Some(cache) = &mut self.heatmap_cache {
                cache.invalidate();
            }
        }

        if self.im_min > self.im_max {
            ui.colored_label(Color32::RED, "Im min doit etre <= Im max.");
            return;
        }

        let nt = non_trivial_zeros(self.im_min, self.im_max);
        ui.horizontal(|ui| {
            ui.label(format!("{} zeros non triviaux calcules", nt.len()));
            ui.checkbox(&mut self.show_zero_labels, "Numeroter gamma_n");
            ui.checkbox(&mut self.color_zeros_by_derivative, "Couleur |zeta'|");
            ui.checkbox(&mut self.animate_2d, "Animation");
        });

        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.riemann_view, RiemannView::Explorer, "Explorateur");
            ui.selectable_value(&mut self.riemann_view, RiemannView::Spacings, "Espacements GUE");
            ui.selectable_value(&mut self.riemann_view, RiemannView::Density, "Densite N(T)");
            ui.selectable_value(&mut self.riemann_view, RiemannView::Primes, "Lien premiers");
            ui.selectable_value(&mut self.riemann_view, RiemannView::Classic2d, "Plan 2D");
            ui.selectable_value(&mut self.riemann_view, RiemannView::Classic3d, "Plan 3D");
        });

        ui.separator();

        let anim = if self.animate_2d && !nt.is_empty() {
            if self.anim_last.elapsed() > Duration::from_millis(400) {
                self.anim_index = (self.anim_index + 1).min(nt.len());
                self.anim_last = Instant::now();
            }
            if self.anim_index >= nt.len() {
                self.anim_index = 0;
            }
            Some(self.anim_index.max(1))
        } else {
            self.anim_index = 0;
            None
        };

        match self.riemann_view {
            RiemannView::Explorer => {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Carte log|zeta(s)| — phase arg(zeta)").strong());
                        riemann_viz::heatmap_explorer(
                            ui,
                            &mut self.heatmap_cache,
                            self.im_min,
                            self.im_max,
                            &nt,
                            anim,
                            self.show_zero_labels,
                            self.color_zeros_by_derivative,
                        );
                    });
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(340.0);
                        ui.label(egui::RichText::new("Espacements normalises vs GUE").strong());
                        riemann_viz::spacing_gue_plot(ui, &nt);
                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Densite des zeros").strong());
                        riemann_viz::zero_density_plot(ui, &nt, self.im_max);
                    });
                });
                ui.add_space(8.0);
                egui::CollapsingHeader::new("Table des zeros (gamma_n, |zeta'|, espacements)")
                    .default_open(true)
                    .show(ui, |ui| {
                        let visible = anim.unwrap_or(nt.len());
                        riemann_viz::zeros_table(ui, &nt, visible);
                    });
            }
            RiemannView::Spacings => {
                ui.label("Les espacements entre zeros suivent la distribution GUE (matrices hermitiennes aleatoires).");
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Histogramme vs Wigner surmise").strong());
                        riemann_viz::spacing_gue_plot(ui, &nt);
                    });
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Suite gamma_{n+1} - gamma_n").strong());
                        riemann_viz::spacing_sequence_plot(ui, &nt);
                    });
                });
                riemann_viz::zeros_table(ui, &nt, nt.len());
            }
            RiemannView::Density => {
                ui.label("Comparaison du nombre reel de zeros avec la formule asymptotique N(T) ~ T/2pi ln(T/2pi) - T/2pi.");
                riemann_viz::zero_density_plot(ui, &nt, self.im_max);
                ui.add_space(8.0);
                riemann_viz::spacing_sequence_plot(ui, &nt);
            }
            RiemannView::Primes => {
                ui.label(
                    "Les oscillations de pi(x) - Li(x) encodent l'information des zeros de zeta (formule explicite).",
                );
                if let Some(store) = &self.store {
                    ui.add(
                        egui::Slider::new(&mut self.primes_link_max, 500..=50_000)
                            .logarithmic(true)
                            .text("Borne x"),
                    );
                    riemann_viz::primes_connection_plot(ui, store, self.primes_link_max);
                } else {
                    ui.colored_label(Color32::YELLOW, "Chargez un fichier de nombres premiers.");
                }
            }
            RiemannView::Classic2d => {
                plots::riemann_plot_2d(ui, self.im_min, self.im_max, self.nb_trivial, anim);
            }
            RiemannView::Classic3d => {
                plots::riemann_plot_3d(ui, &mut self.plot3d, self.im_min, self.im_max, self.nb_trivial);
            }
        }
    }

    fn show_integers(&mut self, ui: &mut egui::Ui) {
        let Some(store) = &self.store else { return };
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut self.display_min).prefix("Min: "));
            ui.add(egui::DragValue::new(&mut self.display_max).prefix("Max: "));
        });

        let lo = self.display_min.min(self.display_max);
        let hi = self.display_min.max(self.display_max);
        if hi - lo > 2000 {
            ui.colored_label(
                Color32::YELLOW,
                "Intervalle limite a 2000 entiers pour la fluidite.",
            );
            return;
        }

        let mut prime_set = std::collections::HashSet::new();
        store.for_each_in_range(lo, hi, |p| {
            prime_set.insert(p);
        });

        egui::ScrollArea::vertical()
            .max_height(320.0)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for n in lo..=hi {
                        let is_prime = prime_set.contains(&n);
                        let color = if is_prime {
                            Color32::from_rgb(220, 38, 38)
                        } else {
                            ui.visuals().text_color()
                        };
                        ui.label(RichText::new(format!("{n} ")).color(color).monospace());
                    }
                });
            });
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_generation();
        if self.gen_running {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        if self.animate_2d && self.graph == GraphKind::Riemann {
            ctx.request_repaint_after(Duration::from_millis(350));
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.heading("Riemann Dashboard");
            ui.label("Analyse des nombres premiers et zeros de la fonction zeta");
        });

        egui::SidePanel::left("side")
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.heading("Donnees");
                if let Some(store) = &self.store {
                    ui.label(format!("Premiers charges : {}", format_num(store.count())));
                    if let (Some(a), Some(b)) = (store.first(), store.last()) {
                        ui.label(format!("Plage : {} -> {}", format_num(a), format_num(b)));
                    }
                } else if let Some(err) = &self.load_error {
                    ui.colored_label(Color32::RED, err);
                }

                ui.separator();
                ui.heading("Intervalle");
                if self.store.is_some() {
                    ui.add(egui::DragValue::new(&mut self.range_min).prefix("Min: "));
                    ui.add(egui::DragValue::new(&mut self.range_max).prefix("Max: "));
                    if let Some(store) = &self.store {
                        let lo = self.range_min.min(self.range_max);
                        let hi = self.range_min.max(self.range_max);
                        ui.label(format!(
                            "Premiers dans l'intervalle : {}",
                            format_num(store.range_len(lo, hi))
                        ));
                    }
                }

                ui.separator();
                ui.heading("Generer les premiers");
                ui.label(format!("Sortie : {}", self.prime_path.display()));
                ui.text_edit_singleline(&mut self.gen_limit_str);
                ui.add_enabled_ui(!self.gen_running, |ui| {
                    if ui.button("Lancer la generation").clicked() {
                        self.start_generation();
                    }
                });
                if self.gen_running {
                    ui.spinner();
                    ui.label("Generation en cours...");
                }
                egui::ScrollArea::vertical()
                    .max_height(160.0)
                    .show(ui, |ui| {
                        for line in &self.gen_log {
                            ui.label(line);
                        }
                    });

                ui.separator();
                if ui.button("Recharger le fichier .bin").clicked() {
                    self.reload_store();
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.store.is_none() {
                ui.colored_label(
                    Color32::YELLOW,
                    format!(
                        "Fichier '{}' introuvable. Generez-le depuis le panneau de gauche.",
                        DEFAULT_PRIME_FILE
                    ),
                );
            }

            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.graph, GraphKind::Histogram, "Histogramme");
                ui.selectable_value(&mut self.graph, GraphKind::Spacing, "Espacements");
                ui.selectable_value(&mut self.graph, GraphKind::Riemann, "Zeros Riemann");
                ui.selectable_value(&mut self.graph, GraphKind::IntegerDisplay, "Entiers");
            });

            ui.separator();

            match self.graph {
                GraphKind::Histogram => {
                    ui.add(egui::Slider::new(&mut self.bins, 10..=200).text("Bins"));
                    self.show_histogram(ui);
                }
                GraphKind::Spacing => {
                    ui.add(egui::Slider::new(&mut self.spacing_bins, 5..=100).text("Bins"));
                    self.show_spacing(ui);
                }
                GraphKind::Riemann => self.show_riemann(ui),
                GraphKind::IntegerDisplay => self.show_integers(ui),
            }
        });
    }
}

fn format_num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}
