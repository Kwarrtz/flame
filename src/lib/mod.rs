use std::thread;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use nalgebra::{Affine2, Point2};

#[cfg(test)]
mod test;

#[derive(Clone)]
pub struct Flame {
    pub transformations: Vec<(f32,Affine2<f32>)>,
}

#[derive(Clone,Copy)]
pub struct Config {
    pub width: usize,
    pub height: usize,
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
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

pub fn run_single(flame: Flame, c: Config) -> Counter {
    let mut counter = vec![vec![0;c.width];c.height];

    let range = Range::new(0.0,1.0);
    let mut rng = thread_rng();

    let mut point = Point2::new( range.ind_sample(&mut rng)
                               , range.ind_sample(&mut rng));

    for _ in 0..(c.iterations / c.workers) {
        point = choose_trans(&flame.transformations) * point;
        let mut x = point[0];
        let mut y = point[1];
        if x > c.x_min && x < c.x_max && y > c.y_min && y < c.y_max {
            x = (x - c.x_min) * (c.width - 1) as f32 / (c.x_max - c.x_min);
            y = (y - c.y_min) * (c.height - 1) as f32 / (c.y_max - c.y_min);
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
