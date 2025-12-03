use image::{DynamicImage, GrayImage, ImageBuffer, RgbImage};
use num_traits::{clamp, one, zero, Bounded, Float, Num, NumAssign, NumCast, ToPrimitive};

use super::buffer::*;
use super::bucket::*;

#[derive(Clone, Copy)]
pub struct RenderConfig {
    pub gamma: f64,
    pub vibrancy: f64,
    pub brightness: f64,
    pub width: usize,
    pub height: usize,
}

impl<T: ToPrimitive + Clone> Buffer<T> {
    pub fn render<S: Bounded + Num + NumCast>(&self, cfg: RenderConfig, iters: usize) -> Buffer<S> {
        let mut buffer = self.clone().convert::<f64>();
        buffer.log_density(cfg.brightness, iters as f64);
        buffer.normalize();
        buffer.gamma(cfg.gamma, cfg.vibrancy);
        buffer.clamp();
        buffer.scale_convert()
    }
}

impl<T: Float + NumAssign + Copy> Buffer<T> {
    pub fn log_density(&mut self, brightness: T, iters: T) {
        for bucket in self.buckets.iter_mut() {
            if bucket.alpha.is_normal() {
                let new_alpha = bucket.alpha.ln() - iters.ln() + brightness;
                let new_alpha = T::max(T::zero(), new_alpha);
                let s = new_alpha / bucket.alpha;
                *bucket *= s;
            }
        }
    }

    pub fn gamma(&mut self, gamma: T, vibrancy: T) {
        let p = gamma.recip() - T::one();
        let p_alpha = p * vibrancy;
        let p_channel = p * (T::one() - vibrancy);
        for bucket in self.buckets.iter_mut() {
            let alpha_scale = bucket.alpha.powf(p_alpha);
            for c in bucket.iter_argb_mut() {
                *c *= c.powf(p_channel) * alpha_scale;
            }
        }
    }

    pub fn normalize(&mut self) {
        let max = self.buckets.iter().cloned().reduce(Bucket::max).unwrap();
        let max_rgb = max.iter_rgb().cloned().reduce(T::max).unwrap();
        for bucket in self.buckets.iter_mut() {
            bucket.alpha /= max.alpha;
            for c in bucket.iter_rgb_mut() {
                *c /= max_rgb;
            }
        }
    }

    pub fn clamp(&mut self) {
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
    pub fn to_gray8(&self) -> GrayImage {
        let raw = self.buckets.iter().map(|b| b.alpha).collect();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, raw)
            .expect("incorrect image buffer size")
    }

    pub fn to_rgb8(&self) -> RgbImage {
        let raw = self
            .buckets
            .iter()
            .map(|b| b.iter_rgb().cloned())
            .flatten()
            .collect();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, raw)
            .expect("incorrect image buffer size")
    }

    pub fn to_dynamic8(&self, grayscale: bool) -> DynamicImage {
        if grayscale {
            DynamicImage::ImageLuma8(self.to_gray8())
        } else {
            DynamicImage::ImageRgb8(self.to_rgb8())
        }
    }
}
