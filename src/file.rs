use std::{fs::File, path::Path};
use nalgebra::{Matrix3, Transform};
use serde::Deserialize;

use super::core::*;

#[derive(Deserialize)]
pub struct FlameSource {
    bounds: [f32; 4],
    functions: Vec<FunctionSource>,
}

impl FlameSource {
    pub fn from_file(f: File) -> serde_json::Result<FlameSource> {
        serde_json::from_reader(f)
    }

    pub fn to_flame(self) -> Flame {
        let funcs = self
            .functions
            .iter()
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
        }
    }
}

impl std::convert::From<FlameSource> for Flame {
    fn from(fs: FlameSource) -> Flame {
        fs.to_flame()
    }
}

#[derive(Deserialize)]
struct FunctionSource(f32, Variation, [f32; 6]);

impl FunctionSource {
    fn to_function(&self) -> (f32, Function) {
        let t = Transform::from_matrix_unchecked(Matrix3::new(
            self.2[0], self.2[1], self.2[4], 
            self.2[2], self.2[3], self.2[5], 
            0.0,       0.0,       1.0,
        ));

        let f = Function {
            var: self.1,
            trans: t,
        };

        (self.0, f)
    }
}

pub fn save_buckets(buckets: &Plotter, path: impl AsRef<Path>) -> image::ImageResult<()> {
    image::save_buffer(
        path, 
        &buckets.to_buffer(), 
        buckets.width as u32, buckets.height as u32, 
        image::ColorType::L8
    )
}