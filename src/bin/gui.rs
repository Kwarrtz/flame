use flame::{Buffer, Flame, RenderConfig};
use ribir::prelude::*;
use async_channel::{self, Receiver, Sender};
use std::{borrow::Cow, thread};

const IMG_WIDTH: usize = 1_000;
const IMG_HEIGHT: usize = 1_000;

fn generate_flame(rx_flame: Receiver<Flame>, tx_image: Sender<PixelImage>) {
    let mut buffer: Buffer<u32> = Buffer::new(IMG_WIDTH, IMG_HEIGHT);
    let mut rng = rand::rng();
    let mut flame = rx_flame.recv_blocking().unwrap();

    loop {
        flame.run_partial(&mut buffer, 1_000_000, &mut rng);

        let img_buffer: Buffer<u8> = buffer.render(RenderConfig {
            gamma: 1.0,
            vibrancy: 0.0,
            width: IMG_WIDTH,
            height: IMG_HEIGHT
        });
        let mut raw_image = vec![255u8; IMG_WIDTH * IMG_HEIGHT * 4];
        for (i, &bucket) in img_buffer.buckets.iter().enumerate() {
            raw_image[4 * i] = bucket.red;
            raw_image[4 * i + 1] = bucket.green;
            raw_image[4 * i + 2] = bucket.blue;
        }
        let ribir_image = PixelImage::new(
            Cow::from(raw_image),
            IMG_WIDTH as u32, IMG_HEIGHT as u32,
            image::ColorFormat::Rgba8
        );
        tx_image.force_send(ribir_image).unwrap();

        if let Ok(new_flame) = rx_flame.try_recv() {
            flame = new_flame;
            buffer = Buffer::new(IMG_WIDTH, IMG_HEIGHT);
        }
    }
}

// fn generate_flame(rx_flame: Receiver<Flame>, tx_image: Sender<PixelImage>) {
//     let mut buffer: Buffer<u32> = Buffer::new(IMG_WIDTH, IMG_HEIGHT);
//     let mut rng = rand::rng();
//     let mut flame = rx_flame.recv_blocking().unwrap();
//     loop {
//         flame.run_partial(&mut buffer, 5_000, &mut rng);
//         let raw_image = buffer.buckets.iter()
//             .flat_map(|bucket| {
//                 std::iter::once(255u8).bucket.iter_rgb()
//             })
//         let ribir_image = PixelImage::new(
//             Cow::from(rgba_image.into_raw()),
//             IMG_WIDTH as u32, IMG_HEIGHT as u32,
//             image::ColorFormat::Rgba8
//         );
//         tx_image.force_send(ribir_image).unwrap();

//         if let Ok(new_flame) = rx_flame.try_recv() {
//             flame = new_flame;
//             buffer = Buffer::new(IMG_WIDTH, IMG_HEIGHT);
//         }
//     }
// }

fn main() {
    let (tx_flame, rx_flame) = async_channel::bounded::<Flame>(1);
    let (tx_image, rx_image) = async_channel::bounded::<PixelImage>(1);
    thread::spawn(move || generate_flame(rx_flame, tx_image));

    App::run(fn_widget! {
        let flame: State<Option<Flame>> = State::value(None);

        watch!($flame;)
            // .debounce(Duration::from_secs(1), AppCtx::scheduler())
            .subscribe(move |_| {
                if let Some(flame_) = $flame.clone() {
                    tx_flame.force_send(flame_).unwrap();
                }
            });

        let image = FatObj::new(State::value(Resource::new(PixelImage::new(
            Cow::from(&[0u8; 500*500*4]),
            500, 500,
            image::ColorFormat::Rgba8
        ))));

        observable::from_stream(rx_image, AppCtx::scheduler())
            .subscribe(move |new_image| {
                *$image.write() = Resource::new(new_image);
            });

        let flame_input = @TextArea { rows: 1000. };
        @Flex {
            direction: Direction::Horizontal,
            align_items: Align::Center,
            background: Brush::Color(Color::WHITE),
            @Expanded {
                // background: Brush::Color(Color::BLACK),
                @ $image {
                    box_fit: BoxFit::CoverX
                }
            }
            @Expanded {
                @ $flame_input {
                    border: Border::all(BorderSide::new(1.0, Brush::Color(Color::BLACK))),
                    border_radius: Radius::all(3.0),
                    on_chars: move |_| {
                        *$flame.write() = Flame::from_yaml(
                            &$flame_input.text()
                        ).ok();
                    }
                }
            }
        }
    });
}
