extern crate rand;
extern crate image;
extern crate nalgebra as na;

use std::sync::{Arc,Mutex};
use std::thread;
use rand::thread_rng;
use rand::distributions::{Range, IndependentSample};
use image::{ColorType};
use na::{Affine2, Point2, Transform, Matrix3};

const WORKERS: usize = 8;

#[derive(Clone)]
struct Config {
    transformations: Vec<(f32,Affine2<f32>)>,
    output_dimensions: (usize, usize),
    output_path: &'static str,
    bounds: ((f32,f32),(f32,f32)),
    iterations: u32,
}

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

fn run(conf: &Config) -> Vec<u8> {
    let counter = Arc::new(Mutex::new(
        vec![vec![0;conf.output_dimensions.0];conf.output_dimensions.1]));
    let mut handles = Vec::new();

    for _ in 0..WORKERS {
        let conf = conf.clone();
        let counter = Arc::clone(&counter);

        handles.push(thread::spawn(move || {
            let (width,height) = conf.output_dimensions;
            let ((x_min,x_max),(y_min,y_max)) = conf.bounds;

            let range = Range::new(0.0,1.0);
            let mut rng = thread_rng();

            let mut point = Point2::new( range.ind_sample(&mut rng)
                                       , range.ind_sample(&mut rng));

            for _ in 0..conf.iterations {
                point = choose_trans(&conf.transformations) * point;
                let mut x = point[0];
                let mut y = point[1];
                if x > x_min && x < x_max && y > y_min && y < y_max {
                    x = (x - x_min) * (width - 1) as f32 / (x_max - x_min);
                    y = (y - y_min) * (height - 1) as f32 / (y_max - y_min);
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

    let conf = Config {
        transformations: trans,
        output_dimensions: (500,700),
        output_path: "data/test.png",
        bounds: ((-2.5,3.0),(-1.0,11.0)),
        iterations: 100000,
    };

    let buf = run(&conf);

    let _ = image::save_buffer(
        conf.output_path,
        &buf[..],
        conf.output_dimensions.0 as u32,
        conf.output_dimensions.1 as u32,
        ColorType::Gray(8)
    );
}
