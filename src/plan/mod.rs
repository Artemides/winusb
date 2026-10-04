pub mod write;

use std::path::PathBuf;

use crate::{
    device::BlockDevice,
    error::{Error, Result},
    iso::{IsoEntry, WindowsMediaInfo},
};

pub const WIM_SPLIT_PART_SIZE: u64 = 3_800 * 1024 * 1024;
pub const TARGET_CAP_MARGIN_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum TargetFilesystem {
    Fat32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum MediaOperation {
    CopyIsoTree {
        excluding: Vec<PathBuf>,
    },
    SplitWim {
        source: PathBuf,
        destination: PathBuf,
        max_part_size: u64,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub enum PartitionTableKind {
    Gpt,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PartitionRole {
    EfiSystem,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PartitionSize {
    RemainingDeviceSpace,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PartitionSpec {
    pub number: u32,
    pub role: PartitionRole,
    pub filesystem: TargetFilesystem,
    pub size: PartitionSize,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TargetLayout {
    pub partition_table: PartitionTableKind,
    pub partitions: Vec<PartitionSpec>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct MediaPlan {
    pub target_layout: TargetLayout,
    pub source_file_count: u64,
    pub source_file_bytes: u64,
    pub operations: Vec<MediaOperation>,
}

pub fn media_plan(media: &WindowsMediaInfo) -> MediaPlan {
    MediaPlan {
        target_layout: TargetLayout::new(),
        source_file_count: media.contents.file_count,
        source_file_bytes: media.contents.total_file_bytes,
        operations: install_image_operations(&media.install_image),
    }
}

fn install_image_operations(install_image: &IsoEntry) -> Vec<MediaOperation> {
    if install_image.size <= WIM_SPLIT_PART_SIZE {
        return vec![MediaOperation::CopyIsoTree {
            excluding: Vec::new(),
        }];
    }

    vec![
        MediaOperation::CopyIsoTree {
            excluding: vec![install_image.path.clone()],
        },
        MediaOperation::SplitWim {
            source: install_image.path.clone(),
            destination: PathBuf::from("sources/install.swm"),
            max_part_size: WIM_SPLIT_PART_SIZE,
        },
    ]
}

pub fn validate_target(plan: &MediaPlan, target: &BlockDevice) -> Result<()> {
    if !target.removable {
        return Err(Error::TargetNotRemovable {
            path: target.path.clone(),
        });
    };

    let required_bytes = plan
        .source_file_bytes
        .checked_add(TARGET_CAP_MARGIN_BYTES)
        .ok_or_else(|| Error::InvalidPlan(String::from("required size overflow")))?;

    if target.size_bytes < required_bytes {
        return Err(Error::TargetTooSmall {
            required_bytes,
            actual_bytes: target.size_bytes,
        });
    };

    Ok(())
}

impl TargetLayout {
    fn new() -> Self {
        Self {
            partition_table: PartitionTableKind::Gpt,
            partitions: vec![PartitionSpec {
                number: 1,
                role: PartitionRole::EfiSystem,
                filesystem: TargetFilesystem::Fat32,
                size: PartitionSize::RemainingDeviceSpace,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_plan(source_file_bytes: u64) -> MediaPlan {
        MediaPlan {
            target_layout: TargetLayout::new(),
            source_file_count: 1,
            source_file_bytes,
            operations: Vec::new(),
        }
    }

    fn test_device(size_bytes: u64, removable: bool) -> BlockDevice {
        BlockDevice {
            kernel_name: String::from("sdb"),
            path: PathBuf::from("/dev/sdb"),
            size_bytes,
            removable,
            model: None,
            serial: None,
        }
    }

    #[test]
    fn rejects_non_removable_devices() {
        let plan = test_plan(1);
        let device = test_device(16 * (2 << 30), false);

        assert!(matches!(
            validate_target(&plan, &device),
            Err(Error::TargetNotRemovable { .. })
        ));
    }

    #[test]
    fn rejects_too_small_devices() {
        let plan = test_plan(8 * (2 << 30));
        let device = test_device(8 * (2 << 30), true);

        assert!(matches!(
            validate_target(&plan, &device),
            Err(Error::TargetTooSmall { .. })
        ));
    }

    #[test]
    fn accept_large_removable_device() {
        let plan = test_plan(8 * 1024 * 1024 * 1024);
        let device = test_device(16 * 1024 * 1024 * 1024, true);

        assert!(validate_target(&plan, &device).is_ok())
    }
}
