use std::collections::HashSet;

use rusqlite::Connection;

const DB_PATH: &str = "flames.db";

use xilem::{
    Blob, Color, EventLoop, WidgetView, WindowOptions, Xilem,
    core::fork,
    dpi::LogicalSize,
    masonry::peniko::ImageData,
    style::Style,
    view::{
        MainAxisAlignment, button, flex_col, flex_row, image, label, portal, spinner, split, task,
        task_raw, text_button, text_input,
    },
    winit::error::EventLoopError,
};

use flame::{
    BlurConfig, Flame, RenderConfig,
    evolve::{EvolveConfig, evolve_init, evolve_step},
};
use flame::{RunConfig, random::FlameDistribution};

/// String-backed mirror of `EvolveConfig`, `RunConfig`, and `RenderConfig`
/// for use as Xilem widget state.
///
/// Each scalar field is stored as a `String` so it can be bound directly to
/// a text-input widget.
/// Use `to_evolve_config`, `to_run_config`, and `to_render_config` to
/// obtain validated config structs.
#[derive(Clone)]
pub struct ConfigFields {
    // FlameDistribution params
    pub uniformity: String,
    pub skewness: String,
    pub symmetry_min: String,
    pub symmetry_max: String,
    pub symmetry_prob: String,
    pub func_num_min: String,
    pub func_num_max: String,
    pub color_num_min: String,
    pub color_num_max: String,

    // standard deviations for Gaussian noise applied each generation
    pub palette_key_mutability: String,
    pub palette_color_mutability: String,
    pub affine_mutability: String,
    pub variation_float_param_mutability: String,
    pub variation_int_param_mut_rate: String,
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
    pub blur_strength: String,
    pub blur_sigma_max: String,
}

impl ConfigFields {
    pub fn to_evolve_config(&self) -> Option<EvolveConfig> {
        macro_rules! parse_f32 {
            ($field:ident) => {
                self.$field.parse::<f32>().ok()?
            };
        }

        let pop_size = self.pop_size.parse::<usize>().ok()?;
        if pop_size == 0 {
            return None;
        }

        let uniformity = parse_f32!(uniformity);
        let skewness = parse_f32!(skewness);
        let sym_min = self.symmetry_min.parse::<i8>().ok()?;
        let sym_max = self.symmetry_max.parse::<i8>().ok()?;
        let fn_min = self.func_num_min.parse::<usize>().ok()?;
        let fn_max = self.func_num_max.parse::<usize>().ok()?;
        let col_min = self.color_num_min.parse::<usize>().ok()?;
        let col_max = self.color_num_max.parse::<usize>().ok()?;
        if sym_min > sym_max || fn_min > fn_max || col_min > col_max {
            return None;
        }

        Some(EvolveConfig {
            flame_distr: FlameDistribution {
                uniformity,
                skewness,
                symmetry_vals: (sym_min..=sym_max)
                    .filter(|s| !matches!(s, -1 | 0 | 1))
                    .collect(),
                symmetry_prob: self.symmetry_prob.parse::<f32>().ok()?,
                func_num_vals: (fn_min..=fn_max).collect(),
                color_num_vals: (col_min..=col_max).collect(),
            },
            palette_key_mutability: parse_f32!(palette_key_mutability),
            palette_color_mutability: parse_f32!(palette_color_mutability),
            affine_mutability: parse_f32!(affine_mutability),
            variation_float_param_mutability: parse_f32!(variation_float_param_mutability),
            variation_int_param_mut_rate: parse_f32!(variation_int_param_mut_rate),
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
        let blur_k: f64 = self.blur_strength.parse().ok()?;
        let blur_sigma_max: f64 = self.blur_sigma_max.parse().ok()?;
        let blur = (blur_sigma_max > 0.0).then(|| BlurConfig {
            strength: blur_k,
            sigma_min: 0.0,
            sigma_max: blur_sigma_max,
            patch_sigma: 2.0,
        });
        Some(RenderConfig {
            brightness: self.brightness.parse().ok()?,
            grayscale: false,
            blur,
        })
    }
}

impl Default for ConfigFields {
    fn default() -> Self {
        ConfigFields {
            uniformity: String::from("0.5"),
            skewness: String::from("0.5"),
            symmetry_min: String::from("-3"),
            symmetry_max: String::from("5"),
            symmetry_prob: String::from("0.35"),
            func_num_min: String::from("4"),
            func_num_max: String::from("7"),
            color_num_min: String::from("3"),
            color_num_max: String::from("7"),
            palette_key_mutability: String::from("0.04"),
            palette_color_mutability: String::from("8"),
            affine_mutability: String::from("0.05"),
            variation_float_param_mutability: String::from("0.1"),
            variation_int_param_mut_rate: String::from("0.15"),
            weight_mutability: String::from("0.05"),
            color_mutability: String::from("0.1"),
            color_speed_mutability: String::from("0.05"),
            bounds_mutability: String::from("0.1"),
            function_insertion_rate: String::from("0.05"),
            color_insertion_rate: String::from("0.1"),
            symmetry_replacement_rate: String::from("0.1"),
            last_replacement_rate: String::from("0.05"),
            pop_size: String::from("100"),
            recomb_rate: String::from("0.5"),
            immaculate_rate: String::from("0.01"),
            fitness_weight: String::from("3"),
            width: String::from("250"),
            height: String::from("250"),
            iters: String::from("20000000"),
            threads: String::from("10"),
            brightness: String::from("22"),
            blur_strength: String::from("0.015"),
            blur_sigma_max: String::from("0.03"),
        }
    }
}

struct AppData {
    // user input
    config: ConfigFields,
    selected: HashSet<usize>,
    // internal data
    pop: Vec<Flame>,
    images: Vec<ImageData>,
    generation: u32,
    timestamp: i64,
    // state tracking
    generating: bool,
    rendered: bool,
    saving: bool,
}

impl AppData {
    fn new() -> Self {
        AppData {
            config: Default::default(),
            pop: vec![],
            images: vec![],
            selected: HashSet::new(),
            generating: false,
            rendered: true,
            saving: false,
            generation: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }

    fn is_processing(&self) -> bool {
        self.generating || !self.rendered
    }

    fn clear(&mut self) {
        *self = AppData {
            config: self.config.clone(),
            ..AppData::new()
        }
    }

    fn log_gen(&self) {
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
        for (i, flame) in self.pop.iter().enumerate() {
            let blob = flame.to_mp().unwrap();
            let is_selected = self.selected.contains(&i) as i64;
            let _ = conn.execute(
                "INSERT INTO flames (timestamp, generation, selected, blob)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![self.timestamp, self.generation as i64, is_selected, blob],
            );
        }
    }
}

fn render_images(
    pop: &[Flame],
    run_cfg: RunConfig,
    render_cfg: RenderConfig
) -> Vec<ImageData>
{
    pop.iter()
        .map(|flame| {
            let mut img_buf = vec![0u8; run_cfg.width * run_cfg.height * 4];
            flame.run(run_cfg).render_raw_rgba(
                &mut img_buf,
                render_cfg,
                run_cfg.iters,
                flame.bounds,
            );
            ImageData {
                data: Blob::new(std::sync::Arc::new(img_buf.into_boxed_slice())),
                format: xilem::ImageFormat::Rgba8,
                alpha_type: xilem::masonry::peniko::ImageAlphaType::Alpha,
                width: run_cfg.width as u32,
                height: run_cfg.height as u32,
            }
        })
        .collect()
}


fn rerender_button(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let b = text_button("Re-render", |data: &mut AppData| data.rendered = false);
    let t = task_raw(
        move |proxy, data: &mut AppData| {
            let run_cfg = data.config.to_run_config().unwrap();
            let render_cfg = data.config.to_render_config().unwrap();
            let pop = data.pop.clone();
            async move {
                xilem::tokio::task::spawn_blocking(move || {
                    let images = render_images(&pop, run_cfg, render_cfg);
                    let _ = proxy.message(images);
                })
                .await
                .unwrap();
            }
        },
        |data: &mut AppData, images| {
            data.images = images;
            data.rendered = true;
        },
    );

    fork(b, (!data.rendered).then_some(t))
}


fn next_gen_button(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let b = text_button("Next generation", |data: &mut AppData| data.generating = true);
    let t = task_raw(
        move |proxy, state: &mut AppData| {
            let evolve_config =
                state.config.to_evolve_config().unwrap();
            let run_cfg = state.config.to_run_config().unwrap();
            let render_cfg = state.config.to_render_config().unwrap();
            let pop = state.pop.clone();
            let selected = state.selected.clone();
            async move { xilem::tokio::task::spawn_blocking(move || {
                let next_pop = if pop.is_empty() {
                    evolve_init(&evolve_config)
                } else {
                    evolve_step(pop, selected, &evolve_config)
                };
                let images = render_images(&next_pop, run_cfg, render_cfg);
                let _ = proxy.message((next_pop, images));
            }).await.unwrap();
        } },
        |data: &mut AppData, (next_pop, images)| {
            if !data.pop.is_empty() {
                data.log_gen();
            }
            data.pop = next_pop;
            data.images = images;
            data.selected.clear();
            data.generating = false;
            data.generation += 1;
        },
    );

    fork(b, data.generating.then_some(t))
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
            let to_save = data
                .pop
                .iter()
                .enumerate()
                .filter(|(i, _)| data.selected.is_empty() || data.selected.contains(i))
                .map(|(_, f)| f)
                .enumerate();
            for (i, flame) in to_save {
                flame.save(path.join(format!("{i}.flam3"))).unwrap();
            }
            data.saving = false;
        },
    );

    fork(b, data.saving.then_some(t))
}

fn clear_button() -> impl WidgetView<AppData> {
    text_button("Clear", |data: &mut AppData| data.clear())
}

fn config_panel(data: &mut AppData) -> impl WidgetView<AppData> + use<> {
    let c = &data.config;

    let gen_fields = flex_col((
        flex_row((
            label("uniformity"),
            text_input(c.uniformity.clone(), |d: &mut AppData, v| {
                d.config.uniformity = v
            }),
        )),
        flex_row((
            label("skewness"),
            text_input(c.skewness.clone(), |d: &mut AppData, v| {
                d.config.skewness = v
            }),
        )),
        flex_row((
            label("symmetry range"),
            text_input(c.symmetry_min.clone(), |d: &mut AppData, v| {
                d.config.symmetry_min = v
            }),
            label("-"),
            text_input(c.symmetry_max.clone(), |d: &mut AppData, v| {
                d.config.symmetry_max = v
            }),
        )),
        flex_row((
            label("symmetry prob"),
            text_input(c.symmetry_prob.clone(), |d: &mut AppData, v| {
                d.config.symmetry_prob = v
            }),
        )),
        flex_row((
            label("num functions range"),
            text_input(c.func_num_min.clone(), |d: &mut AppData, v| {
                d.config.func_num_min = v
            }),
            label("-"),
            text_input(c.func_num_max.clone(), |d: &mut AppData, v| {
                d.config.func_num_max = v
            }),
        )),
        flex_row((
            label("num colors range"),
            text_input(c.color_num_min.clone(), |d: &mut AppData, v| {
                d.config.color_num_min = v
            }),
            label("-"),
            text_input(c.color_num_max.clone(), |d: &mut AppData, v| {
                d.config.color_num_max = v
            }),
        )),
    ));

    let mutability_fields = flex_col((
        flex_row((
            label("palette key"),
            text_input(c.palette_key_mutability.clone(), |d: &mut AppData, v| {
                d.config.palette_key_mutability = v
            }),
        )),
        flex_row((
            label("palette color"),
            text_input(c.palette_color_mutability.clone(), |d: &mut AppData, v| {
                d.config.palette_color_mutability = v
            }),
        )),
        flex_row((
            label("affine"),
            text_input(c.affine_mutability.clone(), |d: &mut AppData, v| {
                d.config.affine_mutability = v
            }),
        )),
        flex_row((
            label("variation float param"),
            text_input(
                c.variation_float_param_mutability.clone(),
                |d: &mut AppData, v| d.config.variation_float_param_mutability = v,
            ),
        )),
        flex_row((
            label("variation int param rate"),
            text_input(
                c.variation_int_param_mut_rate.clone(),
                |d: &mut AppData, v| d.config.variation_int_param_mut_rate = v,
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
            text_input(c.color_speed_mutability.clone(), |d: &mut AppData, v| {
                d.config.color_speed_mutability = v
            }),
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
            text_input(c.function_insertion_rate.clone(), |d: &mut AppData, v| {
                d.config.function_insertion_rate = v
            }),
        )),
        flex_row((
            label("color insertion"),
            text_input(c.color_insertion_rate.clone(), |d: &mut AppData, v| {
                d.config.color_insertion_rate = v
            }),
        )),
        flex_row((
            label("symmetry replacement"),
            text_input(c.symmetry_replacement_rate.clone(), |d: &mut AppData, v| {
                d.config.symmetry_replacement_rate = v
            }),
        )),
        flex_row((
            label("last replacement"),
            text_input(c.last_replacement_rate.clone(), |d: &mut AppData, v| {
                d.config.last_replacement_rate = v
            }),
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
            text_input(c.width.clone(), |d: &mut AppData, v| d.config.width = v),
        )),
        flex_row((
            label("height"),
            text_input(c.height.clone(), |d: &mut AppData, v| d.config.height = v),
        )),
        flex_row((
            label("iters"),
            text_input(c.iters.clone(), |d: &mut AppData, v| d.config.iters = v),
        )),
        flex_row((
            label("threads"),
            text_input(c.threads.clone(), |d: &mut AppData, v| d.config.threads = v),
        )),
        flex_row((
            label("brightness"),
            text_input(c.brightness.clone(), |d: &mut AppData, v| {
                d.config.brightness = v
            }),
        )),
        flex_row((
            label("blur strength"),
            text_input(c.blur_strength.clone(), |d: &mut AppData, v| {
                d.config.blur_strength = v
            }),
        )),
        flex_row((
            label("blur sigma max"),
            text_input(c.blur_sigma_max.clone(), |d: &mut AppData, v| {
                d.config.blur_sigma_max = v
            }),
        )),
    ));

    flex_col((
        label("Generation").text_size(25.0),
        gen_fields,
        label("Mutation rates").text_size(25.0),
        mutation_fields,
        label("Mutability").text_size(25.0),
        mutability_fields,
        label("Propagation").text_size(25.0),
        prop_fields,
        label("Rendering").text_size(25.0),
        render_fields,
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

    button(image(img_data), move |data: &mut AppData| {
        if !data.selected.remove(&idx) {
            data.selected.insert(idx);
        }
    })
    .background_color(c)
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
        data.is_processing().then_some(spinner()),
        label(format!("Generation: {}", data.generation)),
        clear_button(),
        rerender_button(data),
        save_button(data),
        next_gen_button(data),
    ))
    .main_axis_alignment(MainAxisAlignment::End);

    split(
        portal(config_panel(data)).padding(15.0),
        portal(flex_col((gallery_rows, button_row))).padding(15.0),
    )
    .split_point(0.2)
}

fn main() -> Result<(), EventLoopError> {
    let app = Xilem::new_simple(
        AppData::new(),
        app_logic,
        WindowOptions::new("Flame evolve").with_initial_inner_size(LogicalSize::new(1600, 800)),
    );
    app.run_in(EventLoop::with_user_event())?;
    Ok(())
}
