use clap::{Arg,App};

pub fn make_app<'a,'b>() -> App<'a,'b> {
    app_from_crate!()
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
            .default_value("100000000"))
        .arg(Arg::with_name("jobs")
            .value_name("N")
            .long("jobs")
            .short("j")
            .help("Number of parallel jobs")
            .takes_value(true)
            .default_value("4"))
        .arg(Arg::with_name("INPUT")
            .help("Path to flame file to be compiled")
            .required(true)
            .index(1))
        .arg(Arg::with_name("OUTPUT")
            .help("Path to output file (must have .PNG or .JPG extension)")
            .required(true)
            .index(2)
            .validator(check_output))
}

fn check_output(path: String) -> Result<(),String> {
    if path.ends_with(".png") || path.ends_with(".jpg") || path.ends_with(".jpeg") {
        Ok(())
    } else {
        Err(String::from("the output file must have PNG or JPEG format"))
    }
}
