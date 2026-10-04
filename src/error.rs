use std::{io, path::PathBuf};
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

    #[error("target device is not removable: {path}")]
    TargetNotRemovable { path: PathBuf },

    #[error(
        "target device is too small: required {required_bytes} bytes, has {actual_bytes} bytes"
    )]
    TargetTooSmall {
        required_bytes: u64,
        actual_bytes: u64,
    },

    #[error("invalid media plan: {0}")]
    InvalidPlan(String),

    #[error("target device or one of its partitions is mounted: {path}")]
    TargetMounted { path: PathBuf },

    #[error("failed to read write confirmation: {0}")]
    ConfirmationIo(#[source] io::Error),

    #[error("operation cancelled")]
    Cancelled,

    #[error("failed to stage  ISO: files: {0}")]
    StageIo(#[source] io::Error),

    #[error("invalid staging request: {0}")]
    StageInvalid(String),

    #[error("failed to check wimlib-checkio: {0}")]
    WimlibIo(#[source] io::Error),

    #[error("sudo dnf install wimlib required")]
    WimlibMissing,

    #[error("wimlib did not run successfully: {0}")]
    WimlibUnavailable(std::process::ExitStatus),

    #[error("wimlib split failed: {0}")]
    WimlibSplitFailed(std::process::ExitStatus),

    #[error("invalid split WIM output: {0}")]
    WimlibOutputInvalid(String),

    #[error("required program not found: {program}; install with: {install_hint}")]
    RequiredToolMissing {
        program: &'static str,
        install_hint: &'static str,
    },

    #[error("failed to start command `{command}`: {source}")]
    CommandIo {
        command: String,
        #[source]
        source: io::Error,
    },

    #[error("command failed: `{command}` with status {status}")]
    CommandFailed {
        command: String,
        status: std::process::ExitStatus,
    },

    #[error("failed to copy payload: {0}")]
    PayloadIo(#[source] io::Error),

    #[error("invalid payload: {0}")]
    PayloadInvalid(String),

    #[error("failed to create or manage mount point: {0}")]
    MountIo(#[source] io::Error),

    #[error("write failed: {operation}; cleanup also failed: {cleanup}")]
    WriteCleanupFailed { operation: String, cleanup: String },
}

pub type Result<T> = std::result::Result<T, Error>;
