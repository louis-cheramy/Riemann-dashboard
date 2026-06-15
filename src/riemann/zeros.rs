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

/// Default Im(s) range for the dashboard (first zero to ~50th zero).
pub fn default_im_range() -> (f64, f64) {
    let zeros = compute_zero_imag_parts(0.0, 200.0);
    let min = zeros.first().copied().unwrap_or(14.0);
    let max = zeros.get(49).copied().unwrap_or(150.0);
    (min, max)
}
