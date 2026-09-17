use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "winusb",
    version,
    about = "create windows bootable meadia on linux"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// inspect a windows iso
    Inspect { iso: PathBuf },
    /// list available block devices
    Devices,
    /// operations to perform
    Plan { iso: PathBuf, device: PathBuf },
    /// wirte windows installation media
    Write { iso: PathBuf, device: PathBuf },
    /// verify usb installed media
    Verift { device: PathBuf },
}
