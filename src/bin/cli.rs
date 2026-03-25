use clap::{Args, Parser, Subcommand};
use clap_num::si_number;
use rand::{
    Rng,
    distr::Uniform,
};
use std::path::{Path, PathBuf};

use flame::*;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Render one or more flames from descriptor files.
    Render(RenderArgs),
    /// Randomly generate flame descriptors.
    Random(RandomArgs),
}

#[derive(Args)]
struct RenderArgs {    
        /// Path to flame descriptor(s) (file extension must be .json, .yaml, or .flam3).
        input: Vec<PathBuf>,
        /// Path to output directory.
        ///
        /// If multiple flames are selected, this is treated as a directory name, which will
        /// be created if it does not already exist. If a single input was provided, it is
        /// instead treated as a full file path which must have the extension .png or .jpeg.
        /// In this case, the -f flag is ignored.
        ///
        /// If this option is not provided, output images have the same path stem as their
        /// source descriptor.
        #[arg(short, long)]
        output: Option<PathBuf>,         
        /// File type of output image. Allowed values are 'jpeg' and 'png'.
        #[arg(short = 'f', long, default_value = "jpeg")]
        filetype: String,
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
        #[arg(short = 'G', long)]
        grayscale: bool,
}

#[derive(Args)]
struct RandomArgs {
    /// Number of flames to be generated.
    num: usize,
    /// Path to output directory.
    output: PathBuf,
    /// Output image type ('json', 'yaml', or 'flam3')
    #[arg(short, long, default_value = "json")]
    filetype: String,
    /// Scaling uniformity for affine transformations.
    #[arg(short, long, default_value_t = 0.5)]
    uniformity: f32,
    /// Maximum skew for affine transformations.
    #[arg(short, long, default_value_t = 0.5)]
    skewness: f32,
    /// Minimum and maximum number of function entries.
    #[arg(short, long, default_values_t = [4, 7])]
    #[arg(value_names = ["MIN", "MAX"])]
    num_functions: Vec<usize>,
}

impl RenderArgs {
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
            brightness: self.brightness,
            grayscale: self.grayscale,
        }
    }
}

fn render_and_save(
    flame: Flame,
    out: impl AsRef<Path>,
    run_cfg: RunConfig,
    render_cfg: RenderConfig,
) -> Result<(), Error> {
    let buffer = flame.run(run_cfg);
    let img_buffer = buffer.render(render_cfg, run_cfg.iters);

    img_buffer.to_dynamic8(render_cfg.grayscale).save(out)?;

    Ok(())
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Render(args) => {
            let run_cfg = args.run_config();
            let render_cfg = args.render_config();
            
            println!("Rendering {} flame(s)...", args.input.len());

            let before_run = std::time::Instant::now();

            if args.input.len() == 1 && let Some(out_path) = args.output {
                // output filename is specified
                let flame = Flame::from_file(&args.input[0])?;
                render_and_save(flame, out_path, run_cfg, render_cfg)?;
            } else {
                // create the output directory, if it's specified and doesn't exist
                if let Some(ref out_dir) = args.output && !out_dir.exists() {
                    std::fs::create_dir(out_dir).map_err(Error::DirectoryWriteError)?;
                }

                for in_path in args.input {
                    let out_path = if let Some(ref out_dir) = args.output {
                        // output directory is specified
                        out_dir
                            .with_file_name(in_path.file_name().expect("input must be a path to a file"))
                            .with_extension(&args.filetype)
                    } else {
                        // no output destination specified
                        in_path.with_extension(&args.filetype)
                    };

                    let flame = Flame::from_file(in_path)?;
                    render_and_save(flame, out_path, run_cfg, render_cfg)?;
                }
            }

            let dur = before_run.elapsed();

            println!(
                "Completed! Rendered in {}.{:03} seconds.",
                dur.as_secs(),
                dur.subsec_millis()
            );
        }

        Commands::Random(args) => {
            let mut rng = rand::rng();

            if !args.output.exists() {
                std::fs::create_dir(&args.output).map_err(Error::DirectoryWriteError)?;
            }

            println!("Generating flames...");

            let before_run = std::time::Instant::now();

            let mut index = 1;
            for _ in 1..=args.num {
                // find the next available index in the directory
                let mut out_path: PathBuf;
                loop {
                    out_path = args.output
                        .join(PathBuf::from(index.to_string()))
                        .with_extension(&args.filetype);

                    if !std::fs::exists(&out_path).map_err(Error::FileWriteError)? {
                        break;
                    }

                    index += 1;
                }

                let mut distr = random::DefaultFlameDistribution::default();
                distr.func_distr.aff_distr.uniformity = args.uniformity;
                distr.func_distr.aff_distr.skewness = args.skewness;
                distr.func_num_distr = Uniform::try_from(
                    args.num_functions[0]..=args.num_functions[1],
                ).unwrap();

                let flame = rng.sample(distr);

                flame.save(out_path)?;
            }

            let dur = before_run.elapsed();

            println!(
                "Completed! Generated in {}.{:03} seconds. Output written to '{}'",
                dur.as_secs(),
                dur.subsec_millis(),
                args.output.display()
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
