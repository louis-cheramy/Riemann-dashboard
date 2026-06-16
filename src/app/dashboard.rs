use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use egui_plot::{Bar, BarChart, Legend, Plot};
use egui::{Color32, RichText};

use crate::app::plots::{self, Plot3DState};
use crate::app::riemann_viz::{self, HeatmapCache};
use crate::app::surface3d::{self, SurfaceCache, SurfaceColor, SurfaceField};
use crate::primes::{generate_primes, PrimeStore, SegmentProgress, DEFAULT_PRIME_FILE, resolve_prime_path};
use crate::riemann::{default_im_range, first_n_non_trivial};

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
    Surface3d,
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
    nb_non_trivial: usize,
    show_explanations: bool,
    riemann_view: RiemannView,
    animate_2d: bool,
    anim_index: usize,
    anim_last: Instant,
    plot3d: Plot3DState,
    heatmap_cache: Option<HeatmapCache>,
    show_zero_labels: bool,
    color_zeros_by_derivative: bool,
    primes_link_max: u64,

    // Surface 3D de zeta(s)
    surface_cam: Plot3DState,
    surface_cache: Option<SurfaceCache>,
    surface_field: SurfaceField,
    surface_color: SurfaceColor,
    surface_res: usize,
    surf_sigma_min: f64,
    surf_sigma_max: f64,
    surf_t_min: f64,
    surf_t_max: f64,
    surface_show_critical: bool,

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
            nb_trivial: 15,
            nb_non_trivial: 50,
            show_explanations: false,
            riemann_view: RiemannView::Explorer,
            animate_2d: false,
            anim_index: 0,
            anim_last: Instant::now(),
            plot3d: Plot3DState::default(),
            heatmap_cache: None,
            show_zero_labels: true,
            color_zeros_by_derivative: true,
            primes_link_max: 10_000,
            surface_cam: Plot3DState::default(),
            surface_cache: None,
            surface_field: SurfaceField::LogMagnitude,
            surface_color: SurfaceColor::Argument,
            surface_res: 80,
            surf_sigma_min: -2.0,
            surf_sigma_max: 2.0,
            surf_t_min: 0.0,
            surf_t_max: 50.0,
            surface_show_critical: true,
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
            ui.add(
                egui::Slider::new(&mut self.nb_non_trivial, 1..=120)
                    .text("Zeros non triviaux"),
            );
            ui.checkbox(&mut self.show_explanations, "Explications");
        });

        let nt = first_n_non_trivial(self.im_min, self.nb_non_trivial);
        self.im_max = nt
            .last()
            .map(|z| z.im + 2.0)
            .unwrap_or(self.im_min + 10.0);
        if (self.im_min, self.im_max) != prev_im {
            if let Some(cache) = &mut self.heatmap_cache {
                cache.invalidate();
            }
        }

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} zeros non triviaux · Im ∈ [{:.1}, {:.1}]",
                nt.len(),
                self.im_min,
                self.im_max
            ));
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
            ui.selectable_value(&mut self.riemann_view, RiemannView::Classic3d, "Colonne 3D");
            ui.selectable_value(&mut self.riemann_view, RiemannView::Surface3d, "Surface ζ 3D");
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

        let (exp_title, exp_body) = explanation_for(self.riemann_view);
        let show_exp = self.show_explanations;
        ui.horizontal_top(|ui| {
            let diag_w = if show_exp {
                (ui.available_width() - 326.0).max(360.0)
            } else {
                ui.available_width()
            };
            ui.vertical(|ui| {
                ui.set_max_width(diag_w);
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
            RiemannView::Surface3d => self.show_surface_3d(ui),
                }
            });
            if show_exp {
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_max_width(312.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.heading(exp_title);
                        ui.add_space(4.0);
                        ui.label(exp_body);
                    });
                });
            }
        });
    }

    fn show_surface_3d(&mut self, ui: &mut egui::Ui) {
        ui.label(
            "Surface 3D de ζ(s) sur le plan complexe. Les zeros sont les puits (log|ζ|) ou les croisements Re=0 ∩ Im=0.",
        );

        ui.horizontal_wrapped(|ui| {
            ui.label("Champ :");
            ui.selectable_value(&mut self.surface_field, SurfaceField::LogMagnitude, "log|ζ|");
            ui.selectable_value(&mut self.surface_field, SurfaceField::Real, "Re(ζ)");
            ui.selectable_value(&mut self.surface_field, SurfaceField::Imag, "Im(ζ)");
            ui.selectable_value(&mut self.surface_field, SurfaceField::RealAndImag, "Re & Im");
            ui.separator();
            ui.label("Couleur :");
            ui.selectable_value(&mut self.surface_color, SurfaceColor::Argument, "arg(ζ)");
            ui.selectable_value(&mut self.surface_color, SurfaceColor::Height, "hauteur");
        });

        ui.horizontal_wrapped(|ui| {
            ui.add(egui::DragValue::new(&mut self.surf_sigma_min).speed(0.05).prefix("σ min: "));
            ui.add(egui::DragValue::new(&mut self.surf_sigma_max).speed(0.05).prefix("σ max: "));
            ui.add(egui::DragValue::new(&mut self.surf_t_min).speed(0.5).prefix("t min: "));
            ui.add(egui::DragValue::new(&mut self.surf_t_max).speed(0.5).prefix("t max: "));
            ui.add(egui::Slider::new(&mut self.surface_res, 30..=140).text("Resolution"));
            ui.checkbox(&mut self.surface_show_critical, "Droite critique + zeros");
        });

        self.surf_sigma_min = self.surf_sigma_min.clamp(-10.0, 0.95);
        self.surf_sigma_max = self.surf_sigma_max.clamp(self.surf_sigma_min + 0.1, 10.0);
        self.surf_t_min = self.surf_t_min.clamp(0.0, 1000.0);
        self.surf_t_max = self.surf_t_max.clamp(self.surf_t_min + 1.0, 1000.0);

        ui.separator();

        surface3d::zeta_surface_3d(
            ui,
            &mut self.surface_cam,
            &mut self.surface_cache,
            self.surf_sigma_min,
            self.surf_sigma_max,
            self.surf_t_min,
            self.surf_t_max,
            self.surface_res,
            self.surface_field,
            self.surface_color,
            self.surface_show_critical,
        );
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

fn explanation_for(view: RiemannView) -> (&'static str, &'static str) {
    match view {
        RiemannView::Explorer => (
            "Explorateur log|ζ|",
            "Carte de couleur du plan complexe. La teinte code la phase arg(ζ(s)) et la \
luminosite la proximite d'un zero (la ou |ζ| chute vers 0). La droite blanche marque \
Re(s) = 1/2.\n\nUtilite : reperer d'un coup d'oeil ou se trouvent les zeros et constater \
qu'ils s'alignent tous sur la droite critique. Les deux graphiques de droite resument la \
statistique des espacements (GUE) et la densite des zeros.",
        ),
        RiemannView::Spacings => (
            "Espacements GUE",
            "Histogramme des ecarts entre zeros consecutifs, normalises pour avoir un ecart \
moyen de 1, compare a la loi de Wigner des matrices aleatoires (GUE).\n\nUtilite : illustrer \
la conjecture de Montgomery-Odlyzko : les zeros de ζ se repoussent exactement comme les \
valeurs propres de grandes matrices hermitiennes aleatoires. C'est un lien profond entre \
theorie des nombres et physique quantique.",
        ),
        RiemannView::Density => (
            "Densite N(T)",
            "Nombre de zeros dont la partie imaginaire est inferieure a T, compare a la \
formule asymptotique N(T) ≈ (T/2π)·ln(T/2π) − T/2π.\n\nUtilite : montrer que les zeros \
deviennent de plus en plus denses quand on monte en hauteur, et que leur comptage suit une \
loi tres precise issue de la formule de Riemann-von Mangoldt.",
        ),
        RiemannView::Primes => (
            "Lien avec les premiers",
            "Comparaison de π(x) (nombre de premiers ≤ x) avec Li(x), et trace de leur \
difference.\n\nUtilite : rendre visible la formule explicite de Riemann : les zeros de ζ \
controlent les oscillations de la repartition des nombres premiers. Chaque zero ajoute une \
onde dans le terme d'erreur π(x) − Li(x).",
        ),
        RiemannView::Classic2d => (
            "Plan complexe 2D",
            "Position des zeros dans le plan : zeros triviaux (bleus) sur l'axe reel negatif \
et zeros non triviaux (rouges) tous sur la droite Re(s) = 1/2.\n\nUtilite : la representation \
la plus directe de l'hypothese de Riemann. Tous les zeros non triviaux connus sont alignes \
sur une unique verticale.",
        ),
        RiemannView::Classic3d => (
            "Colonne 3D des zeros",
            "Les zeros non triviaux places en 3D : X = Re(s), Y = Im(s), Z = rang n du \
zero.\n\nUtilite : on voit que, quel que soit le rang, les points restent colles a la paroi \
Re(s) = 1/2 : ils forment une colonne verticale parfaite. Survolez un point pour lire son \
rang, sa hauteur γ et |ζ'|. Glisser pour pivoter, molette pour zoomer.",
        ),
        RiemannView::Surface3d => (
            "Surface 3D de ζ(s)",
            "Vraie surface de ζ sur le plan complexe.\n\n• log|ζ| : les zeros sont les puits \
qui plongent vers −∞.\n• Re(ζ) / Im(ζ) : les zeros sont les points ou les deux nappes \
valent 0 en meme temps (leurs croisements).\n• Couleur arg(ζ) : la phase tourne comme un \
tourbillon autour de chaque zero.\n\nLa droite critique Re = 1/2 est materialisee par un \
trait cyan et les zeros connus par des marqueurs lumineux. Survolez la surface pour lire \
σ, t, ζ(s), |ζ| et arg(ζ).",
        ),
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
