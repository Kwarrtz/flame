extern crate rand;
extern crate image;
extern crate nalgebra as na;

use rand::ThreadRng;
use rand::distributions::{Weighted, WeightedChoice, Range, IndependentSample};
use image::{ColorType};
use na::{Affine2, Point2, Transform, Matrix3};

const WIDTH: usize = 5000;
const HEIGHT: usize = 7000;
const X_MIN: f32 = -2.5;
const X_MAX: f32 = 3.0;
const Y_MIN: f32 = -1.0;
const Y_MAX: f32 = 11.0;

const ITERS: u32 = 500000000;

const PATH: &str = "data/test.png";

struct Flame<'a> {
    point: Point2<f32>,
    trans: WeightedChoice<'a,Affine2<f32>>,
    rng: ThreadRng,
    buckets: Vec<Vec<u32>>,
}

impl<'a> Flame<'a> {
    fn new(trans: WeightedChoice<'a,Affine2<f32>>) -> Self {
        let mut rng = rand::thread_rng();
        let range = Range::new(0.0,1.0);
        Flame {
            point: Point2::new(range.ind_sample(&mut rng), range.ind_sample(&mut rng)),
            trans: trans,
            rng: rng,
            buckets: vec![vec![0;WIDTH];HEIGHT],
        }
    }

    fn plot(&mut self) {
        let mut x = self.point[0];
        let mut y = self.point[1];
        if x > X_MIN && x < X_MAX && y > Y_MIN && y < Y_MAX {
            x -= X_MIN;
            x *= (WIDTH - 1) as f32 / (X_MAX - X_MIN);
            y -= Y_MIN;
            y *= (HEIGHT - 1) as f32 / (Y_MAX - Y_MIN);
            self.buckets[y as usize][x as usize] += 1;
        }
    }

    fn step(&mut self) {
        self.point = self.trans.ind_sample(&mut self.rng) * self.point;
        self.plot();
    }

    fn to_buffer(self) -> Vec<u8> {
        let mut buf = Vec::new();
        let mut max = 0.0;
        for row in self.buckets.iter().rev() {
            for &bucket in row {
                let val = (bucket as f32).ln();
                buf.push(val);
                max = f32::max(max,val);
            }
        }
        buf.iter().map(|x| (x / max * 255.0) as u8).collect()
    }
}

fn main() {
    let mut funs =
        [ Weighted {
            weight: 1,
            item: Transform::from_matrix_unchecked(Matrix3::new(
                0.0, 0.0, 0.0,
                0.0, 0.16, 0.0,
                0.0, 0.0, 1.0)) }
        , Weighted {
            weight: 87,
            item: Transform::from_matrix_unchecked(Matrix3::new(
                0.85, 0.04, 0.0,
                -0.04, 0.85, 1.6,
                0.0, 0.0, 1.0)) }
        , Weighted {
            weight: 7,
            item: Transform::from_matrix_unchecked(Matrix3::new(
                0.2, -0.26, 0.0,
                0.23, 0.22, 1.6,
                0.0, 0.0, 1.0)) }
        , Weighted {
            weight: 7,
            item: Transform::from_matrix_unchecked(Matrix3::new(
                -0.15, 0.28, 0.0,
                0.26, 0.24, 0.44,
                0.0, 0.0, 1.0)) }
        ];
    let mut flame = Flame::new(WeightedChoice::new(&mut funs));
    for _ in 0..ITERS {
        flame.step();
    }
    let buf = &flame.to_buffer()[..];
    let _ = image::save_buffer(PATH, buf, WIDTH as u32, HEIGHT as u32, ColorType::Gray(8));
}
