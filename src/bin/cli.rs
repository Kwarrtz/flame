use clap::{Args, Parser, Subcommand};
use clap_num::si_number;
use rand::{RngExt, rngs::SmallRng};
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
    /// Enable adaptive Gaussian blur (density estimation denoising).
    #[arg(long, short = 'l')]
    blur: bool,
    /// Blur radius coefficient in world units per log density.
    #[arg(long, short = 's', default_value_t = 0.03)]
    blur_strength: f64,
    /// Minimum blur sigma in world units.
    #[arg(long, default_value_t = 0.0)]
    blur_sigma_min: f64,
    /// Maximum blur sigma in world units.
    #[arg(long, default_value_t = 0.05)]
    blur_sigma_max: f64,
    /// Gaussian sigma (pixels) for local density patch averaging.
    #[arg(long, default_value_t = 2.0)]
    blur_patch_sigma: f64,
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
            blur: self.blur.then(|| BlurConfig {
                strength: self.blur_strength,
                sigma_min: self.blur_sigma_min,
                sigma_max: self.blur_sigma_max,
                patch_sigma: self.blur_patch_sigma,
            }),
        }
    }
}

fn render_and_save(
    flame: Flame,
    out: impl AsRef<Path>,
    run_cfg: RunConfig,
    render_cfg: RenderConfig,
) -> Result<(), Error> {
    let bounds = flame.bounds;
    let buffer = flame.run(run_cfg);
    let img_buffer = buffer.render(render_cfg, run_cfg.iters, bounds);

    img_buffer.to_dynamic8(render_cfg.grayscale).save(out)?;

    Ok(())
}

fn run_render(args: RenderArgs) -> Result<(), Error> {
    let run_cfg = args.run_config();
    let render_cfg = args.render_config();

    println!("Rendering flames...");

    let progress_bar = indicatif::ProgressBar::new(args.input.len() as u64);
    progress_bar.tick();

    let before_run = std::time::Instant::now();

    if args.input.len() == 1
        && let Some(out_path) = args.output
    {
        // output filename is specified
        let flame = Flame::from_file(&args.input[0])?;
        render_and_save(flame, out_path, run_cfg, render_cfg)?;
    } else {
        // create the output directory, if it's specified and doesn't exist
        if let Some(ref out_dir) = args.output
            && !out_dir.exists()
        {
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

            progress_bar.inc(1);
        }
    }

    let dur = before_run.elapsed();

    progress_bar.finish();

    println!(
        "Completed in {}.{:02} seconds",
        dur.as_secs(),
        dur.subsec_millis()
    );

    Ok(())
}

fn run_random(args: RandomArgs) -> Result<(), Error> {
    let mut rng: SmallRng = rand::make_rng();

    if !args.output.exists() {
        std::fs::create_dir(&args.output).map_err(Error::DirectoryWriteError)?;
    }

    println!("Generating flames...");

    let progress_bar = indicatif::ProgressBar::new(args.num as u64);
    progress_bar.tick();

    let before_run = std::time::Instant::now();

    let mut index = 1;
    for _ in 1..=args.num {
        // find the next available index in the directory
        let mut out_path: PathBuf;
        loop {
            out_path = args
                .output
                .join(PathBuf::from(index.to_string()))
                .with_extension(&args.filetype);

            if !std::fs::exists(&out_path).map_err(Error::FileWriteError)? {
                break;
            }

            index += 1;
        }

        let mut distr = random::FlameDistribution::default();
        distr.uniformity = args.uniformity;
        distr.skewness = args.skewness;
        distr.func_num_vals = (args.num_functions[0]..=args.num_functions[1]).collect();

        let flame: Flame = rng.sample(distr);

        flame.save(out_path)?;

        progress_bar.inc(1);
    }

    let dur = before_run.elapsed();

    progress_bar.finish();

    println!(
        "Completed in {}.{:03} seconds, output written to '{}'",
        dur.as_secs(),
        dur.subsec_millis(),
        args.output.display()
    );

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let res = match cli.command {
        Commands::Render(args) => run_render(args),
        Commands::Random(args) => run_random(args),
    };
    if let Err(e) = res {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
