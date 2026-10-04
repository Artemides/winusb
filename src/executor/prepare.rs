use std::path::Path;

use crate::{
    error::Result,
    executor::{
        StageReport,
        split::{self, SplitReport},
        stage_iso_tree,
    },
    plan::{MediaOperation, MediaPlan},
};

#[derive(Debug)]
pub struct PrepareReport {
    pub stage: StageReport,
    pub split: Option<SplitReport>,
}

pub fn prepare_payload(
    iso_path: &Path,
    plan: &MediaPlan,
    dest: &Path,
    temp_dir: &Path,
) -> Result<PrepareReport> {
    let stage = stage_iso_tree(iso_path, plan, dest)?;

    let needs_split = plan
        .operations
        .iter()
        .any(|op| matches!(op, MediaOperation::SplitWim { .. }));

    let split = if needs_split {
        Some(split::split_install_wim(iso_path, plan, dest, temp_dir)?)
    } else {
        None
    };

    Ok(PrepareReport { stage, split })
}
