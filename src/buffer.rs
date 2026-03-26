use nalgebra::Point2;
use num_traits::{NumAssign, NumCast, ToPrimitive};

use super::bucket::*;

/// A two-dimensional array of `Bucket`s
#[derive(Debug, Clone)]
pub struct Buffer<T> {
    pub width: usize,
    pub height: usize,
    pub buckets: Vec<Bucket<T>>,
}

impl<T: Copy> Buffer<T> {
    pub fn get(&self, x: usize, y: usize) -> Option<Bucket<T>> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(self.buckets[x + y * self.width])
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Bucket<T>> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(&mut self.buckets[x + y * self.width])
    }

    /// Get the bucket at a `Point`, i.e., rounding its components
    /// down to the nearest integer
    pub fn at_mut(&mut self, p: Point2<f32>) -> Option<&mut Bucket<T>> {
        // avoid saturating to 0 during cast
        if p[0] < 0.0 || p[1] < 0.0 { return None; }
        self.get_mut(p[0] as usize, p[1] as usize)
    }

    /// Generate a `Buffer` of a certain width and height, populated according to
    /// `buffer.get(x,y) == f(x, y)`
    pub fn from_func(
        width: usize,
        height: usize,
        mut f: impl FnMut(usize, usize) -> Bucket<T>,
    ) -> Self {
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

    /// Accumulate an iterator of `Buffer`s of the same dimensions,
    /// summing their `Bucket`s
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
