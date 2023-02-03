use nalgebra::{Affine2, Point2, Transform, Matrix3 };
use rand::distributions::Uniform;
use rand::prelude::*;
use std::thread;

mod variation;
pub use variation::*;

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
    pub functions: Vec<Function>,
    pub palette: Palette,
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
        
            Plotter::combine(handles.into_iter().map(|h| h.join().unwrap()))
        })
    }

    fn run_single(&self, cfg: RenderConfig) -> Plotter {
        let mut plotter = Plotter::new(cfg, self.bounds);

        let range = Uniform::new(0.0, 1.0);
        let mut rng = thread_rng();

        let mut point = Point2::new(range.sample(&mut rng), range.sample(&mut rng));
        let mut c = range.sample(&mut rng);

        for i in 0..(cfg.iters / cfg.threads) {
            let f = self.rand_func(&mut rng);
            point = f.eval(point);
            c = (c + f.color) / 2.;
            if i >= 20 {
                plotter.plot(point, self.palette.sample(c));
            }
        }

        plotter
    }

    fn rand_func(&self, rng: &mut impl Rng) -> &Function {
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
}

#[derive(Copy, Clone)]
pub struct Function {
    pub weight: f32,
    pub color: f32,
    pub var: Variation,
    pub trans: Affine2<f32>,
}

impl Function {
    pub fn eval(&self, arg: Point2<f32>) -> Point2<f32> {
        self.var.eval(self.trans * arg)
    }
}

#[derive(Clone, Copy)]
struct Bucket {
    count: u32,
    red: u32,
    green: u32,
    blue: u32
}

impl Bucket {
    fn new() -> Bucket {
        Bucket {
            count: 0,
            red: 0,
            green: 0,
            blue: 0
        }
    }

    fn drop(&mut self, color: Color) {
        self.count += 1;
        self.red += color.red as u32;
        self.green += color.green as u32;
        self.blue += color.blue as u32;
    }

    fn max(a: Bucket, b: Bucket) -> Bucket {
        Bucket {
            count: u32::max(a.count, b.count),
            red: u32::max(a.red, b.red),
            green: u32::max(a.green, b.green),
            blue: u32::max(a.blue, b.blue),
        }
    }
}

impl std::ops::AddAssign for Bucket {
    fn add_assign(&mut self, rhs: Bucket) {
        self.count += rhs.count;
        self.red += rhs.red;
        self.green += rhs.green;
        self.blue += rhs.blue;
    }
}

#[derive(Clone)]
pub struct Plotter {
    pub width: usize,
    pub height: usize,
    bounds: Bounds,
    trans: Affine2<f32>,
    buckets: Vec<Vec<Bucket>>
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
            buckets: vec![vec![Bucket::new(); cfg.width]; cfg.height]
        }
    }

    fn plot(&mut self, p: Point2<f32>, color: Color) {
        if self.bounds.contains(&p) {
            let new_p = self.trans * p;
            self.buckets[new_p[1] as usize][new_p[0] as usize].drop(color);
        }
    }

    fn combine(plotters: impl IntoIterator<Item=Plotter>) -> Self {
        let mut plotters_ = plotters.into_iter();
        let mut plotter = plotters_.next().expect("cannot accumulate empty iterator of Plotters");

        for b in plotters_ {
            assert_eq!(plotter.width, b.width);
            assert_eq!(plotter.height, b.height);
            let bucket_pairs = plotter.buckets.iter_mut()
                .zip(b.buckets.iter())
                .map(|(r1, r2)| r1.iter_mut().zip(r2.iter()))
                .flatten();
            for (a, &b) in bucket_pairs {
                *a += b;
            }
        }

        plotter
    }

    pub fn to_buffer_bw(&self) -> Vec<u8> {
        let counts = self.buckets.iter()
            .map(IntoIterator::into_iter).flatten()
            .map(|b| (b.count.clone() as f32).ln());
        let max = counts.clone().reduce(f32::max).unwrap();
        counts.map(|c| (c / max * 255.) as u8).collect()
    }

    pub fn to_buffer(&self) -> Vec<u8> {
        let buckets = self.buckets.iter()
            .map(IntoIterator::into_iter).flatten()
            .map(|b| {
                let count = b.count as f32;
                let scale = count.ln() / count;
                [ b.red as f32 * scale, b.green as f32 * scale, b.blue as f32 * scale]
            });
        let [max_r, max_b, max_g] = buckets.clone()
            .reduce(|[ra,ga,ba], [rb,gb,bb]| {
                [f32::max(ra,rb), f32::max(ga,gb), f32::max(ba,bb)]
            }).unwrap();
        buckets.map(|[r,g,b]| [norm_u8(r, max_r), norm_u8(g, max_g), norm_u8(b, max_b)].into_iter()).flatten().collect()
    }
}

fn norm_u8(x: f32, max: f32) -> u8 {
    (x / max * 255.) as u8
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub(crate) red: u8,
    pub(crate) green: u8,
    pub(crate) blue: u8,
}

impl Color {
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }
}

#[derive(Clone)]
pub struct Palette {
    colors: [Color; 256]
}

impl Palette {
    pub fn new(colors: [Color; 256]) -> Palette {
        Palette { colors }
    }

    fn sample(&self, i: f32) -> Color {
        if i >= 0.0 && i < 1.0 {
            self.colors[(i * 256.) as usize]
        } else {
            panic!("Palette sample index must be between 0 and 1")
        }
    }
}
