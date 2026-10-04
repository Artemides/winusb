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
    Write {
        iso: PathBuf,
        payload: PathBuf,
        device: PathBuf,
        #[arg(long, default_value = "/var/tmp")]
        mount_base: PathBuf,
    },
    /// verify USB-installed Windows media without modifying it
    Verify {
        device: PathBuf,

        #[arg(long, default_value = "/var/tmp")]
        mount_base: PathBuf,
    },
    /// copy iso files  to and ordinary staging dir
    Stage { iso: PathBuf, destination: PathBuf },
    /// requirements
    Requirements,
    /// wim splitting
    SplitStage {
        iso: PathBuf,
        dest: PathBuf,
        temp_dir: PathBuf,
    },
    /// prepare complete Windows files without writing a USB device
    Prepare {
        iso: PathBuf,
        destination: PathBuf,
        temp_dir: PathBuf,
    },
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

                let write_plan = crate::plan::write::build_write_plan(iso, media_plan, target)?;

                crate::device::ensure_unmounted(&write_plan.target)?;

                println!("\nWrite plan:");
                println!("{write_plan:#?}");

                println!("plan ok");
            }

            Command::Write {
                iso,
                device,
                payload,
                mount_base,
            } => {
                let media = crate::iso::inspect(&iso)?;
                let media_plan = crate::plan::media_plan(&media);
                let target = crate::device::find_block_device(&device)?;

                let write_plan = crate::plan::write::build_write_plan(iso, media_plan, target)?;

                crate::executor::copy::validate_payload_tree(&payload)?;
                crate::device::ensure_unmounted(&write_plan.target)?;

                confirm_target(&write_plan.target)?;

                let report = crate::executor::write::write_prepared_payload(
                    &write_plan,
                    &payload,
                    &mount_base,
                )?;

                println!("USB write completed:");
                println!("{report:#?}");
            }

            Command::Verify { device, mount_base } => {
                let target = crate::device::find_block_device(&device)?;

                let report = crate::executor::verify::verify_written_payload(&target, &mount_base)?;

                println!("USB verification completed:");
                println!("{report:#?}");
            }

            Command::Stage { iso, destination } => {
                let media = crate::iso::inspect(&iso)?;
                let plan = crate::plan::media_plan(&media);

                let report = crate::executor::stage_iso_tree(&iso, &plan, &destination)?;

                println!("Staging completed:");
                println!("{report:#?}");
                println!("install.wim was intentionally excluded.");
            }
            Command::Requirements => {
                crate::executor::check_wimlib()?;
                crate::executor::requirements::check_write_tools()?;

                println!("ok")
            }

            Command::SplitStage {
                iso,
                dest,
                temp_dir,
            } => {
                let media = crate::iso::inspect(&iso)?;
                let plan = crate::plan::media_plan(&media);

                let report =
                    crate::executor::split::split_install_wim(&iso, &plan, &dest, &temp_dir)?;

                println!("WIM splitting completed:");
                println!("{report:#?}");
            }

            Command::Prepare {
                iso,
                destination,
                temp_dir,
            } => {
                let media = crate::iso::inspect(&iso)?;
                let plan = crate::plan::media_plan(&media);

                let report = crate::executor::prepare::prepare_payload(
                    &iso,
                    &plan,
                    &destination,
                    &temp_dir,
                )?;

                println!("Payload preparation completed:");
                println!("{report:#?}");
            }
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
