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

impl<D: Distribution<f32> + Copy> Distribution<Variation> for VariationDistribution<D> {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Variation {
        let discr: VariationDiscriminant = rng.random();
        let params = self.0.sample_iter(rng)
            .take(discr.num_parameters());
        Variation::build(discr, params).unwrap()
    }
}

#[derive(Clone)]
pub struct FunctionDistribution<DF,DV> {
    pub weight_distr: DF,
    pub var_distr: DV
}

impl<DF,DV> Distribution<Function> for FunctionDistribution<DF, DV>
where
    DF: Distribution<f32>,
    DV: Distribution<Variation>
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Function {
        let affine = [
            self.weight_distr.sample(rng), self.weight_distr.sample(rng), self.weight_distr.sample(rng),
            self.weight_distr.sample(rng), self.weight_distr.sample(rng), self.weight_distr.sample(rng)
        ];
        let var = self.var_distr.sample(rng);
        Function::new(var, affine)
    }
}

impl<DF,DV> Distribution<FunctionEntry> for FunctionDistribution<DF, DV>
where
    DF: Distribution<f32>,
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
pub struct FlameDistribution<
    DF: Distribution<FunctionEntry>,
    RI: SampleRange<i8>,
    RU: SampleRange<usize>,
    RC: SampleRange<usize>
> {
    pub func_distr: DF,
    pub palette_distr: PaletteDistribution<RC>,
    pub symmetry_range: RI,
    pub func_num_range: RU
}

impl<DF,RI,RU,RC> Distribution<Flame> for FlameDistribution<DF,RI,RU,RC>
where
    DF: Distribution<FunctionEntry> + Clone,
    RI: SampleRange<i8> + Clone,
    RU: SampleRange<usize> + Clone,
    RC: SampleRange<usize> + Clone
{
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Flame {
        let num_funcs = rng.random_range(self.func_num_range.clone());
        let functions: Vec<_> = rng.sample_iter(self.func_distr.clone()).take(num_funcs).collect();
        let symmetry = rng.random_range(self.symmetry_range.clone());
        Flame {
            functions,
            symmetry,
            last: Function::default(),
            palette: self.palette_distr.sample(rng),
            bounds: Bounds::new(-1., 1., -1., 1.)
        }
    }
}
