use nalgebra::{Point2, Rotation2};
use rand::distr::Uniform;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::{f32::consts::TAU, path::Path, thread};

use super::{bounds::*, buffer::*, color::*, error::*, function::*};

/// Parameters for chaos game execution
#[derive(Debug, Clone, Copy)]
pub struct RunConfig {
    pub width: usize,
    pub height: usize,
    pub iters: usize,
    pub threads: usize,
}

/// Flame specification
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "self::_serde::FlameSource",
    into = "self::_serde::FlameSource"
)]
pub struct Flame {
    pub functions: Vec<FunctionEntry>,
    /// Final transform, executed unconditionally before plotting
    #[serde(default)]
    pub last: Function,
    /// Symmetry factor. Values of `0` and `1` both correspond to
    /// no added symmetry. Positive values impose n-fold rotational symmetry
    /// while negative values impose dihedral symmetry (rotation plus reflection).
    #[serde(default)]
    pub symmetry: i8,
    pub palette: Palette,
    // Region of the plane to include in the final image
    pub bounds: Bounds,
}

impl Flame {
    /// Run the chaos game. Returns an unnormalized `Buffer`. The `alpha` channel of each bucket
    /// stores the total number of hits while color channels store aggregated color weight (up to 255 per hit).  
    pub fn run(&self, cfg: RunConfig) -> Buffer<u32> {
        if cfg.threads == 1 {
            return self.run_single_thread(cfg.width, cfg.height, cfg.iters);
        }

        thread::scope(|s| {
            let mut handles = Vec::new();

            // spawn worker threads
            for _ in 0..cfg.threads {
                handles.push(s.spawn(|| {
                    self.run_single_thread(cfg.width, cfg.height, cfg.iters / cfg.threads)
                }));
            }

            // combine output buffers from each thread
            Buffer::combine(handles.into_iter().map(|h| h.join().unwrap()))
        })
    }

    /// Run the chaos game in a single thread.
    fn run_single_thread(&self, width: usize, height: usize, iters: usize) -> Buffer<u32> {
        let mut buffer: Buffer<u32> = Buffer::new(width, height);
        let mut rng = rand::rng();
        self.run_partial(&mut buffer, iters, &mut rng);
        buffer
    }

    /// Run the chaos game on a pre-allocated buffer. Useful for incremental rendering, e.g. as part of a GUI.
    pub fn run_partial(&self, buffer: &mut Buffer<u32>, iters: usize, rng: &mut impl Rng) {
        // a `Flame` with no functions should return a blank buffer / black screen
        if self.functions.is_empty() {
            return;
        }

        let trans = self.bounds.screen_transform(buffer.width, buffer.height);

        // random initial point and color value
        let mut point = Point2::<f32>::new(rng.random(), rng.random());
        let mut c: f32 = rng.random();

        // symmetry is implemented by adding the corresponding transformation as a function
        let num_cases: u8 = if self.symmetry == 0 || self.symmetry == 1 {
            1 // no added symmetry
        } else if self.symmetry > 1 {
            2 // rotational symmetry
        } else {
            3 // dihedral symmetry
        };

        for i in 0..iters {
            // one step of the chaos game

            // choose which kind of function to execute
            match rng.random_range(0..num_cases) {
                // actual flame function
                0 => {
                    // choose random function
                    let entry = self.rand_entry(rng);

                    // update point
                    point = entry.function.eval(rng, point);

                    // update color
                    c *= 1.0 - entry.color_speed;
                    c += entry.color * entry.color_speed;
                }

                // rotation
                1 => {
                    let rot_degree = self.symmetry.abs();
                    let times = rng.random_range(0..rot_degree);
                    let rot = Rotation2::new(TAU * times as f32 / rot_degree as f32);
                    point = rot * point;
                }

                // reflection (across y axis)
                2 => {
                    point[0] = -point[0];
                }

                _ => unreachable!(),
            }

            // calculate point in screen coordinates
            let screen_point = trans * self.last.eval(rng, point);

            if i > 20 && let Some(bucket) = buffer.at_mut(screen_point) {
                // skip plotting the point if its the first 20 iterations or the point
                // is out of bounds

                // get color corresponding to color value `c`
                let color = self.palette.sample(c).expect("color index out of bounds");

                // update buckets
                bucket.alpha += 1;
                bucket.red += color.red as u32;
                bucket.green += color.green as u32;
                bucket.blue += color.blue as u32;
            }
        }
    }

    /// Get a random `FunctionEntry`
    fn rand_entry(&self, rng: &mut impl Rng) -> &FunctionEntry {
        let total: f32 = self.functions.iter().map(|f| f.weight).sum();
        let r = Uniform::new(0.0, total).unwrap().sample(rng);
        let mut x = 0.0;
        for f in &self.functions {
            x += f.weight;
            if r < x {
                return f;
            }
        }

        &self.functions.iter().last().unwrap()
    }

    /// Convert from string in JSON format
    pub fn from_json(src: &str) -> serde_json::Result<Flame> {
        serde_json::from_str(src)
    }

    /// Convert from string in RON format
    pub fn from_ron(src: &str) -> ron::error::SpannedResult<Flame> {
        ron::from_str(src)
    }

    /// Convert from string in YAML format
    pub fn from_yaml(src: &str) -> Result<Flame, serde_yaml::Error> {
        serde_yaml::from_str(src)
    }

    /// Read from a specification file. Auto-detect format using file extension
    pub fn from_file(path: impl AsRef<Path>) -> Result<Flame, Error> {
        let contents = std::fs::read_to_string(path.as_ref()).map_err(Error::FileReadError)?;
        Ok(
            match path
                .as_ref()
                .extension()
                .ok_or(Error::ExtensionError)?
                .to_str()
            {
                Some("json") => Flame::from_json(&contents)?,
                Some("ron") => Flame::from_ron(&contents)?,
                Some("yaml") => Flame::from_yaml(&contents)?,
                _ => return Err(Error::ExtensionError),
            },
        )
    }

    /// Convert to JSON format
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Convert to YAML format
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Save to file, auto-detecting desired format from file extension
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let serialized = match path
            .as_ref()
            .extension()
            .ok_or(Error::ExtensionError)?
            .to_str()
        {
            Some("json") => self.to_json()?,
            Some("yaml") => self.to_yaml()?,
            _ => return Err(Error::ExtensionError),
        };
        std::fs::write(path, serialized).map_err(Error::FileWriteError)?;

        Ok(())
    }
}

mod _serde {
    use super::*;

    #[derive(Serialize, Deserialize)]
    #[serde(tag="version")]
    pub enum FlameSource {
        #[serde(rename="1")]
        Valid {
            functions: Vec<FunctionEntry>,
            #[serde(default)]
            last: Function,
            #[serde(default)]
            symmetry: i8,
            palette: Palette,
            bounds: Bounds,
        },
        #[serde(other)]
        Invalid
    }

    impl From<Flame> for FlameSource {
        fn from(value: Flame) -> Self {
            FlameSource::Valid {
                functions: value.functions,
                last: value.last,
                symmetry: value.symmetry,
                palette: value.palette,
                bounds: value.bounds,
            }
        }
    }

    impl TryFrom<FlameSource> for Flame {
         type Error = FlameError;

         fn try_from(value: FlameSource) -> Result<Self, Self::Error> {
             match value {
                 FlameSource::Valid { functions, last, symmetry, palette, bounds }
                     => Ok(Flame { functions, last, symmetry, palette, bounds }),
                 FlameSource::Invalid => Err(FlameError)
             }
         }
    }
}
