pub mod variation;
pub mod buffer;
pub mod color;
pub mod error;
pub mod render;
pub mod function;
pub mod bounds;
pub mod bucket;
mod flame;
pub mod random;

pub use flame::*;
pub use render::RenderConfig;
pub use error::FlameError;
