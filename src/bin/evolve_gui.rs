use std::{collections::HashSet, ops::RangeInclusive};

use rand::distr::StandardUniform;
use rand::distr::uniform::Uniform;

use xilem::{
    Blob, Color, EventLoop, WidgetView, WindowOptions, Xilem, core::fork, masonry::peniko::ImageData, style::Style, view::{
        MainAxisAlignment, button, flex_col, flex_row, image, label, portal, spinner, split, task_raw, text_button, text_input
    }, winit::error::EventLoopError
};

use flame::{RunConfig, random::{
    AffineDistribution, DefaultFlameDistribution, FlameDistribution, FunctionDistribution,
    PaletteDistribution, VariationDistribution,
}};
use flame::{
    Flame, RenderConfig,
    evolve::{EvolveConfig, evolve_init, evolve_step},
};

type DefaultEvolveConfig = EvolveConfig<
    FunctionDistribution<AffineDistribution, VariationDistribution<StandardUniform>>,
    Uniform<i8>,
    Uniform<usize>,
    PaletteDistribution<RangeInclusive<usize>>,
>;

/// String-backed mirror of `EvolveConfig` for use as Xilem widget state.
///
/// Each scalar field is stored as a `String` so it can be bound directly to a
/// text-input widget. `flame_distr` is kept as `DefaultFlameDistribution`
/// because it cannot be meaningfully represented as a single string.
/// Convert to a validated `EvolveConfig` via `TryFrom`.
#[derive(Clone)]
pub struct EvolveConfigFields {
    pub flame_distr: DefaultFlameDistribution,

    // standard deviations for Gaussian noise applied each generation
    pub palette_key_mutability: String,
    pub palette_color_mutability: String,
    pub affine_mutability: String,
    pub variation_param_mutability: String,
    pub weight_mutability: String,
    pub color_mutability: String,
    pub color_speed_mutability: String,
    pub bounds_mutability: String,

    // probabilities for discrete mutation operators (independent per individual)
    pub function_insertion_rate: String,
    pub color_insertion_rate: String,
    pub symmetry_replacement_rate: String,
    pub last_replacement_rate: String,

    pub pop_size: String,
    /// controls clone vs. recombination: P(clone) = asexuality / (1 + asexuality)
    pub asexuality: String,
    /// fit individuals get weight fitness_weight / (1 + fitness_weight);
    /// unfit individuals get weight 1 / (1 + fitness_weight)
    pub fitness_weight: String,
}

#[derive(Debug)]
pub enum EvolveConfigFieldsError {
    ParseFloat {
        field: &'static str,
        source: std::num::ParseFloatError,
    },
    ParseInt {
        field: &'static str,
        source: std::num::ParseIntError,
    },
    ZeroPopSize,
}

impl std::fmt::Display for EvolveConfigFieldsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseFloat { field, source } => write!(f, "invalid value for {field}: {source}"),
            Self::ParseInt { field, source } => write!(f, "invalid value for {field}: {source}"),
            Self::ZeroPopSize => write!(f, "pop_size must be greater than zero"),
        }
    }
}

impl std::error::Error for EvolveConfigFieldsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ParseFloat { source, .. } => Some(source),
            Self::ParseInt { source, .. } => Some(source),
            Self::ZeroPopSize => None,
        }
    }
}

impl TryFrom<EvolveConfigFields> for DefaultEvolveConfig {
    type Error = EvolveConfigFieldsError;

    fn try_from(fields: EvolveConfigFields) -> Result<Self, Self::Error> {
        macro_rules! parse_f32 {
            ($field:ident) => {
                fields.$field.parse::<f32>().map_err(|source| {
                    EvolveConfigFieldsError::ParseFloat {
                        field: stringify!($field),
                        source,
                    }
                })?
            };
        }

        let pop_size = fields.pop_size.parse::<usize>().map_err(|source| {
            EvolveConfigFieldsError::ParseInt {
                field: "pop_size",
                source,
            }
        })?;
        if pop_size == 0 {
            return Err(EvolveConfigFieldsError::ZeroPopSize);
        }

        Ok(EvolveConfig {
            flame_distr: FlameDistribution {
                func_distr: fields.flame_distr.func_distr,
                symmetry_distr: fields.flame_distr.symmetry_distr,
                func_num_distr: fields.flame_distr.func_num_distr,
                palette_distr: fields.flame_distr.palette_distr,
            },
            palette_key_mutability: parse_f32!(palette_key_mutability),
            palette_color_mutability: parse_f32!(palette_color_mutability),
            affine_mutability: parse_f32!(affine_mutability),
            variation_param_mutability: parse_f32!(variation_param_mutability),
            weight_mutability: parse_f32!(weight_mutability),
            color_mutability: parse_f32!(color_mutability),
            color_speed_mutability: parse_f32!(color_speed_mutability),
            bounds_mutability: parse_f32!(bounds_mutability),
            function_insertion_rate: parse_f32!(function_insertion_rate),
            color_insertion_rate: parse_f32!(color_insertion_rate),
            symmetry_replacement_rate: parse_f32!(symmetry_replacement_rate),
            last_replacement_rate: parse_f32!(last_replacement_rate),
            pop_size,
            asexuality: parse_f32!(asexuality),
            fitness_weight: parse_f32!(fitness_weight),
        })
    }
}

const RUN_CFG: RunConfig = RunConfig {
    iters: 10_000_000,
    width: 250,
    height: 250,
    threads: 10
};

const RENDER_CFG: RenderConfig = RenderConfig {
     brightness: 20.0,
     width: RUN_CFG.width,
     height: RUN_CFG.height,
     grayscale: false
};

impl Default for EvolveConfigFields {
    fn default() -> Self {
        EvolveConfigFields {
            flame_distr: Default::default(),
            palette_key_mutability: String::from("0.04"),
            palette_color_mutability: String::from("10"),
            affine_mutability: String::from("0.05"),
            variation_param_mutability: String::from("0.1"),
            weight_mutability: String::from("0.1"),
            color_mutability: String::from("0.1"),
            color_speed_mutability: String::from("0.05"),
            bounds_mutability: String::from("0.1"),
            function_insertion_rate: String::from("0.05"),
            color_insertion_rate: String::from("0.1"),
            symmetry_replacement_rate: String::from("0"),
            last_replacement_rate: String::from("0"),
            pop_size: String::from("100"),
            asexuality: String::from("1"),
            fitness_weight: String::from("3"),
        }
    }
}

struct AppData {
    evolve_config: EvolveConfigFields,
    pop: Vec<Flame>,
    images: Vec<ImageData>,
    selected: HashSet<usize>,
    processing: bool,
    generation: u32,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            evolve_config: Default::default(),
            pop: vec![],
            images: vec![],
            selected: HashSet::new(),
            processing: false,
            generation: 0,
        }
    }
}

fn next_gen_button(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let b = text_button("Next generation", |data: &mut AppData| {
        data.processing = true
    });
    let t = task_raw(
            move |proxy, state: &mut AppData| {
                let config: DefaultEvolveConfig =
                    state.evolve_config.clone().try_into().unwrap();
                let pop = state.pop.clone();
                let selected = state.selected.clone();
                async move {
                    let next_pop = if pop.is_empty() {
                        evolve_init(&config)
                    } else {
                        evolve_step(pop, selected, &config)
                    };

                    let images = next_pop
                        .iter()
                        .map(|flame| {
                            let mut img_buf = [0u8; RENDER_CFG.width * RENDER_CFG.height * 4];
                            flame
                                .run(RUN_CFG)
                                .render_raw_rgba(&mut img_buf, RENDER_CFG, RUN_CFG.iters);
                            ImageData {
                                data: Blob::new(std::sync::Arc::new(img_buf)),
                                format: xilem::ImageFormat::Rgba8,
                                alpha_type: xilem::masonry::peniko::ImageAlphaType::Alpha,
                                width: RENDER_CFG.width as u32,
                                height: RENDER_CFG.height as u32,
                            }
                        })
                        .collect::<Vec<_>>();

                    let _ = proxy.message((next_pop, images));
                }
            },
            |data: &mut AppData, (next_pop, images)| {
                data.pop = next_pop;
                data.images = images;
                data.selected.clear();
                data.processing = false;
                data.generation += 1;
            },
        );

    fork(b, data.processing.then_some(t))
}

fn config_view(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let c = &data.evolve_config;
    flex_col((
        flex_row((
            label("palette key mutability"),
            text_input(c.palette_key_mutability.clone(), |d: &mut AppData, v| {
                d.evolve_config.palette_key_mutability = v
            }),
        )),
        flex_row((
            label("palette color mutability"),
            text_input(c.palette_color_mutability.clone(), |d: &mut AppData, v| {
                d.evolve_config.palette_color_mutability = v
            }),
        )),
        flex_row((
            label("affine mutability"),
            text_input(c.affine_mutability.clone(), |d: &mut AppData, v| {
                d.evolve_config.affine_mutability = v
            }),
        )),
        flex_row((
            label("variation param mutability"),
            text_input(
                c.variation_param_mutability.clone(),
                |d: &mut AppData, v| d.evolve_config.variation_param_mutability = v,
            ),
        )),
        flex_row((
            label("weight mutability"),
            text_input(c.weight_mutability.clone(), |d: &mut AppData, v| {
                d.evolve_config.weight_mutability = v
            }),
        )),
        flex_row((
            label("color mutability"),
            text_input(c.color_mutability.clone(), |d: &mut AppData, v| {
                d.evolve_config.color_mutability = v
            }),
        )),
        flex_row((
            label("color speed mutability"),
            text_input(c.color_speed_mutability.clone(), |d: &mut AppData, v| {
                d.evolve_config.color_speed_mutability = v
            }),
        )),
        flex_row((
            label("bounds mutability"),
            text_input(c.bounds_mutability.clone(), |d: &mut AppData, v| {
                d.evolve_config.bounds_mutability = v
            }),
        )),
        flex_row((
            label("function insertion rate"),
            text_input(c.function_insertion_rate.clone(), |d: &mut AppData, v| {
                d.evolve_config.function_insertion_rate = v
            }),
        )),
        flex_row((
            label("color insertion rate"),
            text_input(c.color_insertion_rate.clone(), |d: &mut AppData, v| {
                d.evolve_config.color_insertion_rate = v
            }),
        )),
        flex_row((
            label("symmetry replacement rate"),
            text_input(c.symmetry_replacement_rate.clone(), |d: &mut AppData, v| {
                d.evolve_config.symmetry_replacement_rate = v
            }),
        )),
        flex_row((
            label("last replacement rate"),
            text_input(c.last_replacement_rate.clone(), |d: &mut AppData, v| {
                d.evolve_config.last_replacement_rate = v
            }),
        )),
        flex_row((
            label("pop size"),
            text_input(c.pop_size.clone(), |d: &mut AppData, v| {
                d.evolve_config.pop_size = v
            }),
        )),
        flex_row((
            label("asexuality"),
            text_input(c.asexuality.clone(), |d: &mut AppData, v| {
                d.evolve_config.asexuality = v
            }),
        )),
        flex_row((
            label("fitness weight"),
            text_input(c.fitness_weight.clone(), |d: &mut AppData, v| {
                d.evolve_config.fitness_weight = v
            }),
        )),
    ))
}

fn selectable_image(idx: usize, img_data: ImageData, selected: &HashSet<usize>) -> impl WidgetView<AppData> + use<> {
    let c = if selected.contains(&idx) {
        Color::from_rgba8(0, 255, 0, 128)
    } else {
        Color::TRANSPARENT
    };

    button(
        image(img_data),
        move |data: &mut AppData| {
            if !data.selected.remove(&idx) {
                data.selected.insert(idx);
            }
        }
    ).background_color(c)
}

fn app_logic(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let gallery_rows = data
        .images
        .iter()
        .cloned()
        .enumerate()
        .collect::<Vec<_>>()
        .chunks(4)
        .map(|chunk| {
            flex_row(
                chunk
                    .iter()
                    .map(|(idx, img)| selectable_image(*idx, img.clone(), &data.selected))
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let button_row = flex_row((
        data.processing.then_some(spinner()),
        label(format!("Generation: {}", data.generation)),
        next_gen_button(data),
    )).main_axis_alignment(MainAxisAlignment::End);

    split(
        config_view(data),
        portal(flex_col((gallery_rows, button_row))),
    )
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        AppData::default(),
        app_logic,
        WindowOptions::new("Flame evolve"),
    );
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
