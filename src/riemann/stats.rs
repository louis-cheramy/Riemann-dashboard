use std::f64::consts::PI;

/// Nombre asymptotique de zéros avec 0 < Im(s) < T.
pub fn zero_count_asymptotic(t: f64) -> f64 {
    if t < 2.0 {
        return 0.0;
    }
    let u = t / (2.0 * PI);
    u * u.ln() - u
}

/// Espacements normalisés (statistique GUE) entre zéros consécutifs.
pub fn normalized_spacings(gammas: &[f64]) -> Vec<f64> {
    gammas
        .windows(2)
        .filter_map(|w| {
            let gamma_n = w[0];
            if gamma_n < 10.0 {
                return None;
            }
            let gap = w[1] - w[0];
            let mean = 2.0 * PI / (gamma_n / (2.0 * PI)).ln();
            if mean > 0.0 {
                Some(gap / mean)
            } else {
                None
            }
        })
        .collect()
}

/// Distribution de Wigner-Dyson (surmise GUE) pour les espacements normalisés.
pub fn gue_wigner_pdf(s: f64) -> f64 {
    if s < 0.0 {
        return 0.0;
    }
    (32.0 / PI.powi(2)) * s.powi(2) * (-4.0 * s.powi(2) / PI).exp()
}

/// Intégrale logarithmique Li(x) = ∫₂ˣ dt/ln(t).
pub fn logarithmic_integral(x: f64) -> f64 {
    if x <= 2.0 {
        return 0.0;
    }
    let steps = 120;
    let h = (x - 2.0) / steps as f64;
    let mut sum = 0.0;
    for i in 0..=steps {
        let t = 2.0 + i as f64 * h;
        let weight = if i == 0 || i == steps {
            1.0
        } else if i % 2 == 1 {
            4.0
        } else {
            2.0
        };
        sum += weight / t.ln();
    }
    sum * h / 3.0
}
