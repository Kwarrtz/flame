use std::ops::{MulAssign, AddAssign};

use nalgebra::Point2;
use num_traits::{NumAssign, Float, Zero, Bounded, Num, NumCast, ToPrimitive};
use image::{RgbImage, GrayImage, ImageBuffer};

#[derive(Debug, Clone)]
pub struct Bucket<T> {
    pub alpha: T,
    pub red: T,
    pub green: T,
    pub blue: T
}

impl<T: Zero> Bucket<T> {
    fn new() -> Self {
        Bucket {
            alpha: T::zero(),
            red: T::zero(),
            green: T::zero(),
            blue: T::zero()
        }
    }
}

impl<T: MulAssign + Copy> MulAssign<T> for Bucket<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.alpha *= rhs;
        self.red *= rhs;
        self.green *= rhs;
        self.blue *= rhs;
    }
}

impl<T: AddAssign> AddAssign for Bucket<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.alpha += rhs.alpha;
        self.red += rhs.red;
        self.green += rhs.green;
        self.blue += rhs.blue;
    }
}

impl<T: Float> Bucket<T> {
    fn max(self, other: Self) -> Self {
        Bucket {
            alpha: self.alpha.max(other.alpha),
            red: self.red.max(other.red),
            green: self.green.max(other.green),
            blue: self.blue.max(other.blue),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Buffer<T> {
    width: usize,
    height: usize,
    buckets: Vec<Bucket<T>>
}

impl<T: ToPrimitive> Buffer<T> {
    pub fn convert<S: NumCast>(self) -> Buffer<S> {
        Buffer {
            width: self.width, height: self.height,
            buckets: self.buckets.into_iter().map(|b| {
                    Bucket {
                        alpha: S::from(b.alpha).unwrap(),
                        red: S::from(b.red).unwrap(),
                        green: S::from(b.green).unwrap(),
                        blue: S::from(b.blue).unwrap()
                    }
                }).collect()
        }
    }
}

impl<T: NumAssign + Copy> Buffer<T> {
    pub fn new(width: usize, height: usize) -> Self {
        Buffer {
            width, height,
            buckets: vec![Bucket::new(); width * height]
        }
    }

    pub fn at_mut(&mut self, p: Point2<f32>) -> &mut Bucket<T> {
        &mut self.buckets[p[0] as usize + p[1] as usize * self.width]
    }

    pub fn combine(buffers: impl IntoIterator<Item=Self>) -> Self {
        let mut buffers_iter = buffers.into_iter();
        let mut combined = buffers_iter.next()
            .expect("tried to accumulate empty iterator of Buffers");

        for buffer in buffers_iter {
            assert_eq!(combined.width, buffer.width);
            assert_eq!(combined.height, buffer.height);
            let pairs = combined.buckets.iter_mut().zip(buffer.buckets.into_iter());
            for (comb_bucket, new_bucket) in pairs {
                *comb_bucket += new_bucket;
            }
        }

        combined
    }
}

impl<T: Float + NumAssign + Copy + std::fmt::Debug> Buffer<T> {
    pub fn log_density(&mut self) {
        for bucket in self.buckets.iter_mut() {
            if bucket.alpha.is_normal() {
                let s = bucket.alpha.ln() / bucket.alpha;
                *bucket *= s;
            }
        }
    }

    pub fn gamma(&mut self, gamma: T, vibrancy: T) {
        for bucket in self.buckets.iter_mut() {
            let g = gamma.recip() - T::one();
            let iv = T::one() - vibrancy;
            let alpha_s = bucket.alpha.powf(g * vibrancy);
            bucket.alpha = bucket.alpha.powf(gamma.recip());
            bucket.red *= bucket.red.powf(g * iv) * alpha_s;
            bucket.green *= bucket.green.powf(g * iv) * alpha_s;
            bucket.blue *=bucket.blue.powf(g * iv) * alpha_s;
        }
    } 

    pub fn normalize(&mut self, preserve_color: bool) {
        let max = self.buckets.iter().cloned().reduce(Bucket::max).unwrap();
        if preserve_color {
            let max_rgb = T::max(max.red, T::max(max.green, max.blue));
            for bucket in self.buckets.iter_mut() {
                bucket.alpha /= max.alpha;
                bucket.red /= max_rgb;
                bucket.green /= max_rgb;
                bucket.blue /= max_rgb;
            }
        } else {
            for bucket in self.buckets.iter_mut() {
                bucket.alpha /= max.alpha;
                bucket.red /= max.red;
                bucket.green /= max.green;
                bucket.blue /= max.blue;
            }
        }
    }

    pub fn scale_convert<S: Bounded + Num + NumCast>(&self) -> Buffer<S> {
        let new = self.buckets.iter().map(|b| {
            Bucket {
                alpha: scale(b.alpha),
                red: scale(b.red),
                blue: scale(b.blue),
                green: scale(b.green),
            }
        }).collect();

        Buffer {
            width: self.width, height: self.height,
            buckets: new
        }
    }
}

fn scale<T: Float, S: Bounded + Num + NumCast>(val: T) -> S {
    S::from(T::from(S::max_value()).unwrap() * T::max(T::zero(), val)).unwrap()
}

impl Buffer<u8> {
    pub fn into_gray8(&self) -> GrayImage {
        let raw = self.buckets.iter().map(|b| b.alpha).collect();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, raw).unwrap()
    }

    pub fn into_rgb8(&self) -> RgbImage {
        let raw = self.buckets.iter()
            .map(|b| [b.red, b.green, b.blue].into_iter())
            .flatten().collect();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, raw).unwrap()
    }
}