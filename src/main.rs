#![feature(test)]

extern crate image;
extern crate nalgebra;
extern crate rand;
#[macro_use]
extern crate clap;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate ansi_term;

use image::ColorType;
use std::fs::File;
use ansi_term::Color::Red;

mod lib;
use lib::*;

fn main() {
    let matches = cli::make_app().get_matches();

    let dims = values_t_or_exit!(matches,"dimensions",usize);
    let iters = value_t_or_exit!(matches,"iterations",usize);
    let workers = value_t_or_exit!(matches,"jobs",usize);
    let path = matches.value_of("OUTPUT").unwrap();
    let input_file = or_exit(File::open(matches.value_of("INPUT").unwrap()));

    let conf = Config {
        width: dims[0], height: dims[1],
        iterations: iters,
        workers: workers,
    };

    let flame = or_exit(Flame::from_file(input_file));

    println!("Compiling flame...");

    let buf = run(flame, conf);

    match image::save_buffer(
        path,
        &buf[..],
        conf.width as u32,
        conf.height as u32,
        ColorType::Gray(8)
    ) {
        Ok(()) => println!("Completed! Output written to '{}'", path),
        Err(e) => eprintln!("Failed to write output: {}", e)
    };
}

fn or_exit<A,E>(x: Result<A,E>) -> A where E: std::fmt::Display {
    match x {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{} {}", Red.bold().paint("error:"), e);
            ::std::process::exit(1)
        }
    }
}
