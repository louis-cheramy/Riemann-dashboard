use std::io::{self, Write};

use anyhow::Result;
use clap::{Parser, Subcommand};

use riemann_dashboard::app::DashboardApp;
use riemann_dashboard::primes::{generate_primes, SegmentProgress, DEFAULT_PRIME_FILE, resolve_prime_path};

#[derive(Parser)]
#[command(name = "riemann-dashboard")]
#[command(about = "Dashboard des nombres premiers et zeros de Riemann")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Ouvre l'interface graphique (defaut)
    Gui,
    /// Genere le fichier binaire de nombres premiers
    Generate {
        /// Borne maximale (ex: 10000000000)
        limit: u64,
        /// Chemin de sortie
        #[arg(short, long, default_value = DEFAULT_PRIME_FILE)]
        output: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None | Some(Commands::Gui) => run_gui(),
        Some(Commands::Generate { limit, output }) => {
            run_generate(limit, &output)?;
            Ok(())
        }
    }
}

fn run_gui() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("Riemann Dashboard"),
        ..Default::default()
    };
    eframe::run_native(
        "Riemann Dashboard",
        options,
        Box::new(|_cc| Ok(Box::new(DashboardApp::new()))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

fn run_generate(limit: u64, output: &str) -> Result<()> {
    let path = if output == DEFAULT_PRIME_FILE {
        resolve_prime_path()
    } else {
        std::path::PathBuf::from(output)
    };
    println!("Recherche de tous les nombres premiers jusqu'a {limit}...");
    println!("Fichier de sortie : {}", path.display());
    let total = generate_primes(&path, limit, |p: SegmentProgress| {
        println!(
            "Segment {} : {} -> {} traite ({} premiers).",
            p.segment,
            fmt(p.start),
            fmt(p.end),
            fmt(p.count),
        );
        let _ = io::stdout().flush();
    })?;
    println!(
        "Termine : {} premiers enregistres dans '{}'.",
        fmt(total),
        path.display()
    );
    Ok(())
}

fn fmt(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}
