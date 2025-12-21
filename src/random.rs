use std::f32::consts::TAU;

use nalgebra::{Affine2, Matrix3, Rotation2, Similarity2, Transform, Vector2};
use rand::{distr::{uniform::SampleRange, Distribution, StandardUniform}, seq::IndexedRandom, Rng};

use crate::bounds::Bounds;

use super::variation::*;
use super::function::*;
use super::color::*;
use super::Flame;

impl Distribution<VariationDiscriminant> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> VariationDiscriminant {
        VARIATION_DISCRIMINANTS.choose(rng).unwrap().clone()
    }
}

#[derive(Clone)]
pub struct VariationDistribution<D: Distribution<f32>>(pub D);

impl<D: Distribution<f32>> Distribution<Variation> for VariationDistribution<D> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Variation {
        let discr: VariationDiscriminant = rng.random();
        let params = (&self.0).sample_iter(rng)
            .take(discr.num_parameters());
        Variation::build(discr, params).unwrap()
    }
}

#[derive(Clone)]
pub struct NaiveAffineDistribution<D: Distribution<f32>>(pub D);

impl<D: Distribution<f32>> Distribution<Affine2<f32>> for NaiveAffineDistribution<D> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Affine2<f32> {
        Transform::from_matrix_unchecked(Matrix3::new(
            self.0.sample(rng), self.0.sample(rng), self.0.sample(rng),
            self.0.sample(rng), self.0.sample(rng), self.0.sample(rng),
            0., 0., 1.
        ))
    }
}

#[derive(Clone)]
pub struct AffineDistribution {
    pub uniformity: f32,
    pub skewness: f32,
}

impl Distribution<Affine2<f32>> for AffineDistribution {
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

#[derive(Clone)]
pub struct FunctionDistribution<DA,DV> {
    pub aff_distr: DA,
    pub var_distr: DV
}

impl<DA,DV> Distribution<Function> for FunctionDistribution<DA,DV>
where
    DA: Distribution<Affine2<f32>>,
    DV: Distribution<Variation>
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Function {
        let affine_pre = self.aff_distr.sample(rng);
        let affine_post = self.aff_distr.sample(rng);
        let variation = self.var_distr.sample(rng);
        Function {
            variation, affine_pre, affine_post
        }
    }
}

impl<DA,DV> Distribution<FunctionEntry> for FunctionDistribution<DA, DV>
where
    DA: Distribution<Affine2<f32>>,
    DV: Distribution<Variation>
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> FunctionEntry {
        FunctionEntry::new(self.sample(rng), rng.random(), rng.random(), rng.random()).unwrap()
    }
}

impl Distribution<Color> for StandardUniform {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Color {
        Color::rgb(rng.random(), rng.random(), rng.random())
    }
}

#[derive(Clone)]
pub struct PaletteDistribution<RL: SampleRange<usize>>(pub RL);

impl<RL: SampleRange<usize> + Clone> Distribution<Palette> for PaletteDistribution<RL> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Palette {
        let num_colors = rng.random_range(self.0.clone());
        let colors = rng.random_iter().take(num_colors);
        Palette::new::<std::iter::Empty<f32>>(colors, None).unwrap()
    }
}

#[derive(Clone)]
pub struct FlameDistribution<DF,DS,DN,DP> {
    pub func_distr: DF,
    pub symmetry_distr: DS,
    pub func_num_distr: DN,
    pub palette_distr: DP
}

impl<DF,DS,DN,DP> Distribution<Flame> for FlameDistribution<DF,DS,DN,DP>
where
    DF: Distribution<FunctionEntry>,
    DS: Distribution<i8>,
    DN: Distribution<usize>,
    DP: Distribution<Palette>
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Flame {
        let num_funcs = rng.sample(&self.func_num_distr);
        let functions: Vec<_> = rng.sample_iter(&self.func_distr).take(num_funcs).collect();
        let symmetry = rng.sample(&self.symmetry_distr);
        Flame {
            functions,
            symmetry,
            last: Function::default(),
            palette: self.palette_distr.sample(rng),
            bounds: Bounds::new(-1., 1., -1., 1.)
        }
    }
}
