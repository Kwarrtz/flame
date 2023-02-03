use serde::Deserialize;
use nalgebra::Point2;

use std::f32::consts::PI;
const PII: f32 = 1.0 / PI;

#[derive(Clone, Copy, Deserialize)]
pub enum Variation {
    Id,
    Sinusoidal,
    Spherical,
    Swirl,
    Horseshoe,
    Polar,
    Handkerchief,
    Heart,
    Disc,
    Spiral,
    Hyperbolic,
    Diamond,
    Ex,
    Bent,
    Fisheye,
    Eyefish,
    Exponential,
    Cylinder,
    Tangent,
    Blob(f32, f32, f32),
    PDJ(f32, f32, f32, f32),
}

impl Variation {
    pub fn eval(self, arg: Point2<f32>) -> Point2<f32> {
        use self::Variation::*;

        let (x, y) = (arg[0], arg[1]);
        let r = x.powi(2) + y.powi(2);
        let theta = if y == 0.0 {
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

        let (xo, yo) = match self {
            Id => (x, y),
            Sinusoidal => (x.sin(), y.sin()),
            Spherical => (x / r, y / r),
            Swirl => (x * r.sin() - y * r.cos(), x * r.cos() + y * r.sin()),
            Horseshoe => ((x - y) * (x + y) / r, 2.0 * x * y / r),
            Polar => (theta * PII, r - 1.0),
            Handkerchief => ((theta + r).sin(), (theta - r).cos()),
            Heart => (r * (theta * r).sin(), -r * (theta * r).cos()),
            Disc => (theta * PII * (PI * r).sin(), theta * PII),
            Spiral => ((theta.cos() + r.sin()) / r, (theta.sin() - r.cos()) / r),
            Hyperbolic => (theta.sin() / r, r * theta.cos()),
            Diamond => (theta.sin() * r.cos(), theta.cos() * r.sin()),
            Ex => {
                let p0 = (theta + r).sin().powi(3);
                let p1 = (theta - r).cos().powi(3);
                (r * (p0 + p1), r * (p0 - p1))
            }
            Bent => {
                let a = if x >= 0.0 { x } else { 2.0 * x };
                let b = if y >= 0.0 { y } else { 0.5 * y };
                (a, b)
            }
            Fisheye => (2.0 * y / (r + 1.0), 2.0 * x / (r + 1.0)),
            Eyefish => (2.0 * x / (r + 1.0), 2.0 * y / (r + 1.0)),
            Exponential => (
                (x - 1.0).exp() * (PI * y).cos(),
                (x - 1.0).exp() * (PI * y).sin(),
            ),
            Cylinder => (x.sin(), y),
            Tangent => (x.sin() / y.cos(), y.tan()),
            Blob(h, l, w) => {
                let a = r * (l + (h - l) / 2.0 * (1.0 + (theta * w).sin()));
                (a * theta.cos(), a * theta.sin())
            }
            PDJ(a, b, c, d) => ((a * y).sin() - (b * x).cos(), (c * x).sin() - (d * y).cos()),
        };

        Point2::new(xo, yo)
    }
}