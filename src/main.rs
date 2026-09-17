use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;

mod cli;

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let cli = Cli::parse();

    match cli.command {
        cli::Command::Inspect { iso } => {
            println!("Inspecting ISO: {:?}", iso);
        }
        cli::Command::Devices => println!("listing Devices"),
        cli::Command::Plan { iso, device } => {
            println!("planning:\niso: {:?}\nusb:{:?}", iso, device)
        }
        cli::Command::Write { iso, device } => {
            println!("writing...:\niso: {:?}\nusb:{:?}", iso, device)
        }
        cli::Command::Verift { device } => println!("verifying device {:?}", device),
    }

    Ok(())
}
