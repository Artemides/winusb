use std::{io, process::Command};

use crate::error::{Error, Result};

pub fn check_write_tools() -> Result<()> {
    let requirements = [
        ("parted", "sudo dnf install parted"),
        ("mkfs.fat", "sudo dnf install dosfstools"),
        ("mount", "install util-linux"),
        ("umount", "install util-linux"),
        ("udevadm", "install systemd"),
    ];

    for (program, install_hint) in requirements {
        Command::new(program)
            .arg("--version")
            .output()
            .map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    Error::RequiredToolMissing {
                        program,
                        install_hint,
                    }
                } else {
                    Error::WimlibIo(error)
                }
            })?;
    }

    Ok(())
}
