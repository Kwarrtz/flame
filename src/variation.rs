use nalgebra::{Affine2, Point2};
use rand::Rng;
use serde::{Deserialize, Serialize};

use std::f32::consts::{FRAC_1_PI, PI, TAU};

use flame_macro::variation;

/// A non-linear transformation, which can be stochastic and depend
/// on the coefficients of the associated affine pre-transform
/// or on constant parameters
#[variation]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Variation {
    Id,
    Sinusoidal,
    Spherical,
    Swirl,
    Horseshoe,
    Polar,
    Handkerchief,
    BrokenHandkerchief,
    Heart,
    Disc,
    BrokenDisc,
    Spiral,
    Hyperbolic,
    Diamond,
    Ex,
    Bent,
    Fisheye,
    Eyefish,
    Exponential,
    Power,
    Cosine,
    Cylinder,
    Tangent,
    Bubble,
    Cross,
    Secant,
    // stochastic: use random variables
    Noise,
    Gaussian,
    Julia,
    Blur,
    Arch,
    Rays,
    Blade,
    Twintrian,
    Square,
    // dependent: use affine pre-transform coefficients
    Waves,
    Popcorn,
    Rings,
    Fan,
    // parametric
    Blob(#[range(1..7)] i8, f32, f32),
    Pdj(f32, f32, f32, f32),
    Fan2(f32, f32),
    Rings2(#[range(0.01..2.0)] f32),
    Perspective(f32, #[range(0.1..5.0)] f32),
    Curl(f32, f32),
    JuliaN(#[range(1..9)] i8, f32),
    JuliaScope(#[range(1..9)] i8, f32),
    RadialBlur(#[range(-1.0..1.0)] f32),
    Pie(#[range(1..7)] i8, #[range(0.0..TAU)] f32, #[range(0.0..1.0)] f32),
    Ngon(#[range(3..9)] i8, f32, f32, f32),
    Rectangles(#[range(0.01..2.0)] f32, #[range(0.01..2.0)] f32),
}

use self::Variation::*;

/// A source of random variables used by stochastic `Variation`s
struct RandVars<'a, R: Rng>(&'a mut R);

impl<'a, R: Rng> RandVars<'a, R> {
    /// A random number between 0 and 1
    fn psi(&mut self) -> f32 {
        self.0.random()
    }

    /// A random number that is either 0 or pi
    fn omega(&mut self) -> f32 {
        (self.0.random::<bool>() as u8) as f32 * PI
    }

    /// A randum number that is either -1 or 1
    fn lambda(&mut self) -> f32 {
        (2 * self.0.random::<bool>() as u8 - 1) as f32
    }
}

impl Variation {
    pub fn eval(
        self,
        rng: &mut impl Rng,
        affine_pre: &Affine2<f32>,
        arg: Point2<f32>,
    ) -> Point2<f32> {
        let (x, y) = (arg[0], arg[1]);

        // common intermediate values, memoized and lazily evaluated

        let mut r_: Option<f32> = None;
        let mut r = || match r_ {
            Some(r__) => r__,
            None => {
                let r__ = (x * x + y * y).sqrt();
                r_ = Some(r__);
                r__
            }
        };

        let mut theta_: Option<f32> = None;
        let mut theta = || {
            match theta_ {
                Some(theta__) => theta__,
                None => {
                    let theta__ = x.atan2(y);
                    theta_ = Some(theta__);
                    theta__
                }
            }
        };

        let mut phi_: Option<f32> = None;
        let mut phi = || match phi_ {
            Some(phi__) => phi__,
            None => {
                let phi__ = y.atan2(x);
                phi_ = Some(phi__);
                phi__
            }
        };

        let mut rv = RandVars(rng);

        let (xo, yo) = match self {
            // normal
            Id => (x, y),
            Sinusoidal => (x.sin(), y.sin()),
            Spherical => {
                let r2 = x * x + y * y;
                (x / r2, y / r2)
            }
            Swirl => {
                let r2 = x * x + y * y;
                (x * r2.sin() - y * r2.cos(), x * r2.cos() + y * r2.sin())
            }
            Horseshoe => ((x - y) * (x + y) / r(), 2.0 * x * y / r()),
            Polar => (theta() * FRAC_1_PI, r() - 1.0),
            Handkerchief => (r() * (theta() + r()).sin(), r() * (theta() - r()).cos()),
            BrokenHandkerchief => ((theta() + r()).sin(), (theta() - r()).cos()),
            Heart => (r() * (theta() * r()).sin(), -r() * (theta() * r()).cos()),
            Disc => (
                theta() * FRAC_1_PI * (PI * r()).sin(),
                theta() * FRAC_1_PI * (PI * r()).cos(),
            ),
            BrokenDisc => (theta() * FRAC_1_PI * (PI * r()).sin(), theta() * FRAC_1_PI),
            Spiral => (
                (y / r() + r().sin()) / r(),
                (x / r() - r().cos()) / r(),
            ),
            Hyperbolic => (x / r() / r(), y),
            Diamond => (x / r() * r().cos(), y / r() * r().sin()),
            Ex => {
                let p0 = (theta() + r()).sin().powi(3);
                let p1 = (theta() - r()).cos().powi(3);
                (r() * (p0 + p1), r() * (p0 - p1))
            }
            Bent => {
                let a = if x >= 0.0 { x } else { 2.0 * x };
                let b = if y >= 0.0 { y } else { 0.5 * y };
                (a, b)
            }
            Fisheye => (2.0 * y / (r() + 1.0), 2.0 * x / (r() + 1.0)),
            Eyefish => (2.0 * x / (r() + 1.0), 2.0 * y / (r() + 1.0)),
            Exponential => (
                (x - 1.0).exp() * (PI * y).cos(),
                (x - 1.0).exp() * (PI * y).sin(),
            ),
            Power => {
                let a = r().powf(x / r());
                (a * y / r(), a * x / r())
            }
            Cosine => ((PI * x).cos() * y.cosh(), -(PI * x).sin() * y.sinh()),
            Cylinder => (x.sin(), y),
            Tangent => (x.sin() / y.cos(), y.tan()),
            Bubble => {
                let a = 4.0 / (x * x + y * y + 4.0);
                (a * x, a * y)
            }
            Cross => {
                let a = 1.0 / (x * x - y * y).abs();
                (a * x, a * y)
            }
            Secant => (x, r().cos().recip()),
            // stochastic
            Noise => {
                let psi1 = rv.psi();
                let psi2 = TAU * rv.psi();
                (psi1 * x * psi2.cos(), psi1 * y * psi2.sin())
            }
            Gaussian => {
                let a: f32 = (0..4).map(|_| rv.psi() - 2.).sum();
                let psi5 = TAU * rv.psi();
                (a * psi5.cos(), a * psi5.sin())
            }
            Julia => {
                let a = r().sqrt();
                let sgn = rv.lambda();
                let t = theta() / 2.0 + rv.omega();
                (a * sgn * t.cos(), a * sgn * t.sin())
            }
            Blur => {
                let a = rv.psi();
                let t = TAU * rv.psi();
                (a * t.cos(), a * t.sin())
            }
            Arch => {
                let a = rv.psi() * PI;
                let s = a.sin();
                (s, s * s / a.cos())
            }
            Rays => {
                let a = (rv.psi() * PI).tan() / (x * x + y * y);
                (a * x.cos(), a * y.sin())
            }
            Blade => {
                let a = rv.psi() * r();
                let (s, c) = a.sin_cos();
                (x * (c + s), x * (c - s))
            }
            Twintrian => {
                let a = rv.psi() * r();
                let s = a.sin();
                let t = (s * s).log10() + a.cos();
                (x * t, x * (t - PI * s))
            }
            Square => (rv.psi() - 0.5, rv.psi() - 0.5),
            // dependent
            Waves => {
                let mat = affine_pre.matrix();
                let b = mat.m12;
                let c2 = mat.m13 * mat.m13;
                let e = mat.m22;
                let f2 = mat.m23 * mat.m23;
                (x + b * (y / c2).sin(), y + e * (x / f2).sin())
            }
            Popcorn => {
                let mat = affine_pre.matrix();
                (
                    x + mat.m13 * (3.0 * y).tan().sin(),
                    y + mat.m23 * (3.0 * x).tan().sin(),
                )
            }
            Rings => {
                let c2 = affine_pre.matrix().m13.powi(2);
                let t = (r() + c2) % (2.0 * c2) - c2 + r() * (1.0 - c2);
                (t * x / r(), t * y / r())
            }
            Fan => {
                let mat = affine_pre.matrix();
                let t = PI * mat.m13 * mat.m13;
                let sgn = if (theta() + mat.m23) % t > t / 2.0 {
                    -1.0
                } else {
                    1.0
                };
                let a = theta() + sgn * t / 2.0;
                (r() * a.cos(), r() * a.sin())
            }
            // parametric
            Blob(w, h, l) => {
                let a = r() * (l + (h - l) / 2.0 * (1.0 + (theta() * w as f32).sin()));
                (a * y / r(), a * x / r())
            }
            Pdj(a, b, c, d) => ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos()),
            Fan2(a, b) => {
                let p1 = PI * a * a;
                let t = theta() + b - p1 * (2. * theta() * b / p1).trunc();
                let sgn = if t > p1 / 2. { -1. } else { 1. };
                (
                    r() * (theta() + sgn * p1 / 2.).sin(),
                    r() * (theta() + sgn * p1 / 2.).cos(),
                )
            }
            Rings2(val) => {
                let p = val * val;
                let t = r() - 2. * p * ((r() + p) / 2. / p).trunc() + r() * (1. - p);
                (t * x / r(), t * y / r())
            }
            Perspective(angle, dist) => {
                let a = dist / (dist - y * angle.sin());
                (a * x, a * y * angle.cos())
            }
            Curl(c1, c2) => {
                let t1 = 1. + c1 * x + c2 * (x * x - y * y);
                let t2 = c1 * y + 2. * c2 * x * y;
                let a = 1. / (t1 * t1 + t2 * t2);
                (a * (x * t1 + y * t2), a * (y * t1 - x * t2))
            }
            JuliaN(power, dist) => {
                let power = power as f32;
                let p3 = (power.abs() * rv.psi()).trunc();
                let t = (phi() + TAU * p3) / power;
                let a = r().powf(dist / power);
                (a * t.cos(), a * t.sin())
            }
            JuliaScope(power, dist) => {
                let power = power as f32;
                let p3 = (power.abs() * rv.psi()).trunc();
                let t = (rv.lambda() * phi() + TAU * p3) / power;
                let a = r().powf(dist / power);
                (a * t.cos(), a * t.sin())
            }
            RadialBlur(angle) => {
                let p1 = angle * PI / 2.0;
                let t1: f32 = (0..4).map(|_| rv.psi() - 2.0).sum();
                let t2 = phi() + t1 * p1.sin();
                let t3 = t1 * p1.cos() - 1.0;
                (r() * t2.cos() + t3 * x, r() * t2.sin() + t3 * y)
            }
            Pie(slices, rotation, thickness) => {
                let slices = slices as f32;
                let t1 = (rv.psi() * slices + 0.5).trunc();
                let t2 = rotation + (TAU / slices) * (t1 + rv.psi() * thickness);
                let a = rv.psi();
                (a * t2.cos(), a * t2.sin())
            }
            Ngon(sides, power, corners, circle) => {
                let p2 = TAU / sides as f32;
                let t3 = phi() - p2 * (phi() / p2).floor();
                let t4 = if t3 > p2 / 2.0 { t3 } else { t3 - p2 };
                let k = (corners * (1.0 / t4.cos() - 1.0) + circle)
                    / r().powf(power);
                (k * x, k * y)
            }
            Rectangles(rx, ry) => (
                (2.0 * (x / rx).floor() + 1.0) * rx - x,
                (2.0 * (y / ry).floor() + 1.0) * ry - y,
            ),
        };

        Point2::new(xo, yo)
    }
}

impl Default for Variation {
    fn default() -> Self {
        Id
    }
}
