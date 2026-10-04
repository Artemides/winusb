use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::Builder;

use crate::{
    error::{Error, Result},
    executor::{check_wimlib, staging_path},
    plan::{MediaOperation, MediaPlan},
};

#[derive(Debug)]
pub struct SplitReport {
    pub source_bytes: u64,
    pub part_files: Vec<PathBuf>,
    pub part_bytes: u64,
}

pub fn split_install_wim(
    iso_path: &Path,
    plan: &MediaPlan,
    staging_dir: &Path,
    temp_dir: &Path,
) -> Result<SplitReport> {
    check_wimlib()?;

    if !temp_dir.is_dir() {
        return Err(Error::StageInvalid(format!(
            "temporary directory does not exist: {}",
            temp_dir.display()
        )));
    }

    let (src_path, dest_path, part_size) = plan
        .operations
        .iter()
        .find_map(|op| match op {
            MediaOperation::SplitWim {
                source,
                destination,
                max_part_size,
            } => Some((source, destination, *max_part_size)),
            _ => None,
        })
        .ok_or_else(|| Error::InvalidPlan(String::from("plan does not require wim splitting")))?;

    let src = crate::iso::source_files(iso_path)?
        .into_iter()
        .find(|file| file.path == *src_path)
        .ok_or_else(|| Error::IsoInvalid(format!("cannot find iso: {:?}", src_path)))?;

    let output = staging_path(staging_dir, dest_path)?;
    let output_parent = output
        .parent()
        .ok_or_else(|| Error::StageInvalid(String::from("split-WIM destination has no parent")))?;

    if !output_parent.is_dir() {
        return Err(Error::StageInvalid(format!(
            "staging directory is incomplete; missing {:?}",
            output_parent
        )));
    }

    if output.exists() {
        return Err(Error::StageInvalid(format!(
            "split-WIM output already exists: {:?}",
            output
        )));
    }

    let mut temp_wim = Builder::new()
        .prefix("winusb-intsll-")
        .suffix(".wim")
        .tempfile_in(temp_dir)
        .map_err(Error::StageIo)?;

    let mut src_reader = src.open(iso_path)?;
    let copied = io::copy(&mut src_reader, temp_wim.as_file_mut()).map_err(Error::StageIo)?;

    if copied != src.size {
        return Err(Error::StageInvalid(format!(
            "temporary WIM size mismatch: expected {}, copied {}",
            src.size, copied
        )));
    }

    temp_wim.as_file_mut().sync_all().map_err(Error::StageIo)?;

    let temp_path = temp_wim.into_temp_path();
    let part_size_mebibytes = part_size / (1024 * 1024);

    let status = Command::new("wimlib-imagex")
        .arg("split")
        .arg(&temp_path)
        .arg(&output)
        .arg(format!("{part_size_mebibytes}M"))
        .status()
        .map_err(Error::WimlibIo)?;

    if !status.success() {
        return Err(Error::WimlibSplitFailed(status));
    }

    let part_files = split_part_paths(&output)?;
    let mut part_bytes: u64 = 0;

    for part in &part_files {
        let meta = fs::symlink_metadata(part).map_err(Error::StageIo)?;

        if !meta.file_type().is_file() {
            return Err(Error::WimlibOutputInvalid(format!(
                "split output is not a regular file: {:?}",
                part
            )));
        }

        if meta.len() > part_size {
            return Err(Error::WimlibOutputInvalid(format!(
                "split output exceeds planned size: {:?}",
                part
            )));
        }

        part_bytes = part_bytes.checked_add(meta.len()).ok_or_else(|| {
            Error::WimlibOutputInvalid(String::from("split WIM total-size overflow"))
        })?
    }

    Ok(SplitReport {
        source_bytes: src.size,
        part_files,
        part_bytes,
    })
}

fn split_part_paths(first_part: &Path) -> Result<Vec<PathBuf>> {
    let parent = first_part
        .parent()
        .ok_or_else(|| Error::WimlibOutputInvalid(String::from("split output has no parent")))?;

    let stem = first_part
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            Error::WimlibOutputInvalid(String::from("invalid split-output: filename"))
        })?;

    let extension = first_part
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            Error::WimlibOutputInvalid(String::from("split-output filename needs an extension"))
        })?;

    let mut parts = Vec::new();

    for idx in 1_u32.. {
        let path = if idx == 1 {
            first_part.to_path_buf()
        } else {
            parent.join(format!("{stem}{idx}.{extension}"))
        };

        if !path.exists() {
            break;
        }

        parts.push(path);
    }

    if parts.is_empty() {
        return Err(Error::WimlibOutputInvalid(String::from(
            "wimlib created no split WIM files",
        )));
    }

    Ok(parts)
}
