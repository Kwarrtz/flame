use clap::{Parser, Subcommand};
use clap_num::si_number;
use std::path::{Path, PathBuf};
use rand::Rng;

use flame::*;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Number of iterations of the chaos game to run (accepts SI postfixes).
    ///
    /// Higher values reduce noise but take longer to run.
    #[arg(short, long, default_value = "100M", value_parser = si_number::<usize>)]
    iters: usize,
    /// Number of parallel threads.
    #[arg(short, long, default_value_t = 10)]
    threads: usize,
    /// Dimensions (in pixels) of the output image.
    #[arg(short, long, number_of_values = 2, default_values_t = [1000, 1000])]
    #[arg(value_names = ["WIDTH", "HEIGHT"])]
    dims: Vec<usize>,
    /// Image brightness.
    #[arg(short, long, default_value_t = 20.0)]
    brightness: f64,
    /// Output a grayscale image, ignoring any specified color information.
    #[arg(short='G', long)]
    grayscale: bool,
    /// Gamma correction factor. (Deprecated, recommended to use the `brightness` flag instead.)
    #[arg(short, long, default_value_t = 1.0)]
    gamma: f64,
    /// Gamma color vibrancy (between 0 and 1).
    ///
    /// When this value is zero, gamma correction is applied independently to each color channel,
    /// which can lead to washed out colors. When it is one, gamma correction only affects luminance.
    /// Values between 0 and 1 interpolate geometrically between these extremes.
    #[arg(short, long, default_value_t = 0.5)]
    vibrancy: f64,
}

#[derive(Subcommand)]
enum Commands {
    /// Render from a pre-existing flame descriptor file.
    Render {
        /// Path to flame descriptor (file extension must be JSON or YAML).
        input: PathBuf,
        /// Path to output image (file extension must be JPEG or PNG).
        output: PathBuf,
    },
    /// Randomly generate flames.
    RandGen {
        /// Number of flames to be generated.
        num: usize,
        /// Path to output directory.
        output: PathBuf,
    },
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
            brightness: self.brightness,
            grayscale: self.grayscale,
        }
    }
}

fn render_and_save(
    flame: Flame,
    out: impl AsRef<Path>,
    run_cfg: RunConfig,
    render_cfg: RenderConfig
) -> Result<(), FlameError>
{
    let buffer = flame.run(run_cfg);
    let img_buffer = buffer.render(render_cfg, run_cfg.iters);

    img_buffer.to_dynamic8(render_cfg.grayscale).save(out)?;

    Ok(())
}

fn run() -> Result<(), FlameError> {
    let cli = Cli::parse();
    let run_cfg = cli.run_config();
    let render_cfg = cli.render_config();

    match cli.command {
        Commands::Render { input, output } => {
            let flame: Flame = Flame::from_file(input)?;

            println!("Rendering flame...");

            let before_run = std::time::Instant::now();

            render_and_save(flame, &output, run_cfg, render_cfg)?;

            let dur = before_run.elapsed();

            println!(
                "Completed! Rendered in {}.{:03} seconds. Output written to '{}'",
                dur.as_secs(),
                dur.subsec_millis(),
                output.display()
            );
        }

        Commands::RandGen { num, output } => {
            let mut rng = rand::rng();

            std::fs::create_dir(&output)
                .map_err(FlameError::DirectoryWriteError)?;

            println!("Generating flames...");

            let before_run = std::time::Instant::now();

            for i in 1..=num {
                let file_output = output.join(PathBuf::from(i.to_string()));

                let distr = random::FlameDistribution {
                    func_distr: random::FunctionDistribution {
                        weight_distr: rand::distr::Uniform::new(-1., 1.).unwrap(),
                        var_distr: random::VariationDistribution(
                            rand::distr::StandardUniform
                        ),
                    },
                    palette_distr: random::PaletteDistribution(3..=7),
                    symmetry_range: 1..=1,
                    func_num_range: 3..=6,
                };

                let flame = rng.sample(distr);

                flame.save(file_output.with_extension("json"))?;
                render_and_save(flame, file_output.with_extension("png"), run_cfg, render_cfg)?;
            }

            let dur = before_run.elapsed();

            println!(
                "Completed! Rendered in {}.{:03} seconds. Output written to '{}'",
                dur.as_secs(),
                dur.subsec_millis(),
                output.display()
            );
        }
    };

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
