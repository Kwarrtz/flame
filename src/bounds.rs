use serde::{Serialize,Deserialize};
use nalgebra::{Point2};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x_min: f32,
    pub x_max: f32,
    pub y_min: f32,
    pub y_max: f32,
}

impl Bounds {
    pub fn new(x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> Self {
        Bounds { x_min, x_max, y_min, y_max }
    }

    pub fn contains(&self, p: &Point2<f32>) -> bool {
        let x = p[0];
        let y = p[1];
        x > self.x_min && x < self.x_max && y > self.y_min && y < self.y_max
    }

    pub fn width(&self) -> f32 {
        self.x_max - self.x_min
    }

    pub fn height(&self) -> f32 {
        self.y_max - self.y_min
    }
}

impl Default for Bounds {
    fn default() -> Bounds {
        Bounds {
            x_min: -1., x_max: 1.,
            y_min: -1., y_max: 1.
        }
    }
}
