use std::path::Path;
use nalgebra::{Matrix3, Transform};
use serde::Deserialize;

use super::core::*;
use super::error::*;

#[derive(Deserialize)]
#[serde(rename="Flame")]
pub struct FlameSource {
    bounds: [f32; 4],
    #[serde(rename="final", default)]
    last: FunctionSource,
    functions: Vec<FunctionEntrySource>,
    palette: PaletteSource,
}

impl FlameSource {
    pub fn from_json(src: &str) -> serde_json::Result<FlameSource> {
        serde_json::from_str(src)
    }

    pub fn from_ron(src: &str) -> ron::error::SpannedResult<FlameSource> {
        ron::from_str(src)
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<FlameSource, FlameError> {
        let contents = std::fs::read_to_string(path.as_ref())?;
        Ok(match path.as_ref().extension().ok_or(FlameError::ExtensionError)?.to_str() {
            Some("json") => FlameSource::from_json(&contents)?,
            Some("ron") => FlameSource::from_ron(&contents)?,
            _ => return Err(FlameError::ExtensionError)
        })
    }

    pub fn to_flame(self) -> Flame {
        let funcs = self.functions.iter()
            .map(FunctionEntrySource::to_function_entry)
            .collect();

        Flame {
            bounds: Bounds::new(
                self.bounds[0],
                self.bounds[1],
                self.bounds[2],
                self.bounds[3],
            ),
            last: self.last.to_function(),
            functions: funcs,
            palette: self.palette.to_palette(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename="Function")]
struct FunctionSource(Variation, [f32; 6]);

impl FunctionSource {
    fn to_function(&self) -> Function {
        let t = Transform::from_matrix_unchecked(Matrix3::new(
            self.1[0], self.1[1], self.1[4],
            self.1[2], self.1[3], self.1[5],
            0.0,       0.0,       1.0,
        ));

        Function {
            var: self.0,
            trans: t,
        }
    }
}

impl Default for FunctionSource {
    fn default() -> Self {
        FunctionSource(Variation::Id, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    }
}

#[derive(Deserialize)]
#[serde(rename="FunctionEntry")]
struct FunctionEntrySource(f32, Variation, [f32; 6], f32);

impl FunctionEntrySource {
    fn to_function_entry(&self) -> FunctionEntry {
        FunctionEntry {
            weight: self.0,
            color: (self.3 * 256.0) as u8,
            function: FunctionSource(self.1, self.2).to_function()
        }
    }
}

#[derive(Deserialize)]
#[serde(transparent, rename="Palette")]
struct PaletteSource(Vec<ColorSource>);

impl PaletteSource {
    fn to_palette(&self) -> Palette {
        if self.0.len() > 256 { panic!("too many colors in palette description"); }

        let spacing = 256 / (self.0.len() - 1);
        let leftover = 256 % (self.0.len() - 1);

        let mut p_colors = [Color::rgb(0, 0, 0); 256];

        let mut colors = self.0.iter().map(ColorSource::to_color);
        let mut start_color = colors.next().unwrap();
        let mut offset = 0;

        for (i, end_color) in colors.enumerate() {
            let span = if i < leftover { spacing + 1 } else { spacing };
            for j in 0 .. span {
                let t = j as f32 / (span - 1) as f32;
                let c = Color::rgb(
                    lerp(start_color.red, end_color.red, t),
                    lerp(start_color.green, end_color.green, t),
                    lerp(start_color.blue, end_color.blue, t)
                );
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
        Color::rgb(self.0, self.1, self.2)
    }
}
