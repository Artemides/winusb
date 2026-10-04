use std::{
    fs::{self, OpenOptions},
    io,
    path::Path,
};

use crate::{
    error::{Error, Result},
    executor::staging_path,
};

#[derive(Debug, Default)]
pub struct CopyReport {
    pub copied_files: u64,
    pub copied_bytes: u64,
}

pub fn copy_payload_tree(src: &Path, dest: &Path) -> Result<CopyReport> {
    if !src.is_dir() {
        return Err(Error::PayloadInvalid(format!(
            "payload source is not a directory: {src:#?}",
        )));
    }

    if !dest.is_dir() {
        return Err(Error::PayloadInvalid(format!(
            "payload destination is not a directory: {dest:#?}",
        )));
    }

    let mut report = CopyReport::default();

    copy_directory(src, src, dest, &mut report)?;

    Ok(report)
}

fn copy_directory(
    src_root: &Path,
    current_source: &Path,
    dest_root: &Path,
    report: &mut CopyReport,
) -> Result<()> {
    for entry in fs::read_dir(current_source).map_err(Error::PayloadIo)? {
        let entry = entry.map_err(Error::PayloadIo)?;
        let src_path = entry.path();
        let src_relative_path = src_path.strip_prefix(src_root).expect("not inside root");

        let dest_path = staging_path(dest_root, src_relative_path)?;

        let file_type = entry.file_type().map_err(Error::PayloadIo)?;
        if file_type.is_dir() {
            fs::create_dir_all(&dest_path).map_err(Error::PayloadIo)?;

            copy_directory(src_root, &src_path, dest_root, report)?;

            continue;
        }

        if !file_type.is_file() {
            return Err(Error::PayloadInvalid(format!(
                "payload contains unsupported entry: {src_path:#?}"
            )));
        }

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent).map_err(Error::PayloadIo)?;
        }

        let e_size = entry.metadata().map_err(Error::PayloadIo)?.len();
        let mut input = fs::File::open(&src_path).map_err(Error::PayloadIo)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&dest_path)
            .map_err(Error::PayloadIo)?;

        let copied = io::copy(&mut input, &mut output).map_err(Error::PayloadIo)?;

        if copied != e_size {
            return Err(Error::PayloadInvalid(format!(
                "copied size mismatch for {src_relative_path:#?}",
            )));
        }

        report.copied_files += 1;
        report.copied_bytes = report
            .copied_bytes
            .checked_add(copied)
            .ok_or_else(|| Error::PayloadInvalid(String::from("copied-byte overflow")))?;
    }

    Ok(())
}

pub fn validate_payload_tree(source: &Path) -> Result<()> {
    let required_files = ["bootmgr.efi", "sources/boot.wim", "sources/setup.exe"];

    for relative_path in required_files {
        let path = source.join(relative_path);

        if !path.is_file() {
            return Err(Error::PayloadInvalid(format!(
                "required payload file is missing: {}",
                path.display()
            )));
        }
    }

    let install_wim = source.join("sources/install.wim");
    let install_swm = source.join("sources/install.swm");

    if !install_wim.is_file() && !install_swm.is_file() {
        return Err(Error::PayloadInvalid(String::from(
            "payload needs sources/install.wim or sources/install.swm",
        )));
    }

    Ok(())
}
