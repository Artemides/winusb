use std::path::Path;

use tempfile::Builder;

use crate::{
    error::{Error, Result},
    executor::{
        copy::{CopyReport, copy_payload_tree, validate_payload_tree},
        execute_commands, format_target_commands,
        mount::{mount_commands, sync_commands, unmount_commands},
        partition_path,
        requirements::check_write_tools,
    },
    plan::write::WritePlan,
};

#[derive(Debug)]
pub struct WriteReport {
    pub copied: CopyReport,
}

pub fn write_prepared_payload(
    write_plan: &WritePlan,
    payload_dir: &Path,
    mount_base: &Path,
) -> Result<WriteReport> {
    validate_payload_tree(payload_dir)?;

    if !mount_base.is_dir() {
        return Err(Error::PayloadInvalid(format!(
            "mount base is not a directory: {}",
            mount_base.display()
        )));
    }

    check_write_tools()?;

    crate::device::ensure_unmounted(&write_plan.target)?;

    let format_commands =
        format_target_commands(&write_plan.media.target_layout, &write_plan.target)?;
    execute_commands(&format_commands)?;

    crate::device::ensure_unmounted(&write_plan.target)?;

    let partition = partition_path(&write_plan.target, 1);

    let mount_dir = Builder::new()
        .prefix("winusb-mount-")
        .tempdir_in(mount_base)
        .map_err(Error::MountIo)?;

    execute_commands(&mount_commands(&partition, mount_dir.path()))?;

    let write_result: Result<WriteReport> = (|| {
        let copied = copy_payload_tree(payload_dir, mount_dir.path())?;

        execute_commands(&sync_commands())?;

        Ok(WriteReport { copied })
    })();

    let unmount_result = execute_commands(&unmount_commands(mount_dir.path()));

    match (write_result, unmount_result) {
        (Ok(report), Ok(())) => Ok(report),
        (Err(operation), Ok(())) => Err(operation),
        (Ok(_), Err(cleanup)) => Err(cleanup),
        (Err(operation), Err(cleanup)) => Err(Error::WriteCleanupFailed {
            operation: operation.to_string(),
            cleanup: cleanup.to_string(),
        }),
    }
}
