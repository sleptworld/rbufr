use nom;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV Error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Table not found: {0}")]
    TableNotFound(#[from] anyhow::Error),

    #[error("Table not found")]
    TableNotFoundEmpty,

    #[error("Parse Error: {0}")]
    ParseError(String),

    #[error("File is not a valid BUFR file")]
    Nom(String),

    #[error("Unsupported BUFR version: {0}")]
    UnsupportedVersion(u8),
}

impl<'a> From<nom::Err<nom::error::Error<&'a [u8]>>> for Error {
    fn from(value: nom::Err<nom::error::Error<&'a [u8]>>) -> Self {
        Self::Nom(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
