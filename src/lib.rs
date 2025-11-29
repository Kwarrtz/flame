use nalgebra::{Affine2, Point2, Transform, Matrix3 };
use rand::distr::Uniform;
use rand::prelude::*;
use std::thread;

mod variation;
pub use variation::*;

mod buffer;
pub use buffer::*;

mod color;
pub use color::*;

mod error;
pub use error::*;

mod file;
pub use file::*;

mod render;
pub use render::*;

#[derive(Debug, Clone, Copy)]
pub struct RunConfig {
    pub width: usize,
    pub height: usize,
    pub iters: usize,
    pub threads: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Flame {
    pub functions: Vec<FunctionEntry>,
    pub last: Function,
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

        let mut point = Point2::new(rng.random(), rng.random());
        let mut c: f32 = rng.random();

        for i in 0 .. iters {
            let entry = self.rand_entry(rng);

            point = entry.function.eval(point);
            point = self.last.eval(point);
            c *= 1.0 - entry.color_speed;
            c += entry.color * entry.color_speed;

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
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FunctionEntry {
    pub function: Function,
    pub weight: f32,
    pub color: f32,
    pub color_speed: f32,
}

impl FunctionEntry {
    fn new(
        function: Function,
        weight: f32, color: f32, color_speed: f32
    ) -> Result<FunctionEntry, FunctionEntryError> {
        if color > 1.0 || color < 0.0 {
            return Err(FunctionEntryError::Color)
        }

        if color_speed > 1.0 || color_speed < 0.0 {
            return Err(FunctionEntryError::ColorSpeed)
        }

        Ok(FunctionEntry {
            weight: weight,
            color: color,
            color_speed: color_speed,
            function: function
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub var: Variation,
    pub trans: Affine2<f32>,
}

impl Function {
    pub fn eval(&self, arg: Point2<f32>) -> Point2<f32> {
        self.var.eval(self.trans * arg)
    }
}

impl Default for Function {
    fn default() -> Function {
        Function {
            var: Variation::Id,
            trans: Affine2::identity()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
}

impl Bounds {
    pub fn new(x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> Self {
        Bounds { x_min, x_max, y_min, y_max }
    }

    pub fn contains(&self, p: &Point2<f32>) -> bool {
        let x = p[0];
        let y = p[1];
        x > self.x_min && x < self.x_max && y > self.y_min && y < self.y_max
    }

    pub fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> f32 {
        self.y_max - self.y_min
    }
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds {
            x_min: -1., x_max: 1.,
            y_min: -1., y_max: 1.
        }
    }
}
