use std::collections::HashSet;

use nalgebra::{Affine2, Matrix3, Transform};
use rand::{distr::Distribution, distr::weighted::WeightedIndex, seq::SliceRandom, Rng};
use rand_distr::Normal;

use super::{
    Flame,
    bounds::Bounds,
    color::{Color, Palette},
    function::{Function, FunctionEntry},
    random::FlameDistribution,
    variation::Variation,
};

/// Configuration for the genetic flame evolution algorithm
#[derive(Clone)]
pub struct EvolveConfig<DF, DS, DN, DP> {
    pub flame_distr: FlameDistribution<DF, DS, DN, DP>,

    // standard deviations for Gaussian noise applied each generation
    pub palette_key_mutability: f32,
    pub palette_color_mutability: f32,
    pub affine_mutability: f32,
    pub variation_param_mutability: f32,
    pub weight_mutability: f32,
    pub color_mutability: f32,
    pub color_speed_mutability: f32,
    pub bounds_mutability: f32,

    // probabilities for discrete mutation operators (independent per individual)
    pub function_insertion_rate: f32,
    pub color_insertion_rate: f32,
    pub symmetry_replacement_rate: f32,
    pub last_replacement_rate: f32,

    pub pop_size: usize,
    /// Controls clone vs. recombination: P(clone) = asexuality / (1 + asexuality)
    pub asexuality: f32,
    /// Fit individuals get weight fitness_weight / (1 + fitness_weight);
    /// unfit individuals get weight 1 / (1 + fitness_weight)
    pub fitness_weight: f32,
}

/// Generate an initial random population
pub fn evolve_init<DF, DS, DN, DP>(cfg: &EvolveConfig<DF, DS, DN, DP>) -> Vec<Flame>
where
    DF: Distribution<FunctionEntry>,
    DS: Distribution<i8>,
    DN: Distribution<usize>,
    DP: Distribution<Palette>,
{
    let mut rng = rand::rng();
    (0..cfg.pop_size)
        .map(|_| rng.sample(&cfg.flame_distr))
        .collect()
}

/// Produce the next generation from the previous one and a set of fit individual indices
pub fn evolve_step<DF, DS, DN, DP>(
    prev_gen: Vec<Flame>,
    fit: HashSet<usize>,
    cfg: &EvolveConfig<DF, DS, DN, DP>,
) -> Vec<Flame>
where
    DF: Distribution<FunctionEntry> + Distribution<Function>,
    DS: Distribution<i8>,
{
    let mut rng = rand::rng();
    let fit_w = cfg.fitness_weight / (1.0 + cfg.fitness_weight);
    let unfit_w = 1.0 / (1.0 + cfg.fitness_weight);

    let weights: Vec<f32> = (0..prev_gen.len())
        .map(|i| if fit.contains(&i) { fit_w } else { unfit_w })
        .collect();
    let weighted = WeightedIndex::new(&weights).expect("population must be non-empty");

    let p_clone = cfg.asexuality / (1.0 + cfg.asexuality);

    (0..cfg.pop_size)
        .map(|_| {
            let mut offspring = if rng.random::<f32>() < p_clone {
                prev_gen[weighted.sample(&mut rng)].clone()
            } else {
                let a = &prev_gen[weighted.sample(&mut rng)];
                let b = &prev_gen[weighted.sample(&mut rng)];
                recombine(a, b, &mut rng)
            };
            mutate(&mut offspring, cfg, &mut rng);
            offspring
        })
        .collect()
}

// --- Recombination ---

fn recombine(a: &Flame, b: &Flame, rng: &mut impl Rng) -> Flame {
    // functions: sample n without replacement from the combined pool
    let min_len = a.functions.len().min(b.functions.len());
    let max_len = a.functions.len().max(b.functions.len());
    let n = rng.random_range(min_len..=max_len);
    let mut pool: Vec<FunctionEntry> = a.functions.iter().chain(&b.functions).cloned().collect();
    pool.shuffle(rng);
    pool.truncate(n);

    // last and symmetry: coin flip
    let last = if rng.random::<bool>() {
        a.last.clone()
    } else {
        b.last.clone()
    };
    let symmetry = if rng.random::<bool>() {
        a.symmetry
    } else {
        b.symmetry
    };

    // bounds: field-wise mean
    let bounds = Bounds::new(
        (a.bounds.x_min + b.bounds.x_min) / 2.0,
        (a.bounds.x_max + b.bounds.x_max) / 2.0,
        (a.bounds.y_min + b.bounds.y_min) / 2.0,
        (a.bounds.y_max + b.bounds.y_max) / 2.0,
    );

    // palette: random size, i-th color by coin flip; evenly-spaced keys
    let min_colors = a.palette.len().min(b.palette.len());
    let max_colors = a.palette.len().max(b.palette.len());
    let s = rng.random_range(min_colors..=max_colors);
    let colors: Vec<Color> = (0..s)
        .map(|i| match (a.palette.get(i), b.palette.get(i)) {
            (Some((c, _)), None) | (None, Some((c, _))) => c,
            (Some((ca, _)), Some((cb, _))) => {
                if rng.random::<bool>() {
                    ca
                } else {
                    cb
                }
            }
            (None, None) => unreachable!(),
        })
        .collect();
    let palette = Palette::new::<std::iter::Empty<f32>>(colors, None)
        .expect("palette recombination produced invalid palette");

    Flame {
        functions: pool,
        last,
        symmetry,
        palette,
        bounds,
    }
}

// --- Mutation ---

fn mutate<DF, DS, DN, DP>(flame: &mut Flame, cfg: &EvolveConfig<DF, DS, DN, DP>, rng: &mut impl Rng)
where
    DF: Distribution<FunctionEntry> + Distribution<Function>,
    DS: Distribution<i8>,
{
    let weight_dist = Normal::new(0.0, cfg.weight_mutability).unwrap();
    let color_dist = Normal::new(0.0, cfg.color_mutability).unwrap();
    let color_speed_dist = Normal::new(0.0, cfg.color_speed_mutability).unwrap();

    // continuous perturbations on all function entries
    for entry in &mut flame.functions {
        entry.function.affine_pre =
            perturb_affine(&entry.function.affine_pre, cfg.affine_mutability, rng);
        entry.function.affine_post =
            perturb_affine(&entry.function.affine_post, cfg.affine_mutability, rng);
        mutate_variation(
            &mut entry.function.variation,
            cfg.variation_param_mutability,
            rng,
        );
        entry.weight = (entry.weight + rng.sample(weight_dist)).max(1e-6);
        entry.color = (entry.color + rng.sample(color_dist)).clamp(0.0, 1.0);
        entry.color_speed = (entry.color_speed + rng.sample(color_speed_dist)).clamp(0.0, 1.0);
    }

    // perturb the final transform
    flame.last.affine_pre = perturb_affine(&flame.last.affine_pre, cfg.affine_mutability, rng);
    flame.last.affine_post = perturb_affine(&flame.last.affine_post, cfg.affine_mutability, rng);
    mutate_variation(
        &mut flame.last.variation,
        cfg.variation_param_mutability,
        rng,
    );

    // perturb palette colors and interior keys
    mutate_palette(
        &mut flame.palette,
        cfg.palette_key_mutability,
        cfg.palette_color_mutability,
        rng,
    );

    // perturb bounds, ensuring x_min < x_max and y_min < y_max
    let bounds_dist = Normal::new(0.0, cfg.bounds_mutability).unwrap();
    flame.bounds.x_min += rng.sample(bounds_dist);
    flame.bounds.x_max += rng.sample(bounds_dist);
    flame.bounds.y_min += rng.sample(bounds_dist);
    flame.bounds.y_max += rng.sample(bounds_dist);
    if flame.bounds.x_min >= flame.bounds.x_max {
        std::mem::swap(&mut flame.bounds.x_min, &mut flame.bounds.x_max);
        if flame.bounds.x_min >= flame.bounds.x_max {
            flame.bounds.x_max = flame.bounds.x_min + 1e-6;
        }
    }
    if flame.bounds.y_min >= flame.bounds.y_max {
        std::mem::swap(&mut flame.bounds.y_min, &mut flame.bounds.y_max);
        if flame.bounds.y_min >= flame.bounds.y_max {
            flame.bounds.y_max = flame.bounds.y_min + 1e-6;
        }
    }

    // discrete: function insertion and deletion (independent)
    if rng.random::<f32>() < cfg.function_insertion_rate {
        let entry: FunctionEntry = rng.sample(&cfg.flame_distr.func_distr);
        flame.functions.push(entry);
    }
    if rng.random::<f32>() < cfg.function_insertion_rate && !flame.functions.is_empty() {
        let idx = rng.random_range(0..flame.functions.len());
        flame.functions.remove(idx);
    }

    // discrete: color insertion and deletion (independent)
    if rng.random::<f32>() < cfg.color_insertion_rate {
        insert_random_color(&mut flame.palette, rng);
    }
    if rng.random::<f32>() < cfg.color_insertion_rate && flame.palette.len() > 2 {
        let idx = rng.random_range(0..flame.palette.len());
        flame.palette.remove(idx);
    }

    // discrete: symmetry replacement
    if rng.random::<f32>() < cfg.symmetry_replacement_rate {
        flame.symmetry = rng.sample(&cfg.flame_distr.symmetry_distr);
    }

    // discrete: final transform replacement
    if rng.random::<f32>() < cfg.last_replacement_rate {
        flame.last = rng.sample(&cfg.flame_distr.func_distr);
    }
}

fn perturb_affine(affine: &Affine2<f32>, stddev: f32, rng: &mut impl Rng) -> Affine2<f32> {
    let dist = Normal::new(0.0, stddev).unwrap();
    let m = affine.matrix();
    Transform::from_matrix_unchecked(Matrix3::new(
        m.m11 + rng.sample(dist),
        m.m12 + rng.sample(dist),
        m.m13 + rng.sample(dist),
        m.m21 + rng.sample(dist),
        m.m22 + rng.sample(dist),
        m.m23 + rng.sample(dist),
        0.0,
        0.0,
        1.0,
    ))
}

fn mutate_variation(var: &mut Variation, stddev: f32, rng: &mut impl Rng) {
    let (discr, mut params) = var.clone().deconstruct();
    if params.is_empty() {
        return;
    }
    let dist = Normal::new(0.0, stddev).unwrap();
    for p in &mut params {
        *p += rng.sample(dist);
    }
    *var = Variation::build(discr, params).expect("deconstruct/build roundtrip must succeed");
}

fn mutate_palette(palette: &mut Palette, key_stddev: f32, color_stddev: f32, rng: &mut impl Rng) {
    let len = palette.len();

    // collect all colors and interior keys (palette positions 1..len-1)
    let mut colors: Vec<Color> = (0..len).map(|i| palette.get(i).unwrap().0).collect();
    let mut interior_keys: Vec<f32> = (1..len - 1).map(|i| palette.get(i).unwrap().1).collect();

    // perturb each color channel
    let color_dist = Normal::new(0.0, color_stddev).unwrap();
    for color in &mut colors {
        color.red = (color.red as f32 + rng.sample(color_dist)).clamp(0.0, 255.0) as u8;
        color.green = (color.green as f32 + rng.sample(color_dist)).clamp(0.0, 255.0) as u8;
        color.blue = (color.blue as f32 + rng.sample(color_dist)).clamp(0.0, 255.0) as u8;
    }

    // perturb interior keys and restore monotonicity
    let key_dist = Normal::new(0.0, key_stddev).unwrap();
    for key in &mut interior_keys {
        *key = (*key + rng.sample(key_dist)).clamp(1e-9, 1.0 - 1e-9);
    }
    interior_keys.sort_unstable_by(f32::total_cmp);

    if let Ok(new_palette) = Palette::new(colors, Some(interior_keys)) {
        *palette = new_palette;
    }
    // on failure (keys degenerate), leave palette unchanged
}

fn insert_random_color(palette: &mut Palette, rng: &mut impl Rng) {
    let len = palette.len();
    // pick a random gap [idx, idx+1] to insert into
    let idx = rng.random_range(0..len - 1);
    let k0 = palette.get(idx).unwrap().1;
    let k1 = palette.get(idx + 1).unwrap().1;

    if k0 >= k1 {
        return; // degenerate interval, skip
    }
    let new_key = rng.random_range(k0..k1);
    let new_color: Color = rng.random();

    // collect existing data, insert new color and key, rebuild
    let mut colors: Vec<Color> = (0..len).map(|i| palette.get(i).unwrap().0).collect();
    let mut interior_keys: Vec<f32> = (1..len - 1).map(|i| palette.get(i).unwrap().1).collect();

    colors.insert(idx + 1, new_color);
    // the new color at position idx+1 is interior (since idx < len-1 means idx+1 < len,
    // and after insertion len becomes len+1, so idx+1 < len = new len - 1).
    // its interior_keys index is idx+1 - 1 = idx.
    interior_keys.insert(idx, new_key);

    if let Ok(new_palette) = Palette::new(colors, Some(interior_keys)) {
        *palette = new_palette;
    }
}
