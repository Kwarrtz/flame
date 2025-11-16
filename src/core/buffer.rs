use std::ops::{AddAssign, MulAssign};

use image::{GrayImage, ImageBuffer, RgbImage};
use nalgebra::Point2;
use num_traits::{clamp, one, zero, Bounded, Float, Num, NumAssign, NumCast, ToPrimitive, Zero};

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

// pub struct BucketIterMut<'a, T> {
//     bucket: &'a mut Bucket<T>,
//     i: u8
// }

// impl<'a, T> Iterator for BucketIterMut<'a, T> {
//     type Item = &'a mut T;

//     fn next(&mut self) -> Option<&'a mut T> {
//         let c = match self.i {
//             0 => &mut self.bucket.alpha,
//             1 => &mut self.bucket.red,
//             2 => &mut self.bucket.green,
//             3 => &mut self.bucket.blue,
//             _ => return None
//         };

//         self.i += 1;

//         let ptr = c as *mut T;
//         unsafe {
//             Some(&mut *ptr)
//         }
//     }
// }

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
    fn new() -> Self {
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
    fn max(self, other: Self) -> Self {
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
    width: usize,
    height: usize,
    buckets: Vec<Bucket<T>>,
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

impl<T: Float + NumAssign + Copy> Buffer<T> {
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
            let g = gamma.recip() - one();
            let giv = g * (T::one() - vibrancy);
            let alpha_s = bucket.alpha.powf(g * vibrancy);
            for c in bucket.iter_argb_mut() {
                *c *= c.powf(giv) * alpha_s;
            }
        }
    }

    pub fn filter(&self, samples: usize) -> Buffer<T> {
        let s = 1 + 2 * samples;
        let width = self.width / s;
        let height = self.height / s;

        let mut buffer = Buffer::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let b = buffer.get_mut(x, y);
                for yi in 0..s {
                    for xi in 0..s {
                        *b += self.get(s * x + xi, s * y + yi);
                    }
                }
                *b *= T::from(s.pow(2)).unwrap().recip();
            }
        }

        buffer
    }

    pub fn normalize(&mut self, preserve_color: bool) {
        let max = self.buckets.iter().cloned().reduce(Bucket::max).unwrap();
        if preserve_color {
            let max_rgb = T::max(max.red, T::max(max.green, max.blue));
            for bucket in self.buckets.iter_mut() {
                bucket.alpha /= max.alpha;
                for c in bucket.iter_rgb_mut() {
                    *c /= max_rgb;
                }
            }
        } else {
            for bucket in self.buckets.iter_mut() {
                for (c, cm) in bucket.iter_argb_mut().zip(max.iter_argb()) {
                    *c /= *cm;
                }
            }
        }
    }

    pub fn normalize_clamp(&mut self) {
        for bucket in self.buckets.iter_mut() {
            for c in bucket.iter_argb_mut() {
                *c = clamp(*c, zero(), one())
            }
        }
    }

    pub fn scale_convert<S: Bounded + Num + NumCast>(&self) -> Buffer<S> {
        Buffer {
            width: self.width,
            height: self.height,
            buckets: self.buckets.iter().cloned().map(|b| b.map(scale)).collect(),
        }
    }
}

fn scale<T: Float, S: Bounded + Num + NumCast>(val: T) -> S {
    S::from(T::from(S::max_value()).unwrap() * T::max(zero(), val)).unwrap()
}

impl Buffer<u8> {
    pub fn into_gray8(&self) -> GrayImage {
        let raw = self.buckets.iter().map(|b| b.alpha).collect();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, raw).unwrap()
    }

    pub fn into_rgb8(&self) -> RgbImage {
        let raw = self
            .buckets
            .iter()
            .map(|b| b.iter_rgb().cloned())
            .flatten()
            .collect();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, raw).unwrap()
    }
}
