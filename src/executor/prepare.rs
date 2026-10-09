use std::path::{Path, PathBuf};

use crate::{
    error::Result,
    executor::{
        StageReport, setup,
        split::{self, SplitReport},
        stage_iso_tree,
    },
    plan::{MediaOperation, MediaPlan},
};

#[derive(Debug)]
pub struct PrepareReport {
    pub stage: StageReport,
    pub split: Option<SplitReport>,
    pub customization: SetupCustomization,
    pub customization_path: Option<PathBuf>,
}

#[derive(Debug)]
pub enum SetupCustomization {
    Standard,
    BypassChecks,
}

pub fn prepare_payload(
    iso_path: &Path,
    plan: &MediaPlan,
    dest: &Path,
    temp_dir: &Path,
    customization: SetupCustomization,
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

    let customization_path = setup::setup_customization(dest, &customization)?;

    Ok(PrepareReport {
        stage,
        split,
        customization,
        customization_path,
    })
}
