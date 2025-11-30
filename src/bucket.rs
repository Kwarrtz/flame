use std::ops::{AddAssign, MulAssign};

use num_traits::{zero, Float, Zero};

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
    pub fn iter_rgb<'a>(&'a self) -> BucketIter<'a, T> {
        BucketIter { bucket: self, i: 1 }
    }

    pub fn iter_rgb_mut<'a>(&'a mut self) -> BucketIterMut<'a, T> {
        // BucketIterMut { bucket: self, i: 1 }
        BucketIterMut {
            alpha: None,
            red: Some(&mut self.red),
            green: Some(&mut self.green),
            blue: Some(&mut self.blue),
        }
    }

    pub fn iter_argb<'a>(&'a self) -> BucketIter<'a, T> {
        BucketIter { bucket: self, i: 0 }
    }

    pub fn iter_argb_mut<'a>(&'a mut self) -> BucketIterMut<'a, T> {
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

impl<T: Float + Copy> Bucket<T> {
    pub fn max(self, other: Self) -> Self {
        Bucket::from_argb(
            self.iter_argb()
                .zip(other.iter_argb())
                .map(|(a, b)| T::max(*a, *b)),
        )
        .unwrap()
    }
}
