use nalgebra::{Affine2, Point2, Transform, Matrix3};
use rand::Rng;
use serde::{Serialize, Deserialize};

use super::{
    variation::*,
    error::*
};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from="self::_serde::FunctionEntrySource", into="self::_serde::FunctionEntrySource")]
pub struct FunctionEntry {
    pub function: Function,
    pub weight: f32,
    pub color: f32,
    pub color_speed: f32,
}

impl FunctionEntry {
    pub fn new(
        function: Function,
        weight: f32, color: f32, color_speed: f32
    ) -> Result<FunctionEntry, FunctionEntryError> {
        if color > 1.0 || color < 0.0 {
            return Err(FunctionEntryError::Color)
        }

        if color_speed > 1.0 || color_speed < 0.0 {
            return Err(FunctionEntryError::ColorSpeed)
        }

        Ok(FunctionEntry {
            weight: weight,
            color: color,
            color_speed: color_speed,
            function: function
        })
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(from="self::_serde::FunctionSource", into="self::_serde::FunctionSource")]
pub struct Function {
    pub variation: Variation,
    pub affine_pre: Affine2<f32>,
    pub affine_post: Affine2<f32>
}

impl Function {
    pub fn from_raw(variation: Variation, affine_pre: [f32; 6], affine_post: [f32; 6]) -> Self {
        Function {
            variation,
            affine_pre: affine_from_raw(affine_pre),
            affine_post: affine_from_raw(affine_post)
        }
    }

    pub fn eval(&self, rng: &mut impl Rng, arg: Point2<f32>) -> Point2<f32> {
        self.affine_post * self.variation.eval(rng, self.affine_pre * arg)
    }
}

fn affine_from_raw(raw: [f32; 6]) -> Affine2<f32> {
    Transform::from_matrix_unchecked(Matrix3::new(
        raw[0], raw[1], raw[4],
        raw[2], raw[3], raw[5],
        0.0,       0.0,       1.0,
    ))
}

fn affine_to_raw(affine: Affine2<f32>) -> [f32; 6] {
    let mat = affine.matrix();
    [
        mat.m11, mat.m12, mat.m21, mat.m22,
        mat.m13, mat.m23
    ]
}

mod _serde {
    use super::*;

    const fn default_affine() -> [f32; 6] {
        [1., 0., 0., 1., 0., 0.]
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename="Function")]
    pub struct FunctionSource {
        #[serde(default)]
        variation: Variation,
        #[serde(default="default_affine")]
        affine_pre: [f32; 6],
        #[serde(default="default_affine")]
        affine_post: [f32; 6],
    }

    impl From<FunctionSource> for Function {
        fn from(src: FunctionSource) -> Function {
            Function::from_raw(src.variation, src.affine_pre, src.affine_post)
        }
    }

    impl From<Function> for FunctionSource {
        fn from(func: Function) -> Self {
            FunctionSource {
                variation: func.variation,
                affine_pre: super::affine_to_raw(func.affine_pre),
                affine_post: super::affine_to_raw(func.affine_post),
            }
        }
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename="FunctionEntry")]
    pub struct FunctionEntrySource {
        weight: f32,
        #[serde(flatten)]
        function: Function,
        color: f32,
        color_speed: Option<f32>
    }

    impl TryFrom<FunctionEntrySource> for FunctionEntry {
        type Error = FunctionEntryError;

        fn try_from(src: FunctionEntrySource) -> Result<Self, Self::Error> {
            FunctionEntry::new(
                src.function.into(),
                src.weight,
                src.color,
                src.color_speed.unwrap_or(0.5)
            )
        }
    }

    impl From<FunctionEntry> for FunctionEntrySource {
        fn from(entry: FunctionEntry) -> Self {
            FunctionEntrySource {
                weight: entry.weight,
                function: entry.function,
                color: entry.color,
                color_speed: Some(entry.color_speed)
            }
        }
    }
}
