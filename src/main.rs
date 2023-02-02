use clap::Parser;
use clap_num::si_number;
use std::fs::File;
use std::path::PathBuf;

use flame::*;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to flame descriptor file
    input: PathBuf,
    /// Path to output image (file extension must be JPEG or PNG)
    output: PathBuf,
    /// Number of iterations of the chaos game to run (accepts SI prefixes)
    #[arg(short, long, default_value = "5M", value_parser = si_number::<usize>)]
    iters: usize,
    /// Number of parallel threads
    #[arg(short, long, default_value_t = 5)]
    jobs: usize,
    /// Dimensions (in pixels) of the output image
    #[arg(short, long, number_of_values = 2, default_values_t = [500, 500])]
    #[arg(value_names = ["WIDTH", "HEIGHT"])]
    dims: Vec<usize>,
}

impl Cli {
    fn to_config(&self) -> Config {
        Config {
            width: self.dims[0],
            height: self.dims[1],
            iterations: self.iters,
            workers: self.jobs
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::try_parse()?;
    let input_file = File::open(&cli.input)?;
    let conf = cli.to_config();

    let flame: Flame = FlameSource::from_file(input_file)?.into();

    println!("Rendering flame...");

    let before_run = std::time::Instant::now();

    let buckets = flame.run(conf);

    let dur = before_run.elapsed();

    save_buckets(&buckets, &cli.output)?;

    println!(
        "Completed! Process took {}.{:03} seconds. Output written to '{}'",
        dur.as_secs(),
        dur.subsec_millis(),
        cli.output.display()
    );

    Ok(())
}