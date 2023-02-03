use nalgebra::{Affine2, Point2, Transform, Matrix3 };
use rand::distributions::Uniform;
use rand::prelude::*;
use serde::Deserialize;
use std::f32::consts::PI;
use std::thread;

const PII: f32 = 1.0 / PI;

#[derive(Clone, Copy)]
pub struct Bounds {
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
}

impl Bounds {
    fn contains(&self, p: &Point2<f32>) -> bool {
        let x = p[0];
        let y = p[1];
        x > self.x_min && x < self.x_max && y > self.y_min && y < self.y_max
    }

    fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    fn height(&self) -> f32 {
        self.y_max - self.y_min
    }
}

#[derive(Clone)]
pub struct Flame {
    pub functions: Vec<(f32, Function)>,
    pub bounds: Bounds,
}

#[derive(Clone, Copy)]
pub struct RenderConfig {
    pub width: usize,
    pub height: usize,
    pub iters: usize,
    pub threads: usize,
}

impl Flame {
    pub fn run(&self, c: RenderConfig) -> Plotter {
        thread::scope(|s| {
            let mut handles = Vec::new();
        
            for _ in 0..c.threads {
                handles.push(s.spawn(|| self.run_single(c)));
            }
        
            Plotter::accumulate(handles.into_iter().map(|h| h.join().unwrap()))
        })
    }

    fn run_single(&self, c: RenderConfig) -> Plotter {
        let mut plotter = Plotter::new(c, self.bounds);

        let range = Uniform::new(0.0, 1.0);
        let mut rng = thread_rng();

        let mut point = Point2::new(range.sample(&mut rng), range.sample(&mut rng));

        for i in 0..(c.iters / c.threads) {
            point = self.rand_func(&mut rng).eval(point);
            if i >= 20 {
                plotter.plot(point);
            }
        }

        plotter
    }

    fn rand_func(&self, rng: &mut impl Rng) -> &Function {
        let r = Uniform::new(0.0, 1.0).sample(rng);
        let mut x = 0.0;
        for (p, t) in &self.functions {
            x += p;
            if r < x {
                return t;
            }
        }
    
        &self.functions.iter().last().unwrap().1
    }
}


#[allow(unused)]
fn sample_palette(p: Palette, i: f32) -> Color {
    if i >= 0.0 && i <= 1.0 {
        p[(i * 255.0) as usize]
    } else {
        panic!("Palette sample index must be between 0 and 1")
    }
}


#[derive(Copy, Clone)]
pub struct Function {
    pub var: Variation,
    pub trans: Affine2<f32>,
}

impl Function {
    pub fn eval(&self, arg: Point2<f32>) -> Point2<f32> {
        self.var.eval(self.trans * arg)
    }
}

#[derive(Clone, Copy, Deserialize)]
pub enum Variation {
    Id,
    Sinusoidal,
    Spherical,
    Swirl,
    Horseshoe,
    Polar,
    Handkerchief,
    Heart,
    Disc,
    Spiral,
    Hyperbolic,
    Diamond,
    Ex,
    Bent,
    Fisheye,
    Eyefish,
    Exponential,
    Cylinder,
    Tangent,
    Blob(f32, f32, f32),
    PDJ(f32, f32, f32, f32),
}

impl Variation {
    pub fn eval(self, arg: Point2<f32>) -> Point2<f32> {
        use self::Variation::*;

        let (x, y) = (arg[0], arg[1]);
        let r = x.powi(2) + y.powi(2);
        let theta = if y == 0.0 {
            if x == 0.0 {
                0.0
            } else if x > 0.0 {
                0.5 * PI
            } else {
                1.5 * PI
            }
        } else {
            (x / y).atan()
        };

        let (xo, yo) = match self {
            Id => (x, y),
            Sinusoidal => (x.sin(), y.sin()),
            Spherical => (x / r, y / r),
            Swirl => (x * r.sin() - y * r.cos(), x * r.cos() + y * r.sin()),
            Horseshoe => ((x - y) * (x + y) / r, 2.0 * x * y / r),
            Polar => (theta * PII, r - 1.0),
            Handkerchief => ((theta + r).sin(), (theta - r).cos()),
            Heart => (r * (theta * r).sin(), -r * (theta * r).cos()),
            Disc => (theta * PII * (PI * r).sin(), theta * PII),
            Spiral => ((theta.cos() + r.sin()) / r, (theta.sin() - r.cos()) / r),
            Hyperbolic => (theta.sin() / r, r * theta.cos()),
            Diamond => (theta.sin() * r.cos(), theta.cos() * r.sin()),
            Ex => {
                let p0 = (theta + r).sin().powi(3);
                let p1 = (theta - r).cos().powi(3);
                (r * (p0 + p1), r * (p0 - p1))
            }
            Bent => {
                let a = if x >= 0.0 { x } else { 2.0 * x };
                let b = if y >= 0.0 { y } else { 0.5 * y };
                (a, b)
            }
            Fisheye => (2.0 * y / (r + 1.0), 2.0 * x / (r + 1.0)),
            Eyefish => (2.0 * x / (r + 1.0), 2.0 * y / (r + 1.0)),
            Exponential => (
                (x - 1.0).exp() * (PI * y).cos(),
                (x - 1.0).exp() * (PI * y).sin(),
            ),
            Cylinder => (x.sin(), y),
            Tangent => (x.sin() / y.cos(), y.tan()),
            Blob(h, l, w) => {
                let a = r * (l + (h - l) / 2.0 * (1.0 + (theta * w).sin()));
                (a * theta.cos(), a * theta.sin())
            }
            PDJ(a, b, c, d) => ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos()),
        };

        Point2::new(xo, yo)
    }
}

pub struct Plotter {
    pub width: usize,
    pub height: usize,
    bounds: Bounds,
    trans: Affine2<f32>,
    counts: Vec<Vec<u32>>
}

impl Plotter {
    fn new(cfg: RenderConfig, bounds: Bounds) -> Self {
        let w_scale = (cfg.width - 1) as f32 / bounds.width();
        let h_scale =  (cfg.height - 1) as f32 / bounds.height();
        let trans = Transform::from_matrix_unchecked(Matrix3::new(
            w_scale, 0., -bounds.x_min * w_scale,
            0., -h_scale, bounds.y_max * h_scale,
            0., 0., 1.
        ));

        Plotter {
            trans, bounds,
            width: cfg.width, height: cfg.height,
            counts: vec![vec![0; cfg.width]; cfg.height]
        }
    }

    fn plot(&mut self, p: Point2<f32>) {
        if self.bounds.contains(&p) {
            let new_p = self.trans * p;
            self.counts[new_p[1] as usize][new_p[0] as usize] += 1;
        }
    }

    fn accumulate(plotters: impl IntoIterator<Item=Plotter>) -> Self {
        let mut plotters_ = plotters.into_iter();
        let mut plotter = plotters_.next().expect("cannot accumulate empty iterator of Plotters");

        for b in plotters_ {
            assert_eq!(plotter.width, b.width);
            assert_eq!(plotter.height, b.height);
            let bucket_pairs = plotter.counts.iter_mut()
                .zip(b.counts.iter())
                .map(|(r1, r2)| r1.iter_mut().zip(r2.iter()))
                .flatten();
            for (a, b) in bucket_pairs {
                *a += b;
            }
        }

        plotter
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let counts = self.counts.iter()
        .map(IntoIterator::into_iter).flatten()
            .map(|c| (c.clone() as f32).ln());
        let max = counts.clone().reduce(f32::max).unwrap();
        counts.map(|c| (c / max * 255.) as u8).collect()
    }
}

#[allow(unused)]
#[derive(Clone, Copy)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

#[allow(unused)]
impl Color {
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }
}

pub type Palette = [Color; 256];
