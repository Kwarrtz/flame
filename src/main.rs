#![feature(test)]

extern crate image;
extern crate nalgebra;
extern crate rand;
#[macro_use]
extern crate clap;

use nalgebra::{Transform,Matrix3};
use image::ColorType;

mod lib;
use lib::*;

fn main() {
    let matches = cli::make_app().get_matches();

    let dims = values_t_or_exit!(matches,"dimensions",usize);
    let iters = value_t_or_exit!(matches,"iterations",usize);
    let workers = value_t_or_exit!(matches,"jobs",usize);
    let path = matches.value_of("OUTPUT").unwrap();
    let quiet = matches.is_present("quiet");
    let _verbose = matches.is_present("verbose");

    let conf = Config {
        width: dims[0], height: dims[1],
        iterations: iters,
        workers: workers,
    };

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


    let flame = Flame {
        transformations: trans,
        x_min: -2.5, x_max: 3.0,
        y_min: -1.0, y_max: 11.0,
    };


    if !quiet { println!("Compiling flame...") }

    let buf = run(flame, conf);

    match image::save_buffer(
        path,
        &buf[..],
        conf.width as u32,
        conf.height as u32,
        ColorType::Gray(8)
    ) {
        Ok(()) => println!("Completed! Output written to '{}'", path),
        Err(e) => println!("Failed to write output: {}", e)
    };


}
