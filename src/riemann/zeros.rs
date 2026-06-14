pub const NON_TRIVIAL_IMAG_PARTS: &[f64] = &[
    14.134725, 21.022040, 25.010858, 30.424876, 32.935062, 37.586178, 40.918719, 43.327073,
    48.005150, 49.773832, 52.970321, 56.446247, 59.347044, 60.831780, 65.112544, 67.079811,
    69.546402, 72.067158, 75.704690, 77.144840, 79.337375, 82.910381, 84.735493, 87.425275,
    88.809111, 92.491899, 94.651344, 95.870634, 98.831194, 101.317851, 103.725538, 105.446623,
    107.168611, 111.029536, 111.874659, 114.320221, 116.226680, 118.790782, 121.370125,
    122.946829, 124.256818, 127.516684, 129.578704, 131.087688, 133.497737, 134.756510,
    138.116042, 139.736209, 141.123707, 143.111846,
];

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
    NON_TRIVIAL_IMAG_PARTS
        .iter()
        .copied()
        .filter(|&im| im >= im_min && im <= im_max)
        .enumerate()
        .map(|(i, im)| NonTrivialZero {
            re: 0.5,
            im,
            rank: (i + 1) as u32,
        })
        .collect()
}
