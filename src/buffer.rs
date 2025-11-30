use nalgebra::Point2;
use num_traits::{NumAssign, NumCast, ToPrimitive};

use super::bucket::*;

#[derive(Debug, Clone)]
pub struct Buffer<T> {
    pub width: usize,
    pub height: usize,
    pub buckets: Vec<Bucket<T>>,
}

impl<T: Copy> Buffer<T> {
    pub fn get(&self, x: usize, y: usize) -> Bucket<T> {
        self.buckets[x + y * self.width]
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut Bucket<T> {
        &mut self.buckets[x + y * self.width]
    }

    pub fn at_mut(&mut self, p: Point2<f32>) -> &mut Bucket<T> {
        self.get_mut(p[0] as usize, p[1] as usize)
    }

    pub fn from_func(width: usize, height: usize, mut f: impl FnMut(usize, usize) -> Bucket<T>) -> Self {
        let mut buckets = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                buckets.push(f(x, y));
            }
        }

        Buffer {
            width,
            height,
            buckets,
        }
    }
}

impl<T: ToPrimitive> Buffer<T> {
    pub fn convert<S: NumCast>(self) -> Buffer<S> {
        Buffer {
            width: self.width,
            height: self.height,
            buckets: self
                .buckets
                .into_iter()
                .map(|b| b.map(|c| S::from(c).unwrap()))
                .collect(),
        }
    }
}

impl<T: NumAssign + Copy> Buffer<T> {
    pub fn new(width: usize, height: usize) -> Self {
        Buffer {
            width,
            height,
            buckets: vec![Bucket::new(); width * height],
        }
    }

    pub fn combine(buffers: impl IntoIterator<Item = Self>) -> Self {
        let mut buffers_iter = buffers.into_iter();
        let mut combined = buffers_iter
            .next()
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
