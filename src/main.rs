use anyhow::Result;
use clap::Parser;

use winusb::cli::Cli;

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    if let Err(err) = Cli::parse().run() {
        eprintln!("app error: {:?}", err);
    }

    Ok(())
}
