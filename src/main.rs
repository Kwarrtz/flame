use clap::Parser;
use clap_num::si_number;
use std::fs::File;
use std::path::PathBuf;

use flame::core::*;
use flame::file::*;

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
    #[arg(short, long, default_value = "5M", value_parser = si_number::<usize>)]
    iters: usize,
    /// Number of parallel threads.
    #[arg(short, long, default_value_t = 10)]
    threads: usize,
    /// Dimensions (in pixels) of the output image.
    #[arg(short, long, number_of_values = 2, default_values_t = [500, 500])]
    #[arg(value_names = ["WIDTH", "HEIGHT"])]
    dims: Vec<usize>,
    /// Output a grayscale image, ignoring any specified color information.
    #[arg(short='G', long)]
    grayscale: bool,
    /// Gamma correction factor.
    #[arg(short, long, default_value_t = 2.2)]
    gamma: f64,
    /// Preserve the true ratios of the color channels.
    /// 
    /// When enabled, instead of scaling each color channel independently to 
    /// fit the 8-bit range, they will be scaled by a common factor.
    #[arg(short, long)]
    preserve_color: bool,
    /// Gamma color vibrancy (between 0 and 1).
    /// 
    /// When this value is zero, gamma correction is applied independently to each color channel,
    /// which can lead to washed out colors. When it is one, gamma correction only affects luminance.
    /// Values between 0 and 1 interpolate geometrically between these extremes.
    #[arg(short, long, default_value_t = 0.0)]
    vibrancy: f64,
}

impl Cli {
    fn to_config(&self) -> RenderConfig {
        RenderConfig {
            width: self.dims[0],
            height: self.dims[1],
            iters: self.iters,
            threads: self.threads,
            grayscale: self.grayscale,
            gamma: self.gamma,
            preserve_color: self.preserve_color,
            vibrancy: self.vibrancy
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let input_file = File::open(&cli.input)?;
    let cfg = cli.to_config();

    let flame: Flame = FlameSource::from_file(input_file)?.to_flame();

    println!("Rendering flame...");

    let before_run = std::time::Instant::now();

    let buffer = flame.render(cfg);

    let dur = before_run.elapsed();

    buffer.into_rgb8().save(&cli.output)?;

    println!(
        "Completed! Rendered in {}.{:03} seconds. Output written to '{}'",
        dur.as_secs(),
        dur.subsec_millis(),
        cli.output.display()
    );

    Ok(())
}