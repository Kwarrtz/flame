use nalgebra::{Affine2, Matrix3, Point2, Rotation2, Transform};
use rand::distr::Uniform;
use rand::prelude::*;
use std::{f32::consts::TAU, path::Path, thread};
use serde::{Serialize, Deserialize};

use super::{
    color::*,
    function::*,
    buffer::*,
    error::*,
    bounds::*
};

#[derive(Debug, Clone, Copy)]
pub struct RunConfig {
    pub width: usize,
    pub height: usize,
    pub iters: usize,
    pub threads: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Flame {
    pub functions: Vec<FunctionEntry>,
    #[serde(default)]
    pub last: Function,
    #[serde(default)]
    pub symmetry: i8,
    pub palette: Palette,
    pub bounds: Bounds,
}

impl Flame {
    pub fn run(&self, cfg: RunConfig) -> Buffer<u32> {
        if cfg.threads == 1 {
            return self.run_single_thread(cfg.width, cfg.height, cfg.iters);
        }

        thread::scope(|s| {
            let mut handles = Vec::new();

            for _ in 0 .. cfg.threads {
                handles.push(s.spawn(||
                    self.run_single_thread(cfg.width, cfg.height, cfg.iters / cfg.threads)));
            }

            Buffer::combine(handles.into_iter().map(|h| h.join().unwrap()))
        })
    }

    fn run_single_thread(&self, width: usize, height: usize, iters: usize) -> Buffer<u32> {
        let mut buffer: Buffer<u32> = Buffer::new(width, height);
        let mut rng = rand::rng();
        self.run_partial(&mut buffer, iters, &mut rng);
        buffer
    }

    pub fn run_partial(&self, buffer: &mut Buffer<u32>, iters: usize, rng: &mut impl Rng) {
        if self.functions.is_empty() {
            return;
        }

        let trans = self.screen_transform(buffer.width, buffer.height);

        let mut point = Point2::<f32>::new(rng.random(), rng.random());
        let mut c: f32 = rng.random();

        let num_cases: u8 =
            if self.symmetry == 0
            || self.symmetry == 1 {
                1
            } else if self.symmetry > 1 {
                2
            } else {
                3
            };

        for i in 0 .. iters {
            match rng.random_range(0..num_cases) {
                0 => {
                    let entry = self.rand_entry(rng);
                    point = entry.function.eval(rng, point);
                    point = self.last.eval(rng, point);
                    c *= 1.0 - entry.color_speed;
                    c += entry.color * entry.color_speed;
                }
                1 => {
                    let rot_degree = self.symmetry.abs();
                    let times = rng.random_range(0..rot_degree);
                    let rot = Rotation2::new(TAU * times as f32 / rot_degree as f32);
                    point = rot * point;
                }
                2 => {
                    point[0] = -point[0];
                }
                _ => unreachable!()
            }

            if i > 20 && self.bounds.contains(&point) {
                let screen_point = trans * point;
                let bucket = buffer.at_mut(screen_point);
                let color = self.palette.sample(c).expect("color index out of bounds");
                bucket.alpha += 1;
                bucket.red += color.red as u32;
                bucket.green += color.green as u32;
                bucket.blue += color.blue as u32;
            }
        }
    }

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

    fn screen_transform(&self, width: usize, height: usize) -> Affine2<f32> {
        let w_scale = (width - 1) as f32 / self.bounds.width();
        let h_scale =  (height - 1) as f32 / self.bounds.height();
        Transform::from_matrix_unchecked(Matrix3::new(
            w_scale, 0., -self.bounds.x_min * w_scale,
            0., -h_scale, self.bounds.y_max * h_scale,
            0., 0., 1.
        ))
    }

    pub fn from_json(src: &str) -> serde_json::Result<Flame> {
        serde_json::from_str(src)
    }

    pub fn from_ron(src: &str) -> ron::error::SpannedResult<Flame> {
        ron::from_str(src)
    }

    pub fn from_yaml(src: &str) -> Result<Flame, serde_yaml::Error> {
        serde_yaml::from_str(src)
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Flame, FlameError> {
        let contents = std::fs::read_to_string(path.as_ref())?;
        Ok(match path.as_ref().extension().ok_or(FlameError::ExtensionError)?.to_str() {
            Some("json") => Flame::from_json(&contents)?,
            Some("ron") => Flame::from_ron(&contents)?,
            Some("yaml") => Flame::from_yaml(&contents)?,
            _ => return Err(FlameError::ExtensionError)
        })
    }
}
