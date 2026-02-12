use rand::Rng;
use serde::{Deserialize, Serialize};
use nalgebra::Point2;

use std::f32::consts::{PI, FRAC_1_PI, TAU};

use flame_macro::variation;

#[variation]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Variation {
    Id,
    Sinusoidal,
    Spherical, // r2
    Swirl, // r
    Horseshoe, // r
    Polar, // r, theta
    Handkerchief, // r, theta
    Heart, // r, theta
    Disc, // r, theta
    BrokenDisc, // r, theta
    Spiral, // r, theta
    Hyperbolic, // r, theta
    Diamond, // r, theta
    Ex, // r, theta
    Bent, // r
    Fisheye, // r
    Eyefish,
    Exponential,
    Power, // r, theta
    Cosine,
    Cylinder,
    Tangent,
    Bubble, // r2
    Cross,
    Blob(f32, f32, f32), // theta
    Pdj(f32, f32, f32, f32),
    Fan2(f32, f32), // theta
    Rings2(f32), // theta
    Perspective(f32, f32),
    Curl(f32, f32),
    Noise,
    Gaussian,
    JuliaScope(f32, f32),
    // Square,
}

use self::Variation::*;

struct RandVars<'a, R: Rng>(&'a mut R);

impl<'a, R: Rng> RandVars<'a, R> {
    fn psi(&mut self) -> f32 { self.0.random() }

    #[allow(unused)]
    fn omega(&mut self) -> f32 {
        (self.0.random::<bool>() as u8) as f32 * PI
    }

    fn lambda(&mut self) -> f32 {
        (2 * self.0.random::<bool>() as u8 - 1) as f32
    }
}

impl Variation {
    pub fn eval(self, rng: &mut impl Rng, arg: Point2<f32>) -> Point2<f32> {
        let (x, y) = (arg[0], arg[1]);

        let mut r_: Option<f32> = None;
        let mut r = || {
            match r_ {
                Some(r__) => r__,
                None => {
                    let r__ = (x*x + y*y).sqrt();
                    r_ = Some(r__);
                    r__
                }
            }
        };

        let mut theta_: Option<f32> = None;
        let mut theta = || {
            match theta_ {
                Some(theta__) => theta__,
                None => {
                    // let theta__ = x.atan2(y);
                    // theta_ = Some(theta__);
                    // theta__
                    let theta__ = if y == 0.0 {
                        if x == 0.0 {
                            0.0
                        } else if x > 0.0 {
                            0.5 * PI
                        } else {
                            1.5 * PI
                        }
                    } else {
                        (x / y).atan()
                    };
                    theta_ = Some(theta__);
                    theta__
                }
            }
        };

        let mut phi_: Option<f32> = None;
        let mut phi = || {
            match phi_ {
                Some(phi__) => phi__,
                None => {
                    let phi__ = y.atan2(x);
                    phi_ = Some(phi__);
                    phi__
                }
            }
        };

        let mut rv = RandVars(rng);

        let (xo, yo) = match self {
            Id => (x, y),
            Sinusoidal => (x.sin(), y.sin()),
            Spherical => { let r2 = x*x + y*y; (x / r2, y / r2) }
            Swirl => {
                let r2 = x*x + y*y;
                (x * r2.sin() - y * r2.cos(), x * r2.cos() + y * r2.sin())
            }
            Horseshoe => ((x - y) * (x + y) / r(), 2.0 * x * y / r()),
            Polar => (theta() * FRAC_1_PI, r() - 1.0),
            Handkerchief => ((theta() + r()).sin(), (theta() - r()).cos()),
            Heart => (r() * (theta() * r()).sin(), -r() * (theta() * r()).cos()),
            Disc => (theta() * FRAC_1_PI * (PI * r()).sin(), theta() * FRAC_1_PI * (PI * r()).cos()),
            BrokenDisc => (theta() * FRAC_1_PI * (PI * r()).sin(), theta() * FRAC_1_PI),
            Spiral => ((y + r().sin()) / r(), (x - r().cos()) / r()),
            Hyperbolic => (x / r(), r() * y),
            Diamond => (theta().sin() * r().cos(), theta().cos() * r().sin()),
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
                let a = r().powf(x - 1.0);
                (a * y, a * x)
            }
            Cosine => ((PI * x).cos() * y.cosh(), -(PI * x).sin() * y.sinh()),
            Cylinder => (x.sin(), y),
            Tangent => (x.sin() / y.cos(), y.tan()),
            Bubble => { let a = 4.0 / (x*x + y*y + 4.0); (a * x, a * y ) }
            Cross => { let a = 1.0 / (x*x - y*y).abs(); (a * x, a * x) }
            Blob(h, l, w) => {
                let a = r() * (l + (h - l) / 2.0 * (1.0 + (theta() * w).sin()));
                (a * y, a * x)
            }
            Pdj(a, b, c, d) => ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos()),
            Fan2(a, b) => {
                let p1 = PI * a * a;
                let t = theta() + b - p1 * (2. * theta() * b / p1).trunc();
                let sgn = if t > p1 / 2. { -1. } else { 1. };
                (r() * (theta() + sgn * p1 / 2.).cos(), r() * (theta() + sgn * p1 / 2.).sin())
            }
            Rings2(val) => {
                let p = val * val;
                let t = r() - 2. * p * ((r() + p) / 2. / p).trunc() + r() * (1. - p);
                (t * x, t * y)
            }
            Perspective(angle, dist) => {
                let a = dist / (dist - y * angle.sin());
                (a * x, a * y * angle.cos())
            }
            Curl(c1, c2) => {
                let t1 = 1. + c1 * x + c2 * (x*x - y*y);
                let t2 = c1 * y + 2. * c2 * x * y;
                let a = 1. / (t1 * t1 + t2 * t2);
                (a * (x * t1 + y * t2), a * (y * t1 - x * t2))
            }
            Noise => {
                let psi1 = rv.psi();
                let psi2 = TAU * rv.psi();
                (psi1 * x * psi2.cos(), psi1 * y * psi2.sin())
            }
            Gaussian => {
                let a: f32 = (0..4).map(|_| rv.psi() - 2.).sum();
                let psi5 = TAU * rv.psi();
                (a * psi5.cos(), psi5.sin())
            }
            JuliaScope(power, dist) => {
                let p3 = (power.abs() * rv.psi()).trunc();
                let t = (rv.lambda() * phi() + TAU * p3) / power;
                let a = r().powf(dist / power);
                (a * t.cos(), a * t.sin())
            }
            // Square => (rv.psi() - 0.5, rv.psi() - 0.5),
        };

        Point2::new(xo, yo)
    }
}

impl Default for Variation {
    fn default() -> Self {
        Id
    }
}
