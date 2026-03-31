use std::{f32::consts::TAU};

use nalgebra::{Affine2, Matrix3, Rotation2, Similarity2, Vector2};
use rand::{Rng, RngExt, distr::{Distribution, StandardUniform}, seq::IndexedRandom};

use crate::bounds::Bounds;

use super::variation::*;
use super::function::*;
use super::color::*;
use super::Flame;

impl Distribution<VariationDiscriminant> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> VariationDiscriminant {
        *VARIATION_DISCRIMINANTS.choose(rng).unwrap()
    }
}

impl Distribution<Variation> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Variation {
        let discr: VariationDiscriminant = rng.random();
        let (int_ranges, float_ranges) = discr.parameter_ranges();
        let int_params = int_ranges.iter()
            .map(|range| match range {
                Some(r) => rng.random_range(r.clone()),
                None => rng.random_range(1..6),
            })
            .collect();
        let float_params = float_ranges.iter()
            .map(|range| match range {
                Some(r) => rng.random_range(r.clone()),
                None => rng.random_range(-1.0..1.0)
            })
            .collect();
        Variation::try_from(DynamicVariation { discriminant: discr, int_params, float_params })
            .unwrap()
    }
}

impl Distribution<Color> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Color {
        Color::rgb(rng.random(), rng.random(), rng.random())
    }
}

#[derive(Clone)]
pub struct FlameDistribution {
    pub uniformity: f32,
    pub skewness: f32,
    pub func_num_vals: Vec<usize>,
    pub color_num_vals: Vec<usize>,
    pub symmetry_vals: Vec<i8>,
    pub symmetry_prob: f32,
}

impl Default for FlameDistribution {
    fn default() -> Self {
        FlameDistribution {
            uniformity: 0.5,
            skewness: 0.5,
            func_num_vals: (3..=7).collect(),
            color_num_vals: (3..=7).collect(),
            symmetry_vals: vec![0],
            symmetry_prob: 0.0,
        }
    }
}

impl Distribution<Affine2<f32>> for FlameDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Affine2<f32> {
        let trans_angle = Rotation2::new(rng.random_range(0.0..TAU));
        let translation = trans_angle * Vector2::new(rng.random(), 0.);

        let angle = rng.random_range(0.0..TAU);
        let scaling = rng.random_range(-1.0..1.0);
        let sim = Similarity2::new(translation, angle, scaling);

        let prerot = Rotation2::new(rng.random_range(0.0..TAU));

        let nonuniform_scale = rng.random_range(self.uniformity..1.0);
        let skew = rng.random_range(-self.skewness..self.skewness);
        let aff = Affine2::from_matrix_unchecked(Matrix3::new(
            nonuniform_scale, skew, 0.0,
            0.0, nonuniform_scale.recip(), 0.0,
            0.0, 0.0, 1.0
        ));
        sim * aff * prerot
    }
}

impl Distribution<Function> for FlameDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Function {
        let affine_pre = self.sample(rng);
        let affine_post = self.sample(rng);
        let variation = rng.random();
        Function {
            variation, affine_pre, affine_post
        }
    }
}

impl Distribution<FunctionEntry> for FlameDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FunctionEntry {
        FunctionEntry::new(self.sample(rng), rng.random(), rng.random(), rng.random()).unwrap()
    }
}

impl Distribution<Palette> for FlameDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Palette {
        let num_colors = *self.color_num_vals.choose(rng).unwrap();
        let colors = rng.random_iter().take(num_colors);
        Palette::new::<std::iter::Empty<f32>>(colors, None).unwrap()
    }
}

impl FlameDistribution {
    pub fn random_symmetry<R: Rng + ?Sized>(&self, rng: &mut R) -> i8 {
        if rng.random::<f32>() < self.symmetry_prob {
            0
        } else {
            *self.symmetry_vals.choose(rng).unwrap()
        }
    }
}

impl Distribution<Flame> for FlameDistribution {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Flame {
        let num_funcs = *self.func_num_vals.choose(rng).unwrap();
        let functions: Vec<_> = rng.sample_iter(self).take(num_funcs).collect();
        let symmetry = self.random_symmetry(rng);
        Flame {
            functions,
            symmetry,
            last: Function::default(),
            palette: self.sample(rng),
            bounds: Bounds::new(-1., 1., -1., 1.)
        }
    }
}
