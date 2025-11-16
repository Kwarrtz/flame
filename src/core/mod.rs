use image::DynamicImage;
use nalgebra::{Affine2, Point2, Transform, Matrix3 };
use rand::distributions::Uniform;
use rand::prelude::*;
use std::thread;

mod variation;
pub use variation::*;

mod buffer;
use buffer::*;

mod color;
pub use color::*;

#[derive(Clone, Copy)]
pub struct RenderConfig {
    pub width: usize,
    pub height: usize,
    pub iters: usize,
    pub threads: usize,
    pub grayscale: bool,
    pub gamma: f64,
    pub preserve_color: bool,
    pub vibrancy: f64,
    pub samples: usize,
}

#[derive(Clone)]
pub struct Flame {
    pub functions: Vec<FunctionEntry>,
    pub last: Function,
    pub palette: Palette,
    pub bounds: Bounds,
}

impl Flame {
    pub fn run(&self, cfg: RenderConfig) -> Buffer<u32> {
        thread::scope(|s| {
            let mut handles = Vec::new();

            for _ in 0 .. cfg.threads {
                handles.push(s.spawn(|| self.run_single(cfg)));
            }

            Buffer::combine(handles.into_iter().map(|h| h.join().unwrap()))
        })
    }

    fn run_single(&self, cfg: RenderConfig) -> Buffer<u32> {
        let super_width = cfg.width * (1 + 2 * cfg.samples) + 2 * cfg.samples;
        let super_height = cfg.height * (1 + 2 * cfg.samples) + 2 * cfg.samples;

        let mut buffer: Buffer<u32> = Buffer::new(super_width, super_height);
        let trans = self.screen_transform(super_width, super_height);

        let mut rng = thread_rng();

        let mut point = Point2::new(rng.gen(), rng.gen());
        let mut c: u8 = rng.gen();

        for i in 0 .. (cfg.iters / cfg.threads) {
            let entry = self.rand_entry(&mut rng);

            point = entry.function.eval(point);
            point = self.last.eval(point);
            c = ((c as u16 + entry.color as u16) / 2) as u8;

            if i > 20 && self.bounds.contains(&point) {
                let screen_point = trans * point;
                let bucket = buffer.at_mut(screen_point);
                let color = self.palette.sample(c);
                bucket.alpha += 1;
                bucket.red += color.red as u32;
                bucket.green += color.green as u32;
                bucket.blue += color.blue as u32;
            }
        }

        buffer
    }

    pub fn render(&self, cfg: RenderConfig) -> DynamicImage {
        let mut buffer: Buffer<f64> = self.run(cfg).convert();
        buffer.log_density();
        buffer.normalize(cfg.preserve_color);
        buffer.gamma(cfg.gamma, cfg.vibrancy);
        buffer = buffer.filter(cfg.samples);
        buffer.normalize_clamp();
        let image_buf = buffer.scale_convert();

        if cfg.grayscale {
            DynamicImage::ImageLuma8(image_buf.into_gray8())
        } else {
            DynamicImage::ImageRgb8(image_buf.into_rgb8())
        }
    }

    fn rand_entry(&self, rng: &mut impl Rng) -> &FunctionEntry {
        let r = Uniform::new(0.0, 1.0).sample(rng);
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

#[derive(Clone)]
pub struct FunctionEntry {
    pub function: Function,
    pub weight: f32,
    pub color: u8
}

#[derive(Clone)]
pub struct Function {
    pub var: Variation,
    pub trans: Affine2<f32>,
}

impl Function {
    pub fn eval(&self, arg: Point2<f32>) -> Point2<f32> {
        self.var.eval(self.trans * arg)
    }
}

#[derive(Clone, Copy)]
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
