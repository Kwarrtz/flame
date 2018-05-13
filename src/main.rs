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
use clap::Arg;

mod lib;
use lib::*;

fn main() {
    let app = app_from_crate!()
        .arg(Arg::with_name("dimensions")
            .long("dims")
            .short("d")
            .help("Dimensions (in pixels) of the output image")
            .takes_value(true)
            .multiple(true)
            .require_delimiter(true)
            .number_of_values(2)
            .value_names(&["WIDTH","HEIGHT"])
            .default_value("500,500"))
        .arg(Arg::with_name("iterations")
            .value_name("N")
            .long("iters")
            .short("i")
            .help("Number of iterations of the chaos game to run")
            .takes_value(true)
            .default_value("8"))
        .arg(Arg::with_name("jobs")
            .value_name("N")
            .long("jobs")
            .short("j")
            .help("Number of parallel jobs")
            .takes_value(true)
            .default_value("4"))
        .arg(Arg::with_name("INPUT")
            // .help("Path to flame file to be compiled"
            .required(true)
            .index(1))
        .arg(Arg::with_name("OUTPUT")
            .help("Path to output file (must have .PNG or .JPG extension)")
            .required(true)
            .index(2)
            .validator(check_output));

    let matches = app.get_matches();

    let dims = values_t_or_exit!(matches,"dimensions",usize);
    let iters = value_t_or_exit!(matches,"iterations",u32);
    let workers = value_t_or_exit!(matches,"jobs",usize);
    let path = matches.value_of("OUTPUT").unwrap();
    let input_file = or_exit(File::open(matches.value_of("INPUT").unwrap()));

    let conf = Config {
        width: dims[0], height: dims[1],
        iterations: usize::pow(10, iters),
        workers: workers,
    };

    let flame = or_exit(Flame::from_file(input_file));

    println!("Compiling flame...");

    let before_run = std::time::Instant::now();

    let buf = run(flame, conf);

    let dur = before_run.elapsed();

    match image::save_buffer(
        path,
        &buf[..],
        conf.width as u32,
        conf.height as u32,
        ColorType::Gray(8)
    ) {
        Ok(()) => println!(
            "Completed! Process took {}.{:03} seconds. Output written to '{}'",
            dur.as_secs(),
            dur.subsec_millis(),
            path),
        Err(e) => eprintln!("Failed to write output: {}", e)
    };
}

fn or_exit<A,E>(x: Result<A,E>) -> A where E: std::fmt::Display {
    match x {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{} {}", Red.bold().paint("error:"), e);
            std::process::exit(1)
        }
    }
}

fn check_output(path: String) -> Result<(),String> {
    if path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg") {
        Ok(())
    } else {
        Err(String::from("the output file must have PNG or JPEG format"))
    }
}
