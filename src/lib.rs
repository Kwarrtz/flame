pub mod bounds;
pub mod bucket;
pub mod buffer;
pub mod color;
pub mod error;
pub mod evolve;
pub mod function;
pub mod random;
pub mod render;
pub mod variation;

mod flame;
pub use error::Error;
pub use flame::*;
pub use render::{BlurConfig, RenderConfig};
