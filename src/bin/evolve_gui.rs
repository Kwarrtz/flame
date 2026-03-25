use std::{collections::HashSet, ops::RangeInclusive};

use rand::distr::StandardUniform;
use rand::distr::uniform::Uniform;
use rusqlite::Connection;

const DB_PATH: &str = "flames.db";

use xilem::{
    Blob, Color, EventLoop, WidgetView, WindowOptions, Xilem, core::fork, dpi::LogicalSize, masonry::peniko::ImageData, style::Style, view::{
        MainAxisAlignment, button, flex_col, flex_row, image, label, portal, spinner, split, task, task_raw, text_button, text_input
    }, winit::error::EventLoopError
};

use flame::{RunConfig, random::{
    AffineDistribution, DefaultFlameDistribution, FlameDistribution,
    FunctionDistribution, PaletteDistribution, VariationDistribution,
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

/// String-backed mirror of `EvolveConfig`, `RunConfig`, and `RenderConfig`
/// for use as Xilem widget state.
///
/// Each scalar field is stored as a `String` so it can be bound directly to
/// a text-input widget. `flame_distr` is kept as `DefaultFlameDistribution`
/// because it cannot be meaningfully represented as a single string.
/// Use `to_evolve_config`, `to_run_config`, and `to_render_config` to
/// obtain validated config structs.
#[derive(Clone)]
pub struct ConfigFields {
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
    pub recomb_rate: String,
    pub immaculate_rate: String,
    pub fitness_weight: String,

    // RunConfig fields
    pub width: String,
    pub height: String,
    pub iters: String,
    pub threads: String,

    // RenderConfig fields (grayscale always false)
    pub brightness: String,
}

impl ConfigFields {
    pub fn to_evolve_config(&self) -> Option<DefaultEvolveConfig> {
        macro_rules! parse_f32 {
            ($field:ident) => {
                self.$field.parse::<f32>().ok()?
            };
        }

        let pop_size = self.pop_size.parse::<usize>().ok()?;
        if pop_size == 0 {
            return None;
        }

        Some(EvolveConfig {
            flame_distr: FlameDistribution {
                func_distr: self.flame_distr.func_distr.clone(),
                symmetry_distr: self.flame_distr.symmetry_distr,
                func_num_distr: self.flame_distr.func_num_distr,
                palette_distr: self.flame_distr.palette_distr.clone(),
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
            recomb_rate: parse_f32!(recomb_rate),
            immaculate_rate: parse_f32!(immaculate_rate),
            fitness_weight: parse_f32!(fitness_weight),
        })
    }

    pub fn to_run_config(&self) -> Option<RunConfig> {
        Some(RunConfig {
            width: self.width.parse().ok()?,
            height: self.height.parse().ok()?,
            iters: self.iters.parse().ok()?,
            threads: self.threads.parse().ok()?,
        })
    }

    pub fn to_render_config(&self) -> Option<RenderConfig> {
        Some(RenderConfig {
            brightness: self.brightness.parse().ok()?,
            grayscale: false,
        })
    }
}

impl Default for ConfigFields {
    fn default() -> Self {
        ConfigFields {
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
            recomb_rate: String::from("0.5"),
            immaculate_rate: String::from("0.01"),
            fitness_weight: String::from("3"),
            width: String::from("250"),
            height: String::from("250"),
            iters: String::from("10000000"),
            threads: String::from("10"),
            brightness: String::from("20"),
        }
    }
}

struct AppData {
    config: ConfigFields,
    pop: Vec<Flame>,
    images: Vec<ImageData>,
    selected: HashSet<usize>,
    processing: bool,
    saving: bool,
    generation: u32,
    timestamp: i64,
}

impl AppData {
    fn new() -> Self {
        AppData {
            config: Default::default(),
            pop: vec![],
            images: vec![],
            selected: HashSet::new(),
            processing: false,
            saving: false,
            generation: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}

fn log_gen(
    timestamp: i64,
    generation: u32,
    selected: &HashSet<usize>,
    pop: &[Flame],
) {
    let Ok(conn) = Connection::open(DB_PATH) else {
        return;
    };
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS flames (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp  INTEGER NOT NULL,
            generation INTEGER NOT NULL,
            selected   INTEGER NOT NULL,
            blob       BLOB NOT NULL
        )",
    );
    for (i, flame) in pop.iter().enumerate() {
        let Ok(blob) = flame.to_mp() else { continue };
        let is_selected = selected.contains(&i) as i64;
        let _ = conn.execute(
            "INSERT INTO flames (timestamp, generation, selected, blob)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![timestamp, generation as i64, is_selected, blob],
        );
    }
}

fn next_gen_button(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let b = text_button("Next generation", |data: &mut AppData| {
        data.processing = true
    });
    let t = task_raw(
            move |proxy, state: &mut AppData| {
                let evolve_config: DefaultEvolveConfig =
                    state.config.to_evolve_config().unwrap();
                let run_cfg = state.config.to_run_config().unwrap();
                let render_cfg = state.config.to_render_config().unwrap();
                let pop = state.pop.clone();
                let selected = state.selected.clone();
                async move {
                    let next_pop = if pop.is_empty() {
                        evolve_init(&evolve_config)
                    } else {
                        evolve_step(pop, selected, &evolve_config)
                    };

                    let images = next_pop
                        .iter()
                        .map(|flame| {
                            let mut img_buf =
                                vec![0u8; run_cfg.width * run_cfg.height * 4];
                            flame
                                .run(run_cfg)
                                .render_raw_rgba(
                                    &mut img_buf,
                                    render_cfg,
                                    run_cfg.iters,
                                );
                            ImageData {
                                data: Blob::new(std::sync::Arc::new(
                                    img_buf.into_boxed_slice(),
                                )),
                                format: xilem::ImageFormat::Rgba8,
                                alpha_type: xilem::masonry::peniko::ImageAlphaType::Alpha,
                                width: run_cfg.width as u32,
                                height: run_cfg.height as u32,
                            }
                        })
                        .collect::<Vec<_>>();

                    let _ = proxy.message((next_pop, images));
                }
            },
            |data: &mut AppData, (next_pop, images)| {
                // save outgoing generation with its selection state
                // before overwriting; skip generation 0 (empty pop)
                if !data.pop.is_empty() {
                    log_gen(
                        data.timestamp,
                        data.generation,
                        &data.selected,
                        &data.pop,
                    );
                }
                data.pop = next_pop;
                data.images = images;
                data.selected.clear();
                data.processing = false;
                data.generation += 1;
            },
        );

    fork(b, data.processing.then_some(t))
}

fn save_button(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let b = text_button("Save", |data: &mut AppData| data.saving = true);
    
    let t = task(
        |proxy, _| async move {
            let dialog = rfd::AsyncFileDialog::new().save_file();
            if let Some(handle) = dialog.await {
                let _ = proxy.message(handle);      
            }
        },
        |data: &mut AppData, handle| {
            let path = handle.path();
            if !path.exists() {
                std::fs::create_dir(path).unwrap();
            }
            let to_save = data.pop.iter()
                .enumerate()
                .filter(|(i, _)| data.selected.contains(i))
                .map(|(_, f)| f)
                .enumerate();
            for (i, flame) in to_save {
                flame.save(path.join(format!("{i}.flam3"))).unwrap();
            }
            data.saving = false;
        }
    );

    fork(b, data.saving.then_some(t))
}

fn config_panel(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let c = &data.config;

    let mutability_fields = flex_col((
        flex_row((
            label("palette key"),
            text_input(c.palette_key_mutability.clone(), |d: &mut AppData, v| {
                d.config.palette_key_mutability = v
            }),
        )),
        flex_row((
            label("palette color"),
            text_input(
                c.palette_color_mutability.clone(),
                |d: &mut AppData, v| d.config.palette_color_mutability = v,
            ),
        )),
        flex_row((
            label("affine"),
            text_input(c.affine_mutability.clone(), |d: &mut AppData, v| {
                d.config.affine_mutability = v
            }),
        )),
        flex_row((
            label("variation param"),
            text_input(
                c.variation_param_mutability.clone(),
                |d: &mut AppData, v| d.config.variation_param_mutability = v,
            ),
        )),
        flex_row((
            label("weight"),
            text_input(c.weight_mutability.clone(), |d: &mut AppData, v| {
                d.config.weight_mutability = v
            }),
        )),
        flex_row((
            label("color"),
            text_input(c.color_mutability.clone(), |d: &mut AppData, v| {
                d.config.color_mutability = v
            }),
        )),
        flex_row((
            label("color speed"),
            text_input(
                c.color_speed_mutability.clone(),
                |d: &mut AppData, v| d.config.color_speed_mutability = v,
            ),
        )),
        flex_row((
            label("bounds"),
            text_input(c.bounds_mutability.clone(), |d: &mut AppData, v| {
                d.config.bounds_mutability = v
            }),
        )),
    ));

    let mutation_fields = flex_col((
        flex_row((
            label("function insertion"),
            text_input(
                c.function_insertion_rate.clone(),
                |d: &mut AppData, v| d.config.function_insertion_rate = v,
            ),
        )),
        flex_row((
            label("color insertion"),
            text_input(
                c.color_insertion_rate.clone(),
                |d: &mut AppData, v| d.config.color_insertion_rate = v,
            ),
        )),
        flex_row((
            label("symmetry replacement"),
            text_input(
                c.symmetry_replacement_rate.clone(),
                |d: &mut AppData, v| d.config.symmetry_replacement_rate = v,
            ),
        )),
        flex_row((
            label("last replacement"),
            text_input(
                c.last_replacement_rate.clone(),
                |d: &mut AppData, v| d.config.last_replacement_rate = v,
            ),
        )),
    ));

    let prop_fields = flex_col((
        flex_row((
            label("pop size"),
            text_input(c.pop_size.clone(), |d: &mut AppData, v| {
                d.config.pop_size = v
            }),
        )),
        flex_row((
            label("immaculate rate"),
            text_input(c.immaculate_rate.clone(), |d: &mut AppData, v| {
                d.config.immaculate_rate = v
            }),
        )),
        flex_row((
            label("recomb rate"),
            text_input(c.recomb_rate.clone(), |d: &mut AppData, v| {
                d.config.recomb_rate = v
            }),
        )),
        flex_row((
            label("fitness weight"),
            text_input(c.fitness_weight.clone(), |d: &mut AppData, v| {
                d.config.fitness_weight = v
            }),
        )),
    ));
    
    let render_fields = flex_col((
        flex_row((
            label("width"),
            text_input(c.width.clone(), |d: &mut AppData, v| {
                d.config.width = v
            }),
        )),
        flex_row((
            label("height"),
            text_input(c.height.clone(), |d: &mut AppData, v| {
                d.config.height = v
            }),
        )),
        flex_row((
            label("iters"),
            text_input(c.iters.clone(), |d: &mut AppData, v| {
                d.config.iters = v
            }),
        )),
        flex_row((
            label("threads"),
            text_input(c.threads.clone(), |d: &mut AppData, v| {
                d.config.threads = v
            }),
        )),
        flex_row((
            label("brightness"),
            text_input(c.brightness.clone(), |d: &mut AppData, v| {
                d.config.brightness = v
            }),
        )),
    ));

    flex_col((
        label("Mutation rates").text_size(25.0),
        mutation_fields,
        label("Mutability").text_size(25.0),
        mutability_fields,
        label("Propagation").text_size(25.0),
        prop_fields,
        label("Rendering").text_size(25.0),
        render_fields        
    ))
}

fn selectable_image(
    idx: usize,
    img_data: ImageData,
    selected: &HashSet<usize>,
) -> impl WidgetView<AppData> + use<> {
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
                    .map(|(idx, img)| {
                        selectable_image(*idx, img.clone(), &data.selected)
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let button_row = flex_row((
        data.processing.then_some(spinner()),
        label(format!("Generation: {}", data.generation)),
        save_button(data),
        next_gen_button(data),
    )).main_axis_alignment(MainAxisAlignment::End);

    split(
        portal(config_panel(data)).padding(15.0),
        portal(flex_col((gallery_rows, button_row))).padding(15.0),
    ).split_point(0.2)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        AppData::new(),
        app_logic,
        WindowOptions::new("Flame evolve").with_initial_inner_size(LogicalSize::new(1600,800)),
    );
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
