use std::thread;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use nalgebra::{Transform,Matrix3,Point2,Affine2};
use std::fs::File;
use serde_json;
use std::f32::consts::PI;

const PII: f32 = 1.0 / PI;

#[cfg(test)]
mod test;

#[derive(Clone)]
pub struct Flame {
    pub functions: Vec<(f32,Function)>,
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
}

impl Flame {
    pub fn from_file(f: File) -> serde_json::Result<Flame> {
        Ok(FlameSource::from_file(f)?.to_flame())
    }
}

#[derive(Clone,Copy)]
pub struct Config {
    pub width: usize,
    pub height: usize,
    pub iterations: usize,
    pub workers: usize,
}

pub fn run_single(f: Flame, c: Config) -> Counter {
    let mut counter = vec![vec![Bucket::new();c.width];c.height];

    let range = Range::new(0.0,1.0);
    let mut rng = thread_rng();

    let mut point = Point2::new( range.ind_sample(&mut rng)
                               , range.ind_sample(&mut rng));

    for _ in 0..(c.iterations / c.workers) {
        point = choose_func(&f.functions).eval(point);
        let mut x = point[0];
        let mut y = point[1];
        if x > f.x_min && x < f.x_max && y > f.y_min && y < f.y_max {
            x = (x - f.x_min) * (c.width - 1) as f32 / (f.x_max - f.x_min);
            y = (y - f.y_min) * (c.height - 1) as f32 / (f.y_max - f.y_min);
            counter[y as usize][x as usize].plot();
        }
    }

    counter
}

pub fn to_buffer(buckets: Counter) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut max = 0.0;
    for row in buckets.iter().rev() {
        for &bucket in row {
            let val = (bucket.alpha as f32).ln();
            max = f32::max(max,val);
            buf.push(val);
        }
    }

    buf.iter().map(|x| (x / max * 255.0) as u8).collect()
}


pub fn run(flame: Flame, c: Config) -> Vec<u8> {
    let mut handles = Vec::new();
    let mut buckets = vec![vec![Bucket::new();c.width];c.height];

    for _ in 0..c.workers {
        let f = flame.clone();
        handles.push(thread::spawn(move || run_single(f,c)));
    }

    for handle in handles {
        let counter = handle.join().unwrap();
        for (i,row) in counter.iter().enumerate() {
            for (j,bucket) in row.iter().enumerate() {
                buckets[i][j] += *bucket;
            }
        }
    }

    to_buffer(buckets)
}

fn choose_func(funcs: &Vec<(f32,Function)>) -> Function {
    let r = Range::new(0.0,1.0).ind_sample(&mut thread_rng());
    let mut x = 0.0;
    for &(p,t) in funcs {
        x += p;
        if r < x {
            return t;
        }
    }

    funcs.iter().last().unwrap().1
}

#[allow(unused)]
fn sample_palette(p: Palette, i: f32) -> Color {
    if i >= 0.0 && i <= 1.0 {
        p[(i * 255.0) as usize]
    } else {
        panic!("Palette sample index must be between 0 and 1")
    }
}

#[derive(Deserialize)]
struct FlameSource {
    bounds: [f32;4],
    functions: Vec<FunctionSource>,
}

impl FlameSource {
    fn from_file(f : File) -> serde_json::Result<FlameSource> {
        serde_json::from_reader(f)
    }

    fn to_flame(self) -> Flame {
        let funcs = self.functions.iter().map(FunctionSource::to_function).collect();

        Flame {
            x_min: self.bounds[0], x_max: self.bounds[1],
            y_min: self.bounds[2], y_max: self.bounds[3],
            functions: funcs,
        }
    }
}

#[derive(Deserialize)]
struct FunctionSource(f32,Variation,[f32;6]);

impl FunctionSource {
    fn to_function(&self) -> (f32,Function) {
        let t = Transform::from_matrix_unchecked(Matrix3::new(
            self.2[0], self.2[1], self.2[4],
            self.2[2], self.2[3], self.2[5],
            0.0,       0.0,       1.0
        ));

        let f = Function {
            var: self.1,
            trans: t,
        };

        (self.0, f)
    }
}

#[derive(Copy,Clone)]
pub struct Function {
    pub var: Variation,
    pub trans: Affine2<f32>,
}

impl Function {
    pub fn eval(&self, arg: Point2<f32>) -> Point2<f32> {
        self.var.eval(self.trans * arg)
    }
}

#[derive(Clone,Copy,Deserialize)]
pub enum Variation {
    Id, Sinusoidal, Spherical, Swirl, Horseshoe, Polar,
    Handkerchief, Heart, Disc, Spiral, Hyperbolic,
    Diamond, Ex, Bent, Fisheye, Eyefish, Exponential,
    Cylinder, Tangent, Blob(f32,f32,f32), PDJ(f32,f32,f32,f32),
}

impl Variation {
    pub fn eval(self, arg: Point2<f32>) -> Point2<f32> {
        use self::Variation::*;

        let (x,y) = (arg[0], arg[1]);
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
            (x/y).atan()
        };

        let (xo,yo) = match self {
            Id => (x,y),
            Sinusoidal => (x.sin(),y.sin()),
            Spherical => (x/r, y/r),
            Swirl => (x * r.sin() - y * r.cos(), x * r.cos() + y * r.sin()),
            Horseshoe => ((x-y)*(x+y)/r, 2.0*x*y/r),
            Polar => (theta * PII, r - 1.0),
            Handkerchief => ((theta + r).sin(), (theta - r).cos()),
            Heart => (r * (theta * r).sin(), -r * (theta * r).cos()),
            Disc => (theta * PII * (PI * r).sin(), theta * PII),
            Spiral => ((theta.cos() + r.sin())/r, (theta.sin() - r.cos())/r),
            Hyperbolic => (theta.sin() / r, r * theta.cos()),
            Diamond => (theta.sin() * r.cos(), theta.cos() * r.sin()),
            Ex => {
                let p0 = (theta + r).sin().powi(3);
                let p1 = (theta - r).cos().powi(3);
                (r * (p0 + p1), r * (p0 - p1))
            },
            Bent => {
                let a = if x >= 0.0 { x } else { 2.0 * x };
                let b = if y >= 0.0 { y } else { 0.5 * y };
                (a,b)
            },
            Fisheye => (2.0 * y / (r + 1.0), 2.0 * x / (r + 1.0)),
            Eyefish => (2.0 * x / (r + 1.0), 2.0 * y / (r + 1.0)),
            Exponential => ((x - 1.0).exp() * (PI * y).cos(), (x - 1.0).exp() * (PI * y).sin()),
            Cylinder => (x.sin(), y),
            Tangent => (x.sin() / y.cos(), y.tan()),
            Blob(h,l,w) => {
                let a = r * (l + (h - l)/2.0 * (1.0 + (theta * w).sin()));
                (a * theta.cos(), a * theta.sin())
            },
            PDJ(a,b,c,d) => ((a*y).sin() - (b*x).cos(), (c*x).sin() - (d*y).cos())
        };

        Point2::new(xo,yo)
    }
}

#[derive(Clone,Copy)]
pub struct Bucket {
    pub alpha: u64
}

impl Bucket {
    pub fn new() -> Self {
        Bucket {
            alpha: 0
        }
    }

    pub fn plot(&mut self) {
        self.alpha += 1;
    }
}

impl ::std::ops::Add<Bucket> for Bucket {
    type Output = Bucket;

    fn add(self, rhs: Bucket) -> Bucket {
        Bucket { alpha: self.alpha + rhs.alpha }
    }
}

impl ::std::ops::AddAssign<Bucket> for Bucket {
    fn add_assign(&mut self, rhs: Bucket) {
        self.alpha += rhs.alpha;
    }
}

pub type Counter = Vec<Vec<Bucket>>;

#[allow(unused)]
#[derive(Clone,Copy)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8
}

#[allow(unused)]
impl Color {
    pub fn from_rgb(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }
}

pub type Palette = [Color;256];
