use std::path::Path;
use nalgebra::{Matrix3, Transform};
use serde::Deserialize;

use crate::FunctionEntryError;

use super::{FlameError,Flame,FunctionEntry,Function,Variation,Palette,PaletteError,Color,Bounds};

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
}

impl TryFrom<FlameSource> for Flame {
    type Error = FlameError;

    fn try_from(src: FlameSource) -> Result<Flame, FlameError> {
        let funcs = src.functions.into_iter()
            .map(FunctionEntry::try_from)
            .collect::<Result<_,_>>()?;

        Ok(Flame {
            bounds: Bounds::new(
                src.bounds[0],
                src.bounds[1],
                src.bounds[2],
                src.bounds[3],
            ),
            last: src.last.into(),
            functions: funcs,
            palette: src.palette.try_into()?,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename="Function")]
struct FunctionSource(Variation, [f32; 6]);

impl From<FunctionSource> for Function {
    fn from(src: FunctionSource) -> Function {
        let t = Transform::from_matrix_unchecked(Matrix3::new(
            src.1[0], src.1[1], src.1[4],
            src.1[2], src.1[3], src.1[5],
            0.0,       0.0,       1.0,
        ));

        Function {
            var: src.0,
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

impl TryFrom<FunctionEntrySource> for FunctionEntry {
    type Error = FunctionEntryError;

    fn try_from(src: FunctionEntrySource) -> Result<Self, Self::Error> {
        FunctionEntry::new(FunctionSource(src.1, src.2).into(), src.0, src.3, src.4)
    }
}

#[derive(Deserialize)]
struct PaletteSource {
    colors: Vec<ColorSource>,
    #[serde(default)]
    keys: Vec<f32>
}

impl TryFrom<PaletteSource> for Palette {
    type Error = PaletteError;

    fn try_from(src: PaletteSource) -> Result<Palette, PaletteError> {
        let colors = src.colors.into_iter().map(Color::from);
        let keys = Some(src.keys).filter(|v| !v.is_empty());
        Palette::new(colors, keys)
    }
}

#[derive(Deserialize)]
struct ColorSource(u8, u8, u8);

impl From<ColorSource> for Color {
    fn from(src: ColorSource) -> Color {
        Color::rgb(src.0, src.1, src.2)
    }
}
