use std::path::PathBuf;

use crate::{
    device::BlockDevice,
    error::Result,
    plan::{MediaPlan, validate_target},
};

#[derive(Debug)]
pub struct WritePlan {
    pub source_iso: PathBuf,
    pub target: BlockDevice,
    pub media: MediaPlan,
}

pub fn build_write_plan(
    source_iso: PathBuf,
    media: MediaPlan,
    target: BlockDevice,
) -> Result<WritePlan> {
    validate_target(&media, &target)?;

    Ok(WritePlan {
        source_iso,
        target,
        media,
    })
}
