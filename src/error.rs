use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to access ISO: {0}")]
    IsoIo(#[source] io::Error),
    #[error("Invalid ISO image: {0}")]
    IsoInvalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
