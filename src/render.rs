use image::{DynamicImage, GrayImage, ImageBuffer, RgbImage};
use num_traits::{Bounded, Float, Num, NumAssign, NumCast, ToPrimitive, clamp, one, zero};
use rayon::prelude::*;

use super::bounds::Bounds;
use super::bucket::*;
use super::buffer::*;

/// Configuration for adaptive Gaussian blur (density estimation denoising)
#[derive(Clone, Copy, Debug)]
pub struct BlurConfig {
    /// inverse-density scale in world units; default 0.001
    pub strength: f64,
    /// minimum blur sigma in world units; default 0.1
    pub sigma_min: f64,
    /// maximum blur sigma in world units; default 0.5
    pub sigma_max: f64,
    /// sigma of the Gaussian used to average local density (pixels); default 2.0
    pub patch_sigma: f64,
}

impl Default for BlurConfig {
    fn default() -> Self {
        BlurConfig {
            strength: 0.001,
            sigma_min: 0.1,
            sigma_max: 0.5,
            patch_sigma: 2.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RenderConfig {
    pub brightness: f64,
    pub grayscale: bool,
    pub blur: Option<BlurConfig>,
}

impl<T: ToPrimitive + Clone> Buffer<T> {
    pub fn render<S: Bounded + Num + NumCast>(
        &self,
        cfg: RenderConfig,
        iters: usize,
        bounds: Bounds,
    ) -> Buffer<S> {
        let mut buffer = self.clone().convert::<f64>();
        let ln_alphas = buffer.log_density(cfg.brightness, iters as f64);
        if let Some(blur_cfg) = &cfg.blur {
            buffer.blur(&ln_alphas, blur_cfg, bounds);
        }
        buffer.normalize();
        buffer.scale_convert()
    }

    pub fn render_raw_rgba(
        &self,
        raw: &mut [u8],
        cfg: RenderConfig,
        iters: usize,
        bounds: Bounds,
    ) {
        let img_buffer = self.render(cfg, iters, bounds);
        img_buffer.write_to_raw_rgba8(raw, cfg.grayscale);
    }

    pub fn render_image(
        &self,
        cfg: RenderConfig,
        iters: usize,
        bounds: Bounds,
    ) -> DynamicImage {
        let img_buffer = self.render(cfg, iters, bounds);
        img_buffer.to_dynamic8(cfg.grayscale)
    }
}

impl<T: Float + NumAssign + Copy> Buffer<T> {
    /// Apply log-density scaling. Returns raw `ln(alpha)` per pixel before
    /// brightness/iters compensation — used to drive adaptive blur widths.
    pub fn log_density(&mut self, brightness: T, iters: T) -> Vec<T> {
        self.buckets
            .iter_mut()
            .map(|bucket| {
                let raw_ln = bucket.alpha.ln();
                if bucket.alpha.is_normal() {
                    let new_alpha = raw_ln - iters.ln() + brightness;
                    let new_alpha = T::max(T::zero(), new_alpha);
                    let s = new_alpha / bucket.alpha;
                    *bucket *= s;
                }
                raw_ln
            })
            .collect()
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
        // avoid corruption, particularly for blank images
        if !max_rgb.is_normal() { return; }
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

impl Buffer<f64> {
    /// Apply spatially-adaptive Gaussian blur to all channels.
    ///
    /// Kernel sigma for each pixel is inversely proportional to the average
    /// log hit count in a local patch, computed in world coordinates and
    /// converted to pixels via `bounds`. Sparse regions get wide blur; dense
    /// regions get narrow (or no) blur.
    ///
    /// Uses a two-phase separable approach: horizontal pass then vertical pass,
    /// each parallelized by row via rayon.
    pub fn blur(&mut self, ln_alphas: &[f64], cfg: &BlurConfig, bounds: Bounds) {
        let width = self.width;
        let height = self.height;
        let px_per_world = width as f64 / bounds.width() as f64;
        let ps = cfg.patch_sigma;
        let r = (3.0 * ps).ceil() as i64;
        let inv2ps2 = 1.0 / (2.0 * ps * ps);

        // phase 1: Gaussian-weighted average of ln_alpha in local patch,
        // then map to per-pixel blur sigma in pixels
        let mut sigmas = vec![0.0f64; width * height];
        sigmas
            .par_chunks_mut(width)
            .enumerate()
            .for_each(|(y, row)| {
                for (x, sigma_out) in row.iter_mut().enumerate() {
                    let mut wsum = 0.0f64;
                    let mut wasum = 0.0f64;
                    for dy in -r..=r {
                        for dx in -r..=r {
                            let sx = (x as i64 + dx).clamp(0, width as i64 - 1) as usize;
                            let sy = (y as i64 + dy).clamp(0, height as i64 - 1) as usize;
                            let la = ln_alphas[sx + sy * width];
                            if la.is_finite() && la > 0.0 {
                                let w = (-((dx * dx + dy * dy) as f64) * inv2ps2).exp();
                                wasum += w * la;
                                wsum += w;
                            }
                        }
                    }
                    let sigma_world = if wsum == 0.0 {
                        cfg.sigma_max
                    } else {
                        (cfg.strength / (wasum / wsum)).clamp(cfg.sigma_min, cfg.sigma_max)
                    };
                    *sigma_out = sigma_world * px_per_world;
                }
            });

        // phase 2: 2D Gaussian blur — read from original src, write to temp
        let mut temp = Buffer::<f64>::new(width, height);
        {
            let src = &self.buckets;
            temp.buckets
                .par_chunks_mut(width)
                .enumerate()
                .for_each(|(y, out_row)| {
                    for (x, out) in out_row.iter_mut().enumerate() {
                        let sigma = sigmas[x + y * width];
                        if sigma < 0.5 {
                            *out = src[x + y * width];
                            continue;
                        }
                        let hw = (3.0 * sigma).ceil() as i64;
                        let inv2s2 = 1.0 / (2.0 * sigma * sigma);
                        let mut acc = Bucket::<f64>::new();
                        let mut wsum = 0.0f64;
                        for dy in -hw..=hw {
                            let sy = (y as i64 + dy).clamp(0, height as i64 - 1) as usize;
                            for dx in -hw..=hw {
                                let sx = (x as i64 + dx).clamp(0, width as i64 - 1) as usize;
                                let w = (-((dx * dx + dy * dy) as f64) * inv2s2).exp();
                                let mut b = src[sx + sy * width];
                                b *= w;
                                acc += b;
                                wsum += w;
                            }
                        }
                        acc *= 1.0 / wsum;
                        *out = acc;
                    }
                });
        }
        self.buckets = temp.buckets;
    }
}

fn scale<T: Float, S: Bounded + Num + NumCast>(val: T) -> S {
    S::from(T::from(S::max_value()).unwrap() * T::max(zero(), val)).unwrap()
}

impl Buffer<u8> {
    pub fn write_to_raw_rgba8(self, raw: &mut [u8], grayscale: bool) {
        assert_eq!(
            raw.len(),
            4 * self.width * self.height,
            "attempting to write to RGBA buffer of size {} when Buffer has size {}x{}",
            raw.len(),
            self.width,
            self.height
        );

        for (i, bucket) in self.buckets.iter().enumerate() {
            if grayscale {
                raw[4 * i + 0] = bucket.alpha;
                raw[4 * i + 1] = bucket.alpha;
                raw[4 * i + 2] = bucket.alpha;
            } else {
                raw[4 * i + 0] = bucket.red;
                raw[4 * i + 1] = bucket.green;
                raw[4 * i + 2] = bucket.blue;
            }
            raw[4 * i + 3] = 255;
        }
    }

    pub fn to_gray8(&self) -> GrayImage {
        let raw = self.buckets.iter().map(|b| b.alpha).collect();
        ImageBuffer::from_raw(self.width as u32, self.height as u32, raw)
            .expect("incorrect image buffer size")
    }

    pub fn to_rgb8(&self) -> RgbImage {
        let raw = self
            .buckets
            .iter()
            .flat_map(|b| b.iter_rgb().cloned())
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
