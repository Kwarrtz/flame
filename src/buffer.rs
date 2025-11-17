use std::ops::{AddAssign, MulAssign};


use nalgebra::Point2;
use num_traits::{zero, Float, NumAssign, NumCast, ToPrimitive, Zero};

#[derive(Debug, Clone, Copy)]
pub struct Bucket<T> {
    pub alpha: T,
    pub red: T,
    pub green: T,
    pub blue: T,
}

pub struct BucketIter<'a, T> {
    bucket: &'a Bucket<T>,
    i: u8,
}

impl<'a, T> Iterator for BucketIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let c: &'a T = match self.i {
            0 => &self.bucket.alpha,
            1 => &self.bucket.red,
            2 => &self.bucket.green,
            3 => &self.bucket.blue,
            _ => return None,
        };

        self.i += 1;

        Some(c)
    }
}

pub struct BucketIterMut<'a, T> {
    alpha: Option<&'a mut T>,
    red: Option<&'a mut T>,
    green: Option<&'a mut T>,
    blue: Option<&'a mut T>,
}

impl<'a, T> Iterator for BucketIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.alpha.take() {
            x @ Some(_) => x,
            None => match self.red.take() {
                x @ Some(_) => x,
                None => match self.green.take() {
                    x @ Some(_) => x,
                    None => self.blue.take(),
                },
            },
        }
    }
}

impl<T> Bucket<T> {
    pub fn iter_rgb(&self) -> BucketIter<T> {
        BucketIter { bucket: self, i: 1 }
    }

    pub fn iter_rgb_mut(&mut self) -> BucketIterMut<T> {
        // BucketIterMut { bucket: self, i: 1 }
        BucketIterMut {
            alpha: None,
            red: Some(&mut self.red),
            green: Some(&mut self.green),
            blue: Some(&mut self.blue),
        }
    }

    pub fn iter_argb(&self) -> BucketIter<T> {
        BucketIter { bucket: self, i: 0 }
    }

    pub fn iter_argb_mut(&mut self) -> BucketIterMut<T> {
        // BucketIterMut { bucket: self, i: 0 }
        BucketIterMut {
            alpha: Some(&mut self.alpha),
            red: Some(&mut self.red),
            green: Some(&mut self.green),
            blue: Some(&mut self.blue),
        }
    }

    pub fn from_argb(mut iter: impl Iterator<Item = T>) -> Option<Bucket<T>> {
        Some(Bucket {
            alpha: iter.next()?,
            red: iter.next()?,
            green: iter.next()?,
            blue: iter.next()?,
        })
    }

    pub fn map<S>(self, mut f: impl FnMut(T) -> S) -> Bucket<S> {
        Bucket {
            alpha: f(self.alpha),
            red: f(self.red),
            green: f(self.green),
            blue: f(self.blue),
        }
    }
}

impl<T: Zero> Bucket<T> {
    pub fn new() -> Self {
        Bucket {
            alpha: zero(),
            red: zero(),
            green: zero(),
            blue: zero(),
        }
    }
}

impl<T: MulAssign + Copy> MulAssign<T> for Bucket<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.iter_argb_mut().for_each(|c| *c *= rhs);
    }
}

impl<T: AddAssign + Copy> AddAssign for Bucket<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.iter_argb_mut()
            .zip(rhs.iter_argb())
            .for_each(|(c, cr)| *c += *cr);
    }
}

impl<T: Float> Bucket<T> {
    pub fn max(self, other: Self) -> Self {
        Bucket::from_argb(
            self.iter_argb()
                .zip(other.iter_argb())
                .map(|(a, b)| T::max(*a, *b)),
        )
        .unwrap()
    }
}

#[derive(Debug, Clone)]
pub struct Buffer<T> {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) buckets: Vec<Bucket<T>>,
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
