use std::path::Path;

use crate::{
    error::Result,
    executor::{
        StageReport,
        boot_wim::{BootWimPatchReport, check_bootwim_tools, patch_staged_boot_wim},
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
    pub boot_wim_patch: Option<BootWimPatchReport>,
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

    if matches!(customization, SetupCustomization::BypassChecks) {
        check_bootwim_tools()?;
    }

    let boot_wim = match customization {
        SetupCustomization::Standard => None,
        SetupCustomization::BypassChecks => Some(patch_staged_boot_wim(dest)?),
    };

    Ok(PrepareReport {
        stage,
        split,
        customization,
        boot_wim_patch: boot_wim,
    })
}
