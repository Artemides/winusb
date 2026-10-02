use std::path::PathBuf;

use crate::iso::{IsoEntry, WindowsMediaInfo};

pub const WIM_SPLIT_PART_SIZE: u64 = 3_800 * 1024 * 1024;

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
pub struct MediaPlan {
    pub target_filesystem: TargetFilesystem,
    pub operations: Vec<MediaOperation>,
}

pub fn media_plan(media: &WindowsMediaInfo) -> MediaPlan {
    MediaPlan {
        target_filesystem: TargetFilesystem::Fat32,
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
