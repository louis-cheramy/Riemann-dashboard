use std::f64::consts::PI;

fn borwein_coefficients(n: usize) -> Vec<f64> {
    let mut d = vec![0.0; n + 1];
    d[0] = 1.0;
    let mut val = 1.0;
    let mut sum = 1.0;
    let n_f = n as f64;
    for i in 1..=n {
        let i_f = i as f64;
        val = val * 4.0 * (n_f + i_f - 1.0) * (n_f - i_f + 1.0) / ((2.0 * i_f) * (2.0 * i_f - 1.0));
        sum += val;
        d[i] = sum;
    }
    d
}

fn borwein_terms(t: f64) -> usize {
    30.max((t / 2.5) as usize + 25)
}

fn borwein_terms_general(sigma: f64, t: f64) -> usize {
    borwein_terms(t + (0.5 - sigma).abs() * 35.0)
}

fn eta_complex(sigma: f64, t: f64) -> (f64, f64) {
    let n = borwein_terms_general(sigma, t);
    let coeffs = borwein_coefficients(n);
    let dn = coeffs[n];

    let mut re_eta = 0.0;
    let mut im_eta = 0.0;
    for k in 0..n {
        let c = if k % 2 == 0 { 1.0 } else { -1.0 } * (coeffs[k] - coeffs[n]);
        let base = k as f64 + 1.0;
        let mag = base.powf(-sigma);
        let angle = -t * base.ln();
        re_eta += c * mag * angle.cos();
        im_eta += c * mag * angle.sin();
    }
    re_eta /= -dn;
    im_eta /= -dn;
    (re_eta, im_eta)
}

/// ζ(s) pour s = σ + it, Re(s) > 0.
pub fn zeta_complex(sigma: f64, t: f64) -> (f64, f64) {
    if (sigma - 0.5).abs() < 1e-12 {
        return zeta_on_critical_line(t);
    }

    let (re_eta, im_eta) = eta_complex(sigma, t);
    let ln2 = 2.0_f64.ln();
    let pow_re = 2.0_f64.powf(1.0 - sigma);
    let angle = -t * ln2;
    let two_re = pow_re * angle.cos();
    let two_im = pow_re * angle.sin();

    let denom_re = 1.0 - two_re;
    let denom_im = -two_im;
    let denom_norm = denom_re * denom_re + denom_im * denom_im;

    (
        (re_eta * denom_re + im_eta * denom_im) / denom_norm,
        (im_eta * denom_re - re_eta * denom_im) / denom_norm,
    )
}

pub fn zeta_log_magnitude(sigma: f64, t: f64) -> f64 {
    let (re, im) = zeta_complex(sigma, t);
    (re * re + im * im).sqrt().max(1e-300).ln()
}

pub fn zeta_phase(sigma: f64, t: f64) -> f64 {
    let (re, im) = zeta_complex(sigma, t);
    im.atan2(re)
}

/// |ζ'(1/2 + it)| par différences finies.
pub fn zeta_derivative_magnitude_on_critical(t: f64) -> f64 {
    let h = 1e-5;
    let (z0r, z0i) = zeta_on_critical_line(t);
    let (z1r, z1i) = zeta_on_critical_line(t + h);
    let dr = (z1r - z0r) / h;
    let di = (z1i - z0i) / h;
    (dr * dr + di * di).sqrt()
}

/// Riemann-Siegel theta function θ(t), asymptotic expansion.
pub fn riemann_siegel_theta(t: f64) -> f64 {
    0.5 * t * (t / (2.0 * PI)).ln()
        - 0.5 * t
        - PI / 8.0
        + 1.0 / (48.0 * t)
        + 7.0 / (5760.0 * t.powi(3))
        - 31.0 / (645_120.0 * t.powi(5))
        + 127.0 / (9_676_800.0 * t.powi(7))
}

fn zeta_on_critical_line(t: f64) -> (f64, f64) {
    let n = borwein_terms(t);
    let coeffs = borwein_coefficients(n);
    let dn = coeffs[n];

    let mut re_eta = 0.0;
    let mut im_eta = 0.0;
    for k in 0..n {
        let c = if k % 2 == 0 { 1.0 } else { -1.0 } * (coeffs[k] - coeffs[n]);
        let m = (k as f64 + 1.0).powf(-0.5);
        let angle = -t * (k as f64 + 1.0).ln();
        re_eta += c * m * angle.cos();
        im_eta += c * m * angle.sin();
    }
    re_eta /= -dn;
    im_eta /= -dn;

    let ln2 = 2.0_f64.ln();
    let sqrt2 = 2.0_f64.sqrt();
    let denom_re = 1.0 - sqrt2 * (t * ln2).cos();
    let denom_im = sqrt2 * (t * ln2).sin();
    let denom_norm = denom_re * denom_re + denom_im * denom_im;

    let zeta_re = (re_eta * denom_re + im_eta * denom_im) / denom_norm;
    let zeta_im = (im_eta * denom_re - re_eta * denom_im) / denom_norm;
    (zeta_re, zeta_im)
}

/// Hardy Z-function Z(t) = Re(e^{iθ(t)} ζ(1/2 + it)).
pub fn hardy_z(t: f64) -> f64 {
    let theta = riemann_siegel_theta(t);
    let (zeta_re, zeta_im) = zeta_on_critical_line(t);
    theta.cos() * zeta_re - theta.sin() * zeta_im
}

fn refine_zero(mut lo: f64, mut hi: f64) -> f64 {
    for _ in 0..80 {
        let mid = 0.5 * (lo + hi);
        if hardy_z(lo) * hardy_z(mid) <= 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Compute imaginary parts of non-trivial zeros with Im(s) in [im_min, im_max].
pub fn compute_zero_imag_parts(im_min: f64, im_max: f64) -> Vec<f64> {
    if im_max < im_min {
        return Vec::new();
    }

    let mut zeros = Vec::new();
    let mut t = im_min.max(0.1);
    let mut prev = hardy_z(t);
    let step = 0.4;

    while t <= im_max {
        t += step;
        let cur = hardy_z(t);
        if prev * cur < 0.0 {
            let z = refine_zero(t - step, t);
            if z >= im_min && z <= im_max {
                zeros.push(z);
            }
        }
        prev = cur;
    }

    zeros
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardy_z_at_first_zeros() {
        let known = [14.134725, 21.022040, 25.010858, 30.424876, 32.935062];
        for &t in &known {
            assert!(
                hardy_z(t).abs() < 1e-4,
                "Z({t}) = {}",
                hardy_z(t)
            );
        }
    }

    #[test]
    fn compute_first_five_zeros() {
        let zeros = compute_zero_imag_parts(10.0, 40.0);
        assert!(zeros.len() >= 5);
        let expected = [14.134725, 21.022040, 25.010858, 30.424876, 32.935062];
        for (z, &exp) in zeros.iter().take(5).zip(expected.iter()) {
            assert!((z - exp).abs() < 0.01, "got {z}, expected {exp}");
        }
    }
}
