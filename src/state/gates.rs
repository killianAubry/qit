use num_complex::Complex64;

pub fn hadamard() -> [[Complex64; 2]; 2] {
    let inv_sqrt2 = 1.0 / 2.0_f64.sqrt();

    [
        [
            Complex64::new(inv_sqrt2, 0.0),
            Complex64::new(inv_sqrt2, 0.0),
        ],
        [
            Complex64::new(inv_sqrt2, 0.0),
            Complex64::new(-inv_sqrt2, 0.0),
        ],
    ]
}

pub fn x_gate() -> [[Complex64; 2]; 2] {
    [
        [
            Complex64::new(0.0, 0.0),
            Complex64::new(1.0, 0.0),
        ],
        [
            Complex64::new(1.0, 0.0),
            Complex64::new(0.0, 0.0),
        ],
    ]
}