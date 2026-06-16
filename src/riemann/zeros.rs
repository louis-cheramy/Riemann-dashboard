use super::zeta::compute_zero_imag_parts;

#[derive(Clone, Copy, Debug)]
pub struct NonTrivialZero {
    pub re: f64,
    pub im: f64,
    pub rank: u32,
}

pub fn trivial_zeros(count: u32) -> Vec<f64> {
    (0..count).map(|i| -2.0 - 2.0 * i as f64).collect()
}

pub fn non_trivial_zeros(im_min: f64, im_max: f64) -> Vec<NonTrivialZero> {
    compute_zero_imag_parts(im_min, im_max)
        .into_iter()
        .enumerate()
        .map(|(i, im)| NonTrivialZero {
            re: 0.5,
            im,
            rank: (i + 1) as u32,
        })
        .collect()
}

/// First `n` non-trivial zeros with Im(s) >= im_min (ranked from im_min).
pub fn first_n_non_trivial(im_min: f64, n: usize) -> Vec<NonTrivialZero> {
    if n == 0 {
        return Vec::new();
    }
    // The k-th zero sits near 2*pi*k / ln(k); grow the window until we have enough.
    let mut hi = im_min.max(0.0) + 25.0 + n as f64 * 3.0;
    for _ in 0..6 {
        let zeros = compute_zero_imag_parts(im_min, hi);
        if zeros.len() >= n {
            return zeros
                .into_iter()
                .take(n)
                .enumerate()
                .map(|(i, im)| NonTrivialZero {
                    re: 0.5,
                    im,
                    rank: (i + 1) as u32,
                })
                .collect();
        }
        hi += 20.0 + n as f64 * 2.0;
    }
    non_trivial_zeros(im_min, hi)
        .into_iter()
        .take(n)
        .collect()
}

/// Default Im(s) range for the dashboard (first zero to ~50th zero).
pub fn default_im_range() -> (f64, f64) {
    let zeros = compute_zero_imag_parts(0.0, 200.0);
    let min = zeros.first().copied().unwrap_or(14.0);
    let max = zeros.get(49).copied().unwrap_or(150.0);
    (min, max)
}
