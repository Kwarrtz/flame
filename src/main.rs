#![feature(test)]

extern crate image;
extern crate nalgebra;
extern crate rand;

use nalgebra::{Transform,Matrix3};
use image::ColorType;

mod lib;
use lib::*;

fn main() {
    let trans =
        vec![ (0.01, Transform::from_matrix_unchecked(Matrix3::new(
                0.0, 0.0, 0.0,
                0.0, 0.16, 0.0,
                0.0, 0.0, 1.0)))
        , (0.87, Transform::from_matrix_unchecked(Matrix3::new(
                0.85, 0.04, 0.0,
                -0.04, 0.85, 1.6,
                0.0, 0.0, 1.0)))
        , (0.07, Transform::from_matrix_unchecked(Matrix3::new(
                0.2, -0.26, 0.0,
                0.23, 0.22, 1.6,
                0.0, 0.0, 1.0)))
        , (0.07, Transform::from_matrix_unchecked(Matrix3::new(
                -0.15, 0.28, 0.0,
                0.26, 0.24, 0.44,
                0.0, 0.0, 1.0)))
        ];

    let conf = Config {
        width: 500, height: 700,
        x_min: -2.5, x_max: 3.0,
        y_min: -1.0, y_max: 11.0,
        iterations: 1000000,
        workers: 8,
    };

    let flame = Flame { transformations: trans };

    let buf = run(flame, conf);

    let _ = image::save_buffer(
        "data/test.png",
        &buf[..],
        conf.width as u32,
        conf.height as u32,
        ColorType::Gray(8)
    );
}
