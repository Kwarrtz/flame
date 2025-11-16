use thiserror::Error;

#[derive(Error, Debug)]
pub enum FlameError {
    #[error("could not parse JSON flame file\n{0}")]
    JsonError(#[from] serde_json::Error),
    #[error("could not parse RON flame file\n{0}")]
    RonError(#[from] ron::error::SpannedError),
    #[error("failed to read input file\n{0}")]
    FileReadError(#[from] std::io::Error),
    #[error("input file does not have valid extension (must be .json or .ron)")]
    ExtensionError,
    #[error("failed to save image\n{0}")]
    ImageSaveError(#[from] image::ImageError),
}
