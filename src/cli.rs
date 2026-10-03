use clap::{Parser, Subcommand};
use std::{
    io::{self, Write},
    path::PathBuf,
};

use crate::{
    device::BlockDevice,
    error::{Error, Result},
};

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
                crate::device::ensure_unmounted(&target)?;

                println!("\nMedia plan:");
                println!("{media_plan:#?}");

                println!("\nRequested target: {}", device.display());

                println!("plan ok");
            }

            Command::Write { iso, device } => {
                let media = crate::iso::inspect(&iso)?;
                let media_plan = crate::plan::media_plan(&media);
                let target = crate::device::find_block_device(&device)?;

                crate::plan::validate_target(&media_plan, &target)?;
                crate::device::ensure_unmounted(&target)?;

                confirm_target(&target)?;

                let commands =
                    crate::executor::format_target_commands(&media_plan.target_layout, &target)?;

                println!("\nPlanned formatting commands:");

                for command in commands {
                    println!("  {}", command.display());
                }
            }

            Command::Verift { device } => println!("verifying device {:?}", device),
        }

        Ok(())
    }
}

fn confirm_target(target: &BlockDevice) -> Result<()> {
    println!(
        "\n WARNING: all data on {} will be destroyed",
        target.path.display()
    );

    println!("Model: {}", target.model.as_deref().unwrap_or("unknown"));
    println!("Sixe: {} bytes", target.size_bytes);

    print!(
        "Type the kernel device name '{}' to continue: ",
        target.kernel_name
    );

    io::stdout().flush().map_err(Error::ConfirmationIo)?;

    let mut answer = String::new();

    io::stdin()
        .read_line(&mut answer)
        .map_err(Error::ConfirmationIo)?;

    if answer.trim() != target.kernel_name {
        return Err(Error::Cancelled);
    }

    Ok(())
}
