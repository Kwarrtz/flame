use nalgebra::{Point2, Rotation2};
use rand::prelude::*;
use rand_distr::weighted::WeightedIndex;
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

/// Cached values used during chaos game execution
struct FlameCache {
    rotations: Vec<Rotation2<f32>>,
    entry_distr: WeightedIndex<f32>,
    palette: Box<[Color; 256]>,
    rng: SmallRng,
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
        self.run_partial(&mut buffer, iters);
        buffer
    }

    /// Run the chaos game on a pre-allocated buffer. Useful for incremental rendering, e.g. as part of a GUI.
    pub fn run_partial(&self, buffer: &mut Buffer<u32>, iters: usize) {
        // a `Flame` with no functions should return a blank buffer / black screen
        if self.functions.is_empty() {
            return;
        }

        let trans = self.bounds.screen_transform(buffer.width, buffer.height);

        let mut cache = self.generate_cache();

        // random initial point and color value
        let mut point = Point2::<f32>::new(cache.rng.random(), cache.rng.random());
        let mut c: f32 = cache.rng.random();

        let rot_order = self.symmetry.abs().max(1);
        // include 0 branch for dihedral symmetry
        let start = if self.symmetry < 0 { -1 } else { 0 };

        for i in 0..iters {
            // one step of the chaos game
            
            // three different kinds of transformations possible
            match cache.rng.random_range(start..rot_order) {
            // match cache.frng.i8(start..rot_order) {
                // function
                0 => {
                    let entry = self.rand_entry(&mut cache);
                    point = entry.function.eval(&mut cache.rng, point);
                    c *= 1.0 - entry.color_speed;
                    c += entry.color * entry.color_speed;
                }

                // reflection
                -1 => {
                    point[0] = -point[0];
                }

                // rotation
                num => {
                    point = cache.rotations[(num - 1) as usize] * point;
                }
            }

            // calculate point in screen coordinates
            let screen_point = trans * self.last.eval(&mut cache.rng, point);

            if i > 20 && let Some(bucket) = buffer.at_mut(screen_point) {
                // skip plotting the point if its the first 20 iterations or the point
                // is out of bounds

                // get color corresponding to color value `c`
                let color = cache.palette[(c * 255.0) as usize];

                // update buckets
                bucket.alpha += 1;
                bucket.red += color.red as u32;
                bucket.green += color.green as u32;
                bucket.blue += color.blue as u32;
            }
        }
    }

    /// Get a random `FunctionEntry`
    fn rand_entry(&self, cache: &mut FlameCache) -> &FunctionEntry {
        &self.functions[cache.rng.sample(&cache.entry_distr)]
    }

    fn generate_cache(&self) -> FlameCache {
        let rot_order = self.symmetry.abs().max(1);
        let rotations = (1..rot_order) 
            .map(|n| Rotation2::new(TAU * n as f32 / rot_order as f32))
            .collect();
        let entry_distr = WeightedIndex::new(
            self.functions.iter().map(|e| e.weight)
        ).unwrap();
        FlameCache {
            rotations,
            entry_distr,
            palette: self.palette.generate_cache(),
            rng: rand::make_rng(),
        }
    }

    /// Convert from string in JSON format
    pub fn from_json(src: &str) -> serde_json::Result<Flame> {
        serde_json::from_str(src)
    }

    /// Convert from slice of bytes in MsgPack format
    pub fn from_mp(src: &[u8]) -> Result<Flame, rmp_serde::decode::Error> {
        rmp_serde::from_slice(src)
    }

    /// Convert from string in YAML format
    pub fn from_yaml(src: &str) -> Result<Flame, serde_yaml::Error> {
        serde_yaml::from_str(src)
    }

    /// Read from a specification file. Auto-detect format using file extension
    pub fn from_file(path: impl AsRef<Path>) -> Result<Flame, Error> {
        // let contents = std::fs::read_to_string(path.as_ref()).map_err(Error::FileReadError)?;
        let file = std::fs::File::open(path.as_ref()).map_err(Error::FileReadError)?;
        Ok(
            match path
                .as_ref()
                .extension()
                .ok_or(Error::ExtensionError)?
                .to_str()
            {
                Some("json") => serde_json::from_reader(&file)?,
                Some("yaml") => serde_yaml::from_reader(&file)?,
                Some("flam3") => rmp_serde::from_read(&file)?,
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


    /// Convert to MsgPack format
    pub fn to_mp(&self) -> Result<Vec<u8>, rmp_serde::encode::Error> {
        rmp_serde::to_vec(self)
    }

    /// Save to file, auto-detecting desired format from file extension
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut file = std::fs::File::create(path.as_ref()).map_err(Error::FileWriteError)?;
        match path
            .as_ref()
            .extension()
            .ok_or(Error::ExtensionError)?
            .to_str()
        {
            Some("json") => serde_json::to_writer(file, self)?,
            Some("yaml") => serde_yaml::to_writer(file, self)?,
            Some("flam3") => rmp_serde::encode::write(&mut file, self)?,
            _ => return Err(Error::ExtensionError),
        };

        Ok(())
    }
}

mod _serde {
    use super::*;

    #[derive(Serialize, Deserialize)]
    #[serde(tag="version")]
    pub enum FlameSource {
        #[serde(rename="2")]
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
