pub mod prepare;
pub mod split;

use std::{
    fs::{self, OpenOptions},
    io,
    path::{Component, Path, PathBuf},
    process::Command,
};

use crate::{
    device::BlockDevice,
    error::{
        Error::{self},
        Result,
    },
    plan::{
        MediaOperation, MediaPlan, PartitionRole, PartitionSize, PartitionTableKind,
        TargetFilesystem, TargetLayout,
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub program: &'static str,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn display(&self) -> String {
        std::iter::once(self.program.to_owned())
            .chain(self.args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

pub fn format_target_commands(
    layout: &TargetLayout,
    target: &BlockDevice,
) -> Result<Vec<CommandSpec>> {
    if layout.partition_table != PartitionTableKind::Gpt {
        return Err(Error::InvalidPlan(String::from(
            "only GPT target layouts are supported",
        )));
    }

    let [partition] = layout.partitions.as_slice() else {
        return Err(Error::InvalidPlan(String::from(
            "expected exactly one target partition",
        )));
    };

    if partition.number != 1
        || partition.role != PartitionRole::EfiSystem
        || partition.filesystem != TargetFilesystem::Fat32
        || partition.size != PartitionSize::RemainingDeviceSpace
    {
        return Err(Error::InvalidPlan(String::from(
            "unsupported target partition layout",
        )));
    }

    let target_path = target.path.display().to_string();
    let partition_path = partition_path(target, partition.number);

    Ok(vec![
        CommandSpec {
            program: "parted",
            args: vec![
                "--script".into(),
                "--align".into(),
                "optimal".into(),
                target_path,
                "mklabel".into(),
                "gpt".into(),
                "mkpart".into(),
                "WINUSB".into(),
                "fat32".into(),
                "1MiB".into(),
                "100%".into(),
                "set".into(),
                "1".into(),
                "esp".into(),
                "on".into(),
            ],
        },
        CommandSpec {
            program: "udevadm",
            args: vec!["settle".into()],
        },
        CommandSpec {
            program: "mkfs.fat",
            args: vec![
                "-F".into(),
                "32".into(),
                "-n".into(),
                "WINUSB".into(),
                partition_path.display().to_string(),
            ],
        },
    ])
}

fn partition_path(target: &BlockDevice, partition_number: u32) -> PathBuf {
    let name_ends_in_digit = target
        .kernel_name
        .as_bytes()
        .last()
        .is_some_and(u8::is_ascii_digit);

    let suffix = if name_ends_in_digit {
        format!("p{partition_number}")
    } else {
        partition_number.to_string()
    };

    PathBuf::from(format!("{}{}", target.path.display(), suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str) -> BlockDevice {
        BlockDevice {
            kernel_name: name.into(),
            path: PathBuf::from(format!("/dev/{name}")),
            size_bytes: 16 * 1024 * 1024 * 1024,
            removable: true,
            model: None,
            serial: None,
        }
    }

    #[test]
    fn finds_standard_partition_path() {
        assert_eq!(
            partition_path(&device("sdb"), 1),
            PathBuf::from("/dev/sdb1")
        );
    }

    #[test]
    fn finds_nvme_partition_path() {
        assert_eq!(
            partition_path(&device("nvme0n1"), 1),
            PathBuf::from("/dev/nvme0n1p1")
        );
    }
}

/// STAGING

#[derive(Debug, Default)]
pub struct StageReport {
    pub copied_files: u64,
    pub copied_bytes: u64,
    pub skipped_files: u64,
}

pub fn stage_iso_tree(iso_path: &Path, plan: &MediaPlan, dest: &Path) -> Result<StageReport> {
    prepare_empty_destination(dest)?;

    let e_paths = plan
        .operations
        .iter()
        .find_map(|op| match op {
            MediaOperation::CopyIsoTree { excluding } => Some(excluding),
            _ => None,
        })
        .ok_or_else(|| Error::InvalidPlan(String::from("plan has no ISO copy operation")))?;

    let mut report = StageReport::default();

    for source in crate::iso::source_files(iso_path)? {
        if e_paths.iter().any(|path| path == &source.path) {
            report.skipped_files += 1;

            continue;
        }

        let out_path = staging_path(dest, &source.path)?;

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(Error::StageIo)?;
        }

        let mut input = source.open(iso_path)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&out_path)
            .map_err(Error::StageIo)?;

        let copied = io::copy(&mut input, &mut output).map_err(Error::StageIo)?;

        if copied != source.size {
            return Err(Error::StageInvalid(format!(
                "copied size mismatch for {}",
                source.path.display()
            )));
        }

        report.copied_files += 1;
        report.copied_bytes += copied;
    }

    Ok(report)
}

fn prepare_empty_destination(dest: &Path) -> Result<()> {
    if dest.exists() {
        let mut entries = fs::read_dir(dest).map_err(Error::StageIo)?;

        if entries.next().is_some() {
            return Err(Error::StageInvalid(format!(
                "destination is not empty: {}",
                dest.display()
            )));
        };

        return Ok(());
    }

    fs::create_dir_all(dest).map_err(Error::StageIo)
}

fn staging_path(dest: &Path, relative_path: &Path) -> Result<PathBuf> {
    let mut output = dest.to_path_buf();

    for cmp in relative_path.components() {
        match cmp {
            Component::Normal(part) => output.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(Error::StageInvalid(format!(
                    "unsafe ISO path: {}",
                    relative_path.display()
                )));
            }
        }
    }

    Ok(output)
}

pub fn check_wimlib() -> Result<()> {
    let status = Command::new("wimlib-imagex")
        .arg("--version")
        .status()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                Error::WimlibMissing
            } else {
                Error::WimlibIo(error)
            }
        })?;

    if !status.success() {
        return Err(Error::WimlibUnavailable(status));
    }

    Ok(())
}
