use thiserror::Error;

#[derive(Error, Debug)]
pub enum FunctionEntryError {
    #[error("color speed must be between 0 and 1")]
    ColorSpeed,
    #[error("color must be between 0 and 1")]
    Color
}

#[derive(Error, Debug)]
pub enum PaletteError {
    #[error("at least one key out of bounds (must be strictly between 0 and 1)")]
    OutOfBounds,
    #[error("keys not strictly monotonically increasing")]
    NonMonotonic,
    #[error("incorrect number of keys")]
    IncorrectNumber
}

#[derive(Error, Debug)]
pub enum FlameError {
    #[error("could not parse flame file\n{0}")]
    JsonError(#[from] serde_json::Error),
    #[error("could not parse flame file\n{0}")]
    RonError(#[from] ron::error::SpannedError),
    #[error("failed to read input file\n{0}")]
    FileReadError(#[from] std::io::Error),
    #[error("input file does not have valid extension (must be .json or .ron)")]
    ExtensionError,
    #[error("failed to save image\n{0}")]
    ImageSaveError(#[from] image::ImageError),
    #[error("invalid color palette keys, {0}")]
    PaletteError(#[from] PaletteError),
    #[error("invalid function specification, {0}")]
    FunctionEntryError(#[from] FunctionEntryError)
}
