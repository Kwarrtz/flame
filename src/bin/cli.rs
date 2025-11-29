use clap::Parser;
use clap_num::si_number;
use std::path::PathBuf;

use flame::*;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to flame descriptor file.
    input: PathBuf,
    /// Path to output image (file extension must be JPEG or PNG).
    output: PathBuf,
    /// Number of iterations of the chaos game to run (accepts SI postfixes).
    ///
    /// Higher values reduce noise but take longer to run.
    #[arg(short, long, default_value = "100M", value_parser = si_number::<usize>)]
    iters: usize,
    /// Number of parallel threads.
    #[arg(short, long, default_value_t = 10)]
    threads: usize,
    /// Dimensions (in pixels) of the output image.
    #[arg(short, long, number_of_values = 2, default_values_t = [500, 500])]
    #[arg(value_names = ["WIDTH", "HEIGHT"])]
    dims: Vec<usize>,
    /// Gamma correction factor.
    #[arg(short, long, default_value_t = 2.2)]
    gamma: f64,
    /// Gamma color vibrancy (between 0 and 1).
    ///
    /// When this value is zero, gamma correction is applied independently to each color channel,
    /// which can lead to washed out colors. When it is one, gamma correction only affects luminance.
    /// Values between 0 and 1 interpolate geometrically between these extremes.
    #[arg(short, long, default_value_t = 0.0)]
    vibrancy: f64,
    /// Output a grayscale image, ignoring any specified color information.
    #[arg(short='G', long)]
    grayscale: bool,
    /// Super-sampling radius.
    #[arg(short, long, default_value_t = 0)]
    samples: usize,
}

impl Cli {
    fn run_config(&self) -> RunConfig {
        RunConfig {
            width: self.dims[0],
            height: self.dims[1],
            iters: self.iters,
            threads: self.threads,
        }
    }

    fn render_config(&self) -> RenderConfig {
        RenderConfig {
            width: self.dims[0],
            height: self.dims[1],
            gamma: self.gamma,
            vibrancy: self.vibrancy,
            filter_radius: self.samples
        }
    }
}

fn run() -> Result<(), FlameError> {
    let cli = Cli::parse();
    let run_cfg = cli.run_config();
    let render_cfg = cli.render_config();

    let flame: Flame = Flame::from_file(cli.input)?;

    println!("Rendering flame...");

    let before_run = std::time::Instant::now();

    let buffer = flame.run(run_cfg);
    let img_buffer = buffer.render(render_cfg);

    let dur = before_run.elapsed();

    img_buffer.to_dynamic8(cli.grayscale).save(&cli.output)?;

    println!(
        "Completed! Rendered in {}.{:03} seconds. Output written to '{}'",
        dur.as_secs(),
        dur.subsec_millis(),
        cli.output.display()
    );

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
