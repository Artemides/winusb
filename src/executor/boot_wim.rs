use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::Command,
};

use hadris_optical::iso::io;
use tempfile::Builder;

use crate::{
    error::{Error, Result},
    executor::staging_path,
};

const SETUP_IMAGE_INDEX: u32 = 2;
const SYSTEM_HIVE_PATH: &str = "Windows/System32/config/SYSTEM";

#[derive(Debug)]
pub struct BootWimInspection {
    pub boot_wim: PathBuf,
    pub setup_image_index: u32,
    pub system_hive_bytes: u64,
}

pub fn check_bootwim_tools() -> Result<()> {
    let status = Command::new("hivexregedit")
        .arg("--help")
        .status()
        .map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound.into() {
                Error::RequiredToolMissing {
                    program: "hivexregedit",
                    install_hint: "sudo dnf install perl-hivex",
                }
            } else {
                Error::WimlibIo(e)
            }
        })?;

    if !status.success() {
        return Err(Error::BootWimInvalid("hivexregedit run error".to_string()));
    }

    Ok(())
}

pub fn inspect_staged_boot_wim(staging_dir: &Path, temp_dir: &Path) -> Result<BootWimInspection> {
    check_bootwim_tools()?;

    if !temp_dir.is_dir() {
        return Err(Error::BootWimInvalid(format!(
            "temporary directory does not exist: {}",
            temp_dir.display()
        )));
    }

    let boot_wim = staging_path(staging_dir, Path::new("sources/boot.wim"))?;

    let meta = fs::symlink_metadata(&boot_wim).map_err(Error::PayloadIo)?;

    if !meta.file_type().is_file() {
        return Err(Error::BootWimInvalid(format!(
            "staged boot.wim is not a regular file: {boot_wim:#?}"
        )));
    }

    let work_dir = Builder::new()
        .prefix("wimusb-boot-wim-")
        .tempdir_in(temp_dir)
        .map_err(Error::PayloadIo)?;

    let command = format!(
        "wimlib-imagex extract {:?} {} {}",
        boot_wim, SETUP_IMAGE_INDEX, SYSTEM_HIVE_PATH
    );

    let status = Command::new("wimlib-imagex")
        .arg("extract")
        .arg(&boot_wim)
        .arg(SETUP_IMAGE_INDEX.to_string())
        .arg(SYSTEM_HIVE_PATH)
        .arg("--dest-dir")
        .arg(work_dir.path())
        .status()
        .map_err(Error::WimlibIo)?;

    if !status.success() {
        return Err(Error::CommandFailed { command, status });
    }

    let hive = work_dir.path().join(SYSTEM_HIVE_PATH);
    let hive_meta = fs::symlink_metadata(&hive).map_err(Error::PayloadIo)?;

    if !hive_meta.is_file() {
        return Err(Error::BootWimInvalid(format!(
            "Setup image has no SYSTEM hive at {}",
            SYSTEM_HIVE_PATH
        )));
    }

    Ok(BootWimInspection {
        boot_wim,
        setup_image_index: SETUP_IMAGE_INDEX,
        system_hive_bytes: hive_meta.len(),
    })
}

const BYPASS_VALUES: [&str; 3] = ["BypassTPMCheck", "BypassSecureBootCheck", "BypassRAMCheck"];

const LAB_CONFIG_REG: &str = r#"Windows Registry Editor Version 5.00

[\Setup\LabConfig]
"BypassTPMCheck"=dword:00000001
"BypassSecureBootCheck"=dword:00000001
"BypassRAMCheck"=dword:00000001
"#;

#[derive(Debug)]
pub struct BootWimPatchReport {
    pub boot_wim: PathBuf,
    pub setup_image_index: u32,
    pub bypass_values: [&'static str; 3],
}

pub fn patch_staged_boot_wim(staging_dir: &Path) -> Result<BootWimPatchReport> {
    let boot_wim = staging_path(staging_dir, Path::new("sources/boot.wim"))?;

    let parent = boot_wim.parent().ok_or_else(|| {
        Error::BootWimInvalid(String::from("staged boot.wim has no parent directory"))
    })?;

    let work_dir = Builder::new()
        .prefix("winusb-boot-wim-patch-")
        .tempdir_in(parent)
        .map_err(Error::PayloadIo)?;

    let candidate_wim = work_dir.path().join("boot.wim");
    fs::copy(&boot_wim, &candidate_wim).map_err(Error::PayloadIo)?;

    File::open(&candidate_wim)
        .map_err(Error::PayloadIo)?
        .sync_all()
        .map_err(Error::PayloadIo)?;

    //observe <-
    let extracted = work_dir.path().join("extracted");
    fs::create_dir(&extracted).map_err(Error::PayloadIo)?;

    extract_system_hive(&candidate_wim, &extracted)?;

    let system_hive = extracted.join(SYSTEM_HIVE_PATH);
    // labconfig in extracted?
    let reg_file = work_dir.path().join("labconfig.reg");

    fs::write(&reg_file, LAB_CONFIG_REG).map_err(Error::PayloadIo)?;

    let merge_command = format!(
        "hivexregedit --merge {} {}",
        system_hive.display(),
        reg_file.display()
    );

    let mut merge = Command::new("hivexregedit");
    merge
        .arg("--merge")
        .arg("--prefix")
        .arg(r"HKEY_LOCAL_MACHINE\SYSTEM")
        .arg(&system_hive)
        .arg(&reg_file);

    run_patch_command(merge, merge_command)?;

    let update_command = wim_update_command(&system_hive)?;

    let mut update = Command::new("wimlib-imagex");
    update
        .arg("update")
        .arg(&candidate_wim)
        .arg(SETUP_IMAGE_INDEX.to_string())
        .arg("--command")
        .arg(&update_command);

    run_patch_command(
        update,
        format!(
            "wimlib-imagex update {} {} --command {}",
            candidate_wim.display(),
            SETUP_IMAGE_INDEX,
            update_command,
        ),
    )?;

    let mut verify_wim = Command::new("wimlib-imagex");
    verify_wim.arg("verify").arg(&candidate_wim);

    run_patch_command(
        verify_wim,
        format!("wimlib-imagex verify {}", candidate_wim.display()),
    )?;

    let verified = work_dir.path().join("verified");
    fs::create_dir(&verified).map_err(Error::PayloadIo)?;

    extract_system_hive(&candidate_wim, &verified)?;

    verify_lab_config(&verified.join(SYSTEM_HIVE_PATH))?;

    fs::rename(&candidate_wim, &boot_wim).map_err(Error::PayloadIo)?;

    File::open(&boot_wim)
        .map_err(Error::PayloadIo)?
        .sync_all()
        .map_err(Error::PayloadIo)?;

    Ok(BootWimPatchReport {
        boot_wim,
        setup_image_index: SETUP_IMAGE_INDEX,
        bypass_values: BYPASS_VALUES,
    })
}

fn extract_system_hive(wim: &Path, dest: &Path) -> Result<()> {
    let description = format!(
        "wimlib-imagex extract {} {} {}",
        wim.display(),
        SETUP_IMAGE_INDEX,
        SYSTEM_HIVE_PATH,
    );

    let mut extract = Command::new("wimlib-imagex");
    extract
        .arg("extract")
        .arg(wim)
        .arg(SETUP_IMAGE_INDEX.to_string())
        .arg(SYSTEM_HIVE_PATH)
        .arg("--dest-dir")
        .arg(dest);

    run_patch_command(extract, description)
}

fn verify_lab_config(system_hive: &Path) -> Result<()> {
    let description = format!(
        "hivexregedit --export {} Setup\\LabConfig",
        system_hive.display()
    );

    let output = Command::new("hivexregedit")
        .arg("--export")
        .arg("--prefix")
        .arg(r"HKEY_LOCAL_MACHINE\SYSTEM")
        .arg(system_hive)
        .arg(r"\Setup\LabConfig")
        .output()
        .map_err(|source| Error::CommandIo {
            command: description.clone(),
            source,
        })?;

    if !output.status.success() {
        return Err(Error::BootWimCommandFailed {
            command: description,
            status: output.status,
        });
    }

    let exported = String::from_utf8(output.stdout).map_err(|error| {
        Error::BootWimInvalid(format!("registry export was not UTF-8: {error}"))
    })?;

    for name in BYPASS_VALUES {
        let expected = format!("\"{name}\"=dword:00000001");

        if !exported.contains(&expected) {
            return Err(Error::BootWimInvalid(format!(
                "patched SYSTEM hive is missing {expected}"
            )));
        }
    }

    Ok(())
}

fn wim_update_command(system_hive: &Path) -> Result<String> {
    let source = system_hive.to_str().ok_or_else(|| {
        Error::BootWimInvalid(String::from("SYSTEM hive path is not valid UTF-8"))
    })?;

    if source.contains('\'') || source.contains('\n') || source.contains('\r') {
        return Err(Error::BootWimInvalid(format!(
            "unsupported SYSTEM hive path: {source}"
        )));
    }

    Ok(format!("add '{source}' '/Windows/System32/config/SYSTEM'"))
}

fn run_patch_command(mut command: Command, description: String) -> Result<()> {
    let status = command.status().map_err(|source| Error::CommandIo {
        command: description.clone(),
        source,
    })?;

    if !status.success() {
        return Err(Error::BootWimCommandFailed {
            command: description,
            status,
        });
    }

    Ok(())
}
