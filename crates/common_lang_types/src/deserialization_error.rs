use serde::de;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeserializationError {
    #[error("Error when deserializing.\n\n{0}")]
    Custom(String),
}

impl de::Error for DeserializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        DeserializationError::Custom(msg.to_string())
    }
}
