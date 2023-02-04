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

impl<T: std::ops::AddAssign> std::ops::AddAssign for Bucket<T> {
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
        let mut buffers_ = buffers.into_iter();
        let mut buffer = buffers_.next().expect("tried to accumulate empty iterator of Buffers");

        for b in buffers_ {
            assert_eq!(buffer.width, b.width);
            assert_eq!(buffer.height, b.height);
            for (a, b) in buffer.buckets.iter_mut().zip(b.buckets.into_iter()) {
                *a += b;
            }
        }

        buffer
    }
}

impl<T: Float + NumAssign + Copy + std::fmt::Debug> Buffer<T> {
    pub fn scale_log(&mut self) {
        for bucket in self.buckets.iter_mut() {
            if bucket.alpha.is_normal() {
                let s = bucket.alpha.ln() / bucket.alpha;
                bucket.alpha *= s;
                bucket.red *= s;
                bucket.green *= s;
                bucket.blue *= s;
            }
        }
    }

    pub fn clip_convert<S: Bounded + Num + NumCast>(&self) -> Buffer<S> {
        let max = self.buckets.iter().cloned().reduce(Bucket::max).unwrap();
        let new = self.buckets.iter().map(|b| {
            Bucket {
                alpha: clip(b.alpha / max.alpha),
                red: clip(b.red / max.red),
                blue: clip(b.blue / max.blue),
                green: clip(b.green / max.green),
            }
        }).collect();

        Buffer {
            width: self.width, height: self.height,
            buckets: new
        }
    }
}

fn clip<T: Float, S: Bounded + Num + NumCast>(val: T) -> S {
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