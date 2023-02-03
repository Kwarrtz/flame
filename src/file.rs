use std::{fs::File, path::Path};
use nalgebra::{Matrix3, Transform};
use serde::Deserialize;

use super::core::*;

#[derive(Deserialize)]
pub struct FlameSource {
    bounds: [f32; 4],
    functions: Vec<FunctionSource>,
    palette: PaletteSource,
}

impl FlameSource {
    pub fn from_file(f: File) -> serde_json::Result<FlameSource> {
        serde_json::from_reader(f)
    }

    pub fn to_flame(self) -> Flame {
        let funcs = self.functions.iter()
            .map(FunctionSource::to_function)
            .collect();

        Flame {
            bounds: Bounds {
                x_min: self.bounds[0],
                x_max: self.bounds[1],
                y_min: self.bounds[2],
                y_max: self.bounds[3],
            },
            functions: funcs,
            palette: self.palette.to_palette(),
        }
    }
}

impl std::convert::From<FlameSource> for Flame {
    fn from(fs: FlameSource) -> Flame {
        fs.to_flame()
    }
}

#[derive(Deserialize)]
struct FunctionSource(f32, Variation, [f32; 6], f32);

impl FunctionSource {
    fn to_function(&self) -> Function {
        let t = Transform::from_matrix_unchecked(Matrix3::new(
            self.2[0], self.2[1], self.2[4], 
            self.2[2], self.2[3], self.2[5], 
            0.0,       0.0,       1.0,
        ));

        Function {
            weight: self.0,
            var: self.1,
            trans: t,
            color: self.3,
        }
    }
}

#[derive(Deserialize)]
struct PaletteSource(Vec<ColorSource>);

impl PaletteSource {
    fn to_palette(&self) -> Palette {
        if self.0.len() > 256 { panic!("too many colors in palette description"); }

        let spacing = 256 / (self.0.len() - 1);
        let leftover = 256 % (self.0.len() - 1);

        let mut p_colors = [Color::from_rgb(0, 0, 0); 256];

        let mut colors = self.0.iter().map(ColorSource::to_color);
        let mut start_color = colors.next().unwrap();
        let mut offset = 0;

        for (i, end_color) in colors.enumerate() {
            let span = if i < leftover { spacing + 1 } else { spacing };
            for j in 0 .. span {
                let t = j as f32 / (span - 1) as f32;
                let c = Color {
                    red: lerp(start_color.red, end_color.red, t),
                    green: lerp(start_color.green, end_color.green, t),
                    blue: lerp(start_color.blue, end_color.blue, t),
                };
                p_colors[offset + j] = c;
            }
            offset += span;
            start_color = end_color;
        }

        Palette::new(p_colors)
    }
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 * (1. - t) + b as f32 * t) as u8
}

#[derive(Deserialize)]
struct ColorSource(u8, u8, u8);

impl ColorSource {
    fn to_color(&self) -> Color {
        Color {
            red: self.0,
            green: self.1,
            blue: self.2,
        }
    }
}

pub fn save_buckets(buckets: &Plotter, path: impl AsRef<Path>) -> image::ImageResult<()> {
    image::save_buffer(
        path, 
        &buckets.to_buffer(), 
        buckets.width as u32, buckets.height as u32, 
        image::ColorType::Rgb8
    )
}