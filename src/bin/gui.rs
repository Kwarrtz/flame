use kas::{
    cell_collection, collection,
    image::Sprite,
    prelude::*,
    runner::{Proxy, Runner},
    theme::{MarginStyle, MarkStyle},
    widgets::{
        Button, CheckBox, Column, ComboBox, EditBox, Filler, Frame, Grid, MarkButton, ScrollRegion,
        Separator, SpinBox, Splitter, adapt::AdaptEventCx, column, grid, row,
    },
};
use nalgebra::Affine2;
use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, Sender, channel},
    thread,
    time::Duration,
};

use flame::{
    self, Flame, RenderConfig,
    bounds::Bounds,
    buffer::Buffer,
    color::{Color, Palette},
    function::FunctionEntry,
    variation::{VARIATION_DISCRIMINANTS, Variation, VariationDiscriminant},
};

const ITERS_PER_LOOP: usize = 100_000;
const MAX_ITERS: usize = 1_000_000_000;
const IDLE_SLEEP_DUR: Duration = Duration::from_millis(30);

const DEFAULT_RENDER_CONFIG: RenderConfig = RenderConfig {
    width: 500,
    height: 500,
    brightness: 20.,
    grayscale: false,
};

#[derive(Debug)]
struct NewImage(Size, Vec<u8>);

fn generate_flame(
    rx_flame: Receiver<Flame>,
    rx_render_config: Receiver<RenderConfig>,
    mut proxy: Proxy,
) {
    let mut rng = rand::rng();

    let mut flame = rx_flame.recv().unwrap();
    let mut config = rx_render_config.recv().unwrap();

    let mut buffer: Buffer<u32> = Buffer::new(config.width, config.height);
    let mut iters = 0;

    loop {
        let mut rerender = false; // rerender image from buffer
        let mut restart = false; // create fresh buffer

        if iters < MAX_ITERS {
            // advance flame simulation
            flame.run_partial(&mut buffer, ITERS_PER_LOOP, &mut rng);
            iters += ITERS_PER_LOOP;
            rerender = true;
        } else {
            // if hit MAX_ITERS, sleep and wait for input
            thread::sleep(IDLE_SLEEP_DUR);
        }

        // received new flame
        if let Some(new_flame) = rx_flame.try_iter().last() {
            flame = new_flame;
            restart = true;
        }

        // received new render config
        if let Some(new_config) = rx_render_config.try_iter().last() {
            // dimensions changed
            if [new_config.width, new_config.height] != [config.width, config.height] {
                restart = true;
            }

            rerender = true;
            config = new_config;
        }

        if restart {
            buffer = Buffer::new(config.width, config.height);
            iters = 0;
        }

        if rerender {
            let mut img_buf = vec![255; 4 * config.width * config.height];
            buffer.render_raw_rgba(&mut img_buf, config, iters);

            let size = Size::new(config.width as i32, config.height as i32);
            if proxy.push(NewImage(size, img_buf)).is_err() {
                panic!("proxy closed");
            };
        }
    }
}

#[derive(Debug)]
enum RenderConfigUpdate {
    Width(usize),
    Height(usize),
    Grayscale(bool),
    Brightness(f64),
}

#[derive(Debug)]
struct FlameUpdate(Flame);

#[derive(Debug)]
struct SaveFileTo(Option<rfd::FileHandle>);

#[derive(Debug)]
struct LoadFileFrom(Option<rfd::FileHandle>);

struct AppData {
    config: RenderConfig,
    flame: Flame,
    config_tx: Sender<RenderConfig>,
    flame_tx: Sender<Flame>,
}

impl kas::runner::AppData for AppData {
    fn handle_message(&mut self, messages: &mut impl kas::runner::ReadMessage) {
        if let Some(msg) = messages.try_pop::<RenderConfigUpdate>() {
            use RenderConfigUpdate::*;
            match msg {
                Width(w) => self.config.width = w,
                Height(h) => self.config.height = h,
                Grayscale(gs) => self.config.grayscale = gs,
                Brightness(b) => self.config.brightness = b,
            }

            self.config_tx.send(self.config.clone()).unwrap();
        }

        if let Some(FlameUpdate(new_flame)) = messages.try_pop()
            && self.flame != new_flame
        {
            self.flame = new_flame;
            self.flame_tx.send(self.flame.clone()).unwrap();
        }

        if let Some(SaveFileTo(Some(handle))) = messages.try_pop() {
            self.flame.save(handle.path()).unwrap();
        }

        if let Some(LoadFileFrom(Some(handle))) = messages.try_pop() {
            self.flame = Flame::from_file(handle.path()).unwrap();
            self.flame_tx.send(self.flame.clone()).unwrap();
        }
    }
}

fn render_config_panel() -> impl Widget<Data = RenderConfig> {
    use RenderConfigUpdate::*;
    column![
        row![
            "Width:",
            EditBox::parser(|cfg: &RenderConfig| cfg.width, |value| Width(value))
                .with_width_em(3., 3.),
            "Height:",
            EditBox::parser(|cfg: &RenderConfig| cfg.height, |value| Height(value))
                .with_width_em(3., 3.),
        ],
        row![
            "Brightness:",
            EditBox::parser(
                |cfg: &RenderConfig| cfg.brightness,
                |value| Brightness(value)
            )
            .with_width_em(3., 3.),
            "Grayscale:",
            CheckBox::new_msg(
                |_, cfg: &RenderConfig| cfg.grayscale,
                |checked| Grayscale(checked)
            ),
        ],
    ]
}

fn misc_panel() -> impl Widget<Data = Flame> {
    column![
        row![
            "Bounds:",
            Frame::new(grid! {
                (0,0) => "X:",
                (1,0) => EditBox::parser(
                    |flame: &Flame| flame.bounds.x_min,
                    |val| val,
                )
                .with_width_em(3., 3.)
                .on_message_update_flame(|_, _, flame, val| {
                    flame.bounds.x_min = val;
                }),
                (2,0) => EditBox::parser(
                    |flame: &Flame| flame.bounds.x_max,
                    |val| val,
                )
                .with_width_em(3., 3.)
                .on_message_update_flame(|_, _, flame, val| {
                    flame.bounds.x_max = val;
                }),
                (0,1) => "Y:",
                (1,1) => EditBox::parser(
                    |flame: &Flame| flame.bounds.y_min,
                    |val| val,
                )
                .with_width_em(3., 3.)
                .on_message_update_flame(|_, _, flame, val| {
                    flame.bounds.y_min = val;
                }),
                (2,1) => EditBox::parser(
                    |flame: &Flame| flame.bounds.y_max,
                    |val| val,
                )
                .with_width_em(3., 3.)
                .on_message_update_flame(|_, _, flame, val| {
                    flame.bounds.y_max = val;
                }),
            }),
        ],
        row![
            "Symmetry:",
            EditBox::parser(|flame: &Flame| flame.symmetry, |val| val)
                .with_width_em(3., 3.)
                .on_message_update_flame(|_, _, flame, val| {
                    flame.symmetry = val;
                })
        ]
    ]
}

fn image_widget() -> impl Widget<Data = AppData> {
    Sprite::new()
        .with_logical_size((500., 500.))
        .with_stretch(Stretch::Maximize)
        .map_any()
        .on_configure(|cx, widget| {
            cx.set_send_target_for::<NewImage>(widget.id());
        })
        .on_message(|cx, widget, NewImage(size, buf)| {
            let action: ActionRedraw;
            if size == widget.inner.image_size()
                && let Some(handle) = widget.inner.handle()
            {
                action = cx.draw_shared().image_upload(handle, &buf).unwrap();
            } else {
                let new_handle = cx
                    .draw_shared()
                    .image_alloc(kas::draw::ImageFormat::Rgba8, size)
                    .unwrap();
                action = cx.draw_shared().image_upload(&new_handle, &buf).unwrap();
                widget.inner.set(cx, new_handle);
            }
            cx.action_redraw(action);
        })
}

// #[derive(Debug)]
// struct InnerVal<T>(T);

#[derive(Debug, Clone)]
struct ListAdd;

#[derive(Debug, Clone)]
struct ListRemove(usize);

trait FlameAdaptWidget: Widget<Data = Flame> + Sized {
    fn on_message_update_flame<T: Debug + 'static>(
        self,
        f: impl Fn(&mut AdaptEventCx, &mut Self, &mut Flame, T) + 'static,
    ) -> impl Widget<Data = Flame> {
        self.on_messages(move |cx, widget, flame: &Flame| {
            if let Some(val) = cx.try_pop::<T>() {
                let mut new = flame.clone();
                f(cx, widget, &mut new, val);
                cx.push(FlameUpdate(new));
            }
        })
    }
}

fn affine_field(index: (usize, usize)) -> impl Widget<Data = Affine2<f32>> {
    EditBox::parser(
        move |affine: &Affine2<f32>| affine.matrix()[index],
        |val| val,
    )
    .with_width_em(1., 1.5)
    .on_messages(move |cx, _, affine: &Affine2<f32>| {
        if let Some(val) = cx.try_pop::<f32>() {
            let mut new = affine.clone();
            new.matrix_mut_unchecked()[index] = val;
            cx.push(new);
        }
    })
}

fn affine() -> impl Widget<Data = Affine2<f32>> {
    grid! {
        (0, 0) => affine_field((0, 0)),
        (1, 0) => affine_field((0, 1)),
        (0, 1) => affine_field((1, 0)),
        (1, 1) => affine_field((1, 1)),
        (2, 0) => affine_field((0, 2)),
        (2, 1) => affine_field((1, 2))
    }
    .align(AlignHints {
        horiz: None,
        vert: Some(Align::Center),
    })
}

impl<T: Widget<Data = Flame> + Sized> FlameAdaptWidget for T {}

fn function_entry(index: usize) -> impl Widget<Data = Flame> {
    let affine_pre = affine()
        .map(move |flame: &Flame| &flame.functions[index].function.affine_pre)
        .on_message_update_flame(move |_, _, flame, affine: Affine2<f32>| {
            flame.functions[index].function.affine_pre = affine
        });

    let affine_post = affine()
        .map(move |flame: &Flame| &flame.functions[index].function.affine_post)
        .on_message_update_flame(move |_, _, flame, affine: Affine2<f32>| {
            flame.functions[index].function.affine_post = affine
        });

    let variation = ComboBox::new_msg(
        VARIATION_DISCRIMINANTS
            .iter()
            .map(|v| (format!("{v:?}"), v.clone())),
        move |_, flame: &Flame| flame.functions[index].function.variation.into(),
        |v| v,
    )
    .on_message_update_flame(move |_, _, flame, discr: VariationDiscriminant| {
        let var = Variation::build(discr, vec![0.0; discr.num_parameters()]).unwrap();
        flame.functions[index].function.variation = var;
    });

    Frame::new(Grid::new(cell_collection! {
        (0, 0) => "Weight:",
        (1, 0) => SpinBox::new_msg(
            0.0..=1.0,
            move |_, flame: &Flame| flame.functions[index].weight,
            |v| v
        ).with_step(0.1)
        .with_width_em(1.0, 1.5)
        .on_message_update_flame(move |_, _, flame, weight: f32| flame.functions[index].weight = weight),
        (0..=1, 1) => variation,
        (4, 0) => "Color:",
        (5, 0) => SpinBox::new_msg(
            0.0..=1.0,
            move |_, flame: &Flame| flame.functions[index].color,
            |v| v
        ).with_step(0.1)
        .with_width_em(1.0, 1.5)
        .on_message_update_flame(move |_, _, flame, val: f32| flame.functions[index].color = val),
        (4, 1) => "Speed:",
        (5, 1) => SpinBox::new_msg(
            0.0..=1.0,
            move |_, flame: &Flame| flame.functions[index].color_speed,
            |v| v
        ).with_step(0.1)
        .with_width_em(1.0, 1.5)
        .on_message_update_flame(move |_, _, flame, val: f32| flame.functions[index].color_speed = val),
        (2, 0..=1) => Frame::new(affine_pre),
        (3, 0..=1) => Frame::new(affine_post),
        (6, 0..=1) => MarkButton::new_msg(MarkStyle::X, "delete", ListRemove(index)).map_any(),
    }))
}

fn function_list() -> impl Widget<Data = Flame> {
    ScrollRegion::new_clip(column![
        Button::label_msg("Add Function", ListAdd)
            .map_any()
            .on_message_update_flame(move |_, _, flame, ListAdd| {
                flame.functions.push(FunctionEntry::default());
            }),
        Column::new(vec![])
            .on_update(|cx, widget, flame: &Flame| {
                if flame.functions.len() != widget.len() {
                    widget.clear();
                    widget.resize_with(cx, flame, flame.functions.len(), function_entry);
                }
            })
            .on_message_update_flame(move |_, widget, flame, ListRemove(i)| {
                widget.clear();
                flame.functions.remove(i);
            }),
        Filler::maximize().map_any()
    ])
}

fn color_entry(index: usize) -> impl Widget<Data = Flame> {
    let top = row![
        "Key:",
        EditBox::parser(
            move |flame: &Flame| flame.palette.get(index).unwrap().1,
            |val| val
        )
        .on_message_update_flame(move |_, _, flame, val: f32| {
            if let Some(key) = flame.palette.get_mut(index).1 {
                *key = val;
            }
        }),
        Filler::maximize().map_any(),
        MarkButton::new_msg(MarkStyle::X, "delete", ListRemove(index)).map_any()
    ];

    let bottom = row![
        "R:",
        EditBox::parser(
            move |flame: &Flame| flame.palette.get(index).unwrap().0.red,
            |val| val
        )
        .with_width_em(3., 3.)
        .on_message_update_flame(move |_, _, flame, val: u8| {
            flame.palette.get_mut(index).0.unwrap().red = val;
        }),
        "G:",
        EditBox::parser(
            move |flame: &Flame| flame.palette.get(index).unwrap().0.green,
            |val| val
        )
        .with_width_em(3., 3.)
        .on_message_update_flame(move |_, _, flame, val: u8| {
            flame.palette.get_mut(index).0.unwrap().green = val;
        }),
        "B:",
        EditBox::parser(
            move |flame: &Flame| flame.palette.get(index).unwrap().0.blue,
            |val| val
        )
        .with_width_em(3., 3.)
        .on_message_update_flame(move |_, _, flame, val: u8| {
            flame.palette.get_mut(index).0.unwrap().blue = val;
        }),
    ];

    Frame::new(column![top, bottom])
}

fn color_list() -> impl Widget<Data = Flame> {
    ScrollRegion::new_clip(column![
        Button::label_msg("Add Color", ListAdd)
            .map_any()
            .on_message_update_flame(move |_, _, flame, ListAdd| {
                flame.palette.add(Color::default());
            }),
        Column::new(vec![])
            .on_update(|cx, widget, flame: &Flame| {
                if flame.palette.len() != widget.len() {
                    widget.clear();
                    widget.resize_with(cx, flame, flame.palette.len(), color_entry);
                }
            })
            .on_message_update_flame(move |_, widget, flame, ListRemove(i)| {
                widget.clear();
                if flame.palette.len() > 2 {
                    flame.palette.remove(i);
                }
            }),
        Filler::maximize().map_any()
    ])
}

fn file_bar() -> impl Widget<Data = ()> {
    row![
        Filler::maximize(),
        Button::label_msg("Save", ()).on_messages(|cx, widget, _| {
            if let Some(()) = cx.try_pop() {
                cx.send_spawn(widget.id(), async {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("Flame File", &["json", "yaml"])
                        .save_file()
                        .await;
                    SaveFileTo(file)
                });
            }
        }),
        Button::label_msg("Load", ()).on_messages(|cx, widget, _| {
            if let Some(()) = cx.try_pop() {
                cx.send_spawn(widget.id(), async {
                    let file = rfd::AsyncFileDialog::new()
                        .add_filter("Flame File", &["json", "yaml"])
                        .pick_file()
                        .await;
                    LoadFileFrom(file)
                });
            }
        }),
    ]
}

fn main() -> kas::runner::Result<()> {
    let (config_tx, config_rx) = channel();
    let (flame_tx, flame_rx) = channel();
    let default_flame = Flame {
        functions: vec![FunctionEntry::default(), FunctionEntry::default()],
        last: flame::function::Function::default(),
        symmetry: 1,
        palette: Palette::new::<std::iter::Empty<f32>>(
            vec![Color::rgb(255, 255, 255), Color::rgb(255, 255, 255)],
            None,
        )
        .unwrap(),
        bounds: Bounds::new(-1., 1., -1., 1.),
    };
    flame_tx.send(default_flame.clone()).unwrap();
    config_tx.send(DEFAULT_RENDER_CONFIG).unwrap();

    let app = Runner::new(AppData {
        config: DEFAULT_RENDER_CONFIG,
        flame: default_flame,
        config_tx,
        flame_tx,
    })
    .unwrap();

    let proxy = app.create_proxy();
    thread::spawn(move || generate_flame(flame_rx, config_rx, proxy));

    let config_box = render_config_panel().map(|data: &AppData| &data.config);
    // .with_stretch(None, Some(Stretch::None));

    let misc_box = misc_panel().map(|data: &AppData| &data.flame);

    let image = image_widget();

    let function_editor = function_list()
        .map(|data: &AppData| &data.flame)
        .with_margin_style(MarginStyle::Large);

    let palette_editor = color_list()
        .map(|data: &AppData| &data.flame)
        .with_margin_style(MarginStyle::Large);

    let left_column = column![config_box, misc_box, Separator::new(), palette_editor];

    let root = column![
        Splitter::right(collection![left_column, image, function_editor]),
        file_bar().map_any()
    ];

    app.with(Window::new(root, "Flame Editor")).run()
}
