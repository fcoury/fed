use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThemeParseError {
    #[error("Missing field: {0}")]
    MissingField(String),
}
