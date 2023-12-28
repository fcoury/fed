use plist::Dictionary;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ThemeParseError {
    #[error("Missing field: {0}")]
    MissingField(String),

    #[error("Entry: {0:?} Missing field: {1}")]
    MissingDictionaryField(Dictionary, String),
}
