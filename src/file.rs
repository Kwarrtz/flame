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
    // #[serde(flatten)]
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

    pub fn to_flame(self) -> Result<Flame, FlameError> {
        let funcs = self.functions.iter()
            .map(FunctionEntrySource::to_function_entry)
            .collect();

        Ok(Flame {
            bounds: Bounds::new(
                self.bounds[0],
                self.bounds[1],
                self.bounds[2],
                self.bounds[3],
            ),
            last: self.last.to_function(),
            functions: funcs,
            palette: self.palette.to_palette()?,
        })
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

const fn a_half() -> f32 { 0.5 }

#[derive(Deserialize)]
#[serde(rename="FunctionEntry")]
struct FunctionEntrySource(f32, Variation, [f32; 6], f32, #[serde(default="a_half")] f32);

impl FunctionEntrySource {
    fn to_function_entry(&self) -> FunctionEntry {
        FunctionEntry {
            weight: self.0,
            color: self.3,
            color_speed: self.4,
            function: FunctionSource(self.1, self.2).to_function()
        }
    }
}

#[derive(Deserialize)]
struct PaletteSource {
    colors: Vec<ColorSource>,
    #[serde(default)]
    keys: Vec<f32>
}

impl PaletteSource {
    fn to_palette(self) -> Result<Palette, FlameError> {
        let colors = self.colors.iter().map(ColorSource::to_color);
        let keys = Some(self.keys).filter(|v| !v.is_empty());
        println!("{:?}", keys);
        Palette::new(colors, keys).map_err(FlameError::PaletteError)
    }
}

#[derive(Deserialize)]
struct ColorSource(u8, u8, u8);

impl ColorSource {
    fn to_color(&self) -> Color {
        Color::rgb(self.0, self.1, self.2)
    }
}
