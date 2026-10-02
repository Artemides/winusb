use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("failed to access ISO: {0}")]
    IsoIo(#[source] io::Error),
    #[error("Invalid ISO image: {0}")]
    IsoInvalid(String),
    #[error("failed to inspect block device: {0}")]
    DeviceIo(#[source] io::Error),
    #[error("invalid block device: {0}")]
    DeviceInvalid(String),
}

pub type Result<T> = std::result::Result<T, Error>;
