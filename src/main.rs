extern crate rand;
extern crate image;
extern crate nalgebra as na;

use std::sync::{Arc,Mutex};
use std::thread;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use image::{ColorType};
use na::{Affine2, Point2, Transform, Matrix3};

const WIDTH: usize = 500;
const HEIGHT: usize = 700;

const X_MIN: f32 = -2.5;
const X_MAX: f32 = 3.0;
const Y_MIN: f32 = -1.0;
const Y_MAX: f32 = 11.0;

const ITERS: u32 = 100000;

const PATH: &str = "data/test.png";

const WORKERS: usize = 8;

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

fn run(trans: Vec<(f32,Affine2<f32>)>) -> Vec<u8> {
    let counter = Arc::new(Mutex::new(vec![vec![0;WIDTH];HEIGHT]));
    let mut handles = Vec::new();

    for _ in 0..WORKERS {
        let trans = trans.clone();
        let counter = Arc::clone(&counter);

        handles.push(thread::spawn(move || {
            let range = Range::new(0.0,1.0);
            let mut rng = thread_rng();

            let mut point = Point2::new(range.ind_sample(&mut rng), range.ind_sample(&mut rng));

            for _ in 0..ITERS {
                point = choose_trans(&trans) * point;
                let mut x = point[0];
                let mut y = point[1];
                if x > X_MIN && x < X_MAX && y > Y_MIN && y < Y_MAX {
                    x -= X_MIN;
                    x *= (WIDTH - 1) as f32 / (X_MAX - X_MIN);
                    y -= Y_MIN;
                    y *= (HEIGHT - 1) as f32 / (Y_MAX - Y_MIN);
                    let mut buckets = counter.lock().unwrap();
                    buckets[y as usize][x as usize] += 1;
                }
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let mut buf = Vec::new();
    let mut max = 0.0;
    for row in counter.lock().unwrap().iter().rev() {
        for &bucket in row {
            let val = (bucket as f32).ln();
            buf.push(val);
            max = f32::max(max,val);
        }
    }

    buf.iter().map(|x| (x / max * 255.0) as u8).collect()
}

fn main() {
    let trans =
        vec![ (0.01, Transform::from_matrix_unchecked(Matrix3::new(
                0.0, 0.0, 0.0,
                0.0, 0.16, 0.0,
                0.0, 0.0, 1.0)))
        , (0.87, Transform::from_matrix_unchecked(Matrix3::new(
                0.85, 0.04, 0.0,
                -0.04, 0.85, 1.6,
                0.0, 0.0, 1.0)))
        , (0.07, Transform::from_matrix_unchecked(Matrix3::new(
                0.2, -0.26, 0.0,
                0.23, 0.22, 1.6,
                0.0, 0.0, 1.0)))
        , (0.07, Transform::from_matrix_unchecked(Matrix3::new(
                -0.15, 0.28, 0.0,
                0.26, 0.24, 0.44,
                0.0, 0.0, 1.0)))
        ];

    let buf = run(trans);

    let _ = image::save_buffer(PATH, &buf[..], WIDTH as u32, HEIGHT as u32, ColorType::Gray(8));
}
