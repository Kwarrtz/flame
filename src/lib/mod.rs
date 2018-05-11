use std::thread;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use nalgebra::{Transform,Matrix3,Affine2, Point2};
use std::fs::File;
use serde_json;

pub mod cli;

#[cfg(test)]
mod test;

#[derive(Deserialize)]
struct FlameSource {
    bounds: [f32;4],
    transformations: Vec<[f32;7]>,
}

impl FlameSource {
    fn from_file(f : File) -> serde_json::Result<FlameSource> {
        serde_json::from_reader(f)
    }

    fn to_flame(self) -> Flame {
        let trans = self.transformations.iter().map(|t| {
            let a = Transform::from_matrix_unchecked(Matrix3::new(
                t[1], t[2], t[5],
                t[3], t[4], t[6],
                0.,   0.,    1.));
            (t[0],a)
        }).collect();

        Flame {
            x_min: self.bounds[0], x_max: self.bounds[1],
            y_min: self.bounds[2], y_max: self.bounds[3],
            transformations: trans,
        }
    }
}

#[derive(Clone)]
pub struct Flame {
    pub transformations: Vec<(f32,Affine2<f32>)>,
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

type Counter = Vec<Vec<u32>>;

fn choose_trans(trans: &Vec<(f32,Affine2<f32>)>) -> Affine2<f32> {
    let r = Range::new(0.0,1.0).ind_sample(&mut thread_rng());
    let mut x = 0.0;
    for &(p,t) in trans {
        x += p;
        if r < x {
            return t;
        }
    }

    trans.iter().last().unwrap().1
}

pub fn run_single(f: Flame, c: Config) -> Counter {
    let mut counter = vec![vec![0;c.width];c.height];

    let range = Range::new(0.0,1.0);
    let mut rng = thread_rng();

    let mut point = Point2::new( range.ind_sample(&mut rng)
                               , range.ind_sample(&mut rng));

    for _ in 0..(c.iterations / c.workers) {
        point = choose_trans(&f.transformations) * point;
        let mut x = point[0];
        let mut y = point[1];
        if x > f.x_min && x < f.x_max && y > f.y_min && y < f.y_max {
            x = (x - f.x_min) * (c.width - 1) as f32 / (f.x_max - f.x_min);
            y = (y - f.y_min) * (c.height - 1) as f32 / (f.y_max - f.y_min);
            counter[y as usize][x as usize] += 1;
        }
    }

    counter
}

pub fn to_buffer(buckets: Counter) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut max = 0.0;
    for row in buckets.iter().rev() {
        for &bucket in row {
            let val = (bucket as f32).ln();
            buf.push(val);
            max = f32::max(max,val);
        }
    }

    buf.iter().map(|x| (x / max * 255.0) as u8).collect()
}


pub fn run(flame: Flame, c: Config) -> Vec<u8> {
    let mut handles = Vec::new();
    let mut buckets = vec![vec![0;c.width];c.height];

    for _ in 0..c.workers {
        let f = flame.clone();
        handles.push(thread::spawn(move || run_single(f,c)));
    }

    for handle in handles {
        let counter = handle.join().unwrap();
        for (i,row) in counter.iter().enumerate() {
            for (j,count) in row.iter().enumerate() {
                buckets[i][j] += count;
            }
        }
    }

    to_buffer(buckets)
}
