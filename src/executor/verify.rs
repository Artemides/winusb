use std::path::Path;

use tempfile::Builder;

use crate::{
    device::BlockDevice,
    error::{Error, Result},
    executor::{
        CommandSpec, copy::validate_payload_tree, execute_commands, mount::unmount_commands,
        partition_path,
    },
};

#[derive(Debug)]
pub struct VerifyReport {
    pub partition: std::path::PathBuf,
}

pub fn mount_read_only_commands(partition: &Path, mountpoint: &Path) -> Vec<CommandSpec> {
    vec![CommandSpec {
        program: "mount",
        args: vec![
            "-t".into(),
            "vfat".into(),
            "-o".into(),
            "ro".into(),
            partition.display().to_string(),
            mountpoint.display().to_string(),
        ],
    }]
}

pub fn verify_written_payload(target: &BlockDevice, mount_base: &Path) -> Result<VerifyReport> {
    if !mount_base.is_dir() {
        return Err(Error::PayloadInvalid(format!(
            "mount base is not a directory: {}",
            mount_base.display()
        )));
    }

    crate::device::ensure_unmounted(target)?;

    let partition = partition_path(target, 1);
    let mount_dir = Builder::new()
        .prefix("winusb-verify-")
        .tempdir_in(mount_base)
        .map_err(Error::MountIo)?;

    execute_commands(&mount_read_only_commands(&partition, mount_dir.path()))?;

    let verify_result = validate_payload_tree(mount_dir.path());

    let unmount_result = execute_commands(&unmount_commands(mount_dir.path()));

    match (verify_result, unmount_result) {
        (Ok(()), Ok(())) => Ok(VerifyReport { partition }),
        (Err(operation), Ok(())) => Err(operation),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Err(operation), Err(cleanup)) => Err(Error::WriteCleanupFailed {
            operation: operation.to_string(),
            cleanup: cleanup.to_string(),
        }),
    }
}
