use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{device, plan::media_plan};

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

impl Cli {
    pub fn run(self) -> crate::error::Result<()> {
        match self.command {
            Command::Inspect { iso } => {
                let info = crate::iso::inspect(&iso)?;

                println!("windows info media:\n {:?}", info);
            }

            Command::Devices => {
                for device in crate::device::list_block_devices()? {
                    println!("{device:#?}");
                }
            }

            Command::Plan { iso, device } => {
                let media = crate::iso::inspect(&iso)?;
                let media_plan = crate::plan::media_plan(&media);

                let target = crate::device::find_block_device(&device)?;

                crate::plan::validate_target(&media_plan, &target)?;

                println!("\nMedia plan:");
                println!("{media_plan:#?}");

                println!("\nRequested target: {}", device.display());

                println!("plan ok");
            }

            Command::Write { iso, device } => {
                println!("writing...:\niso: {:?}\nusb:{:?}", iso, device)
            }

            Command::Verift { device } => println!("verifying device {:?}", device),
        }

        Ok(())
    }
}
