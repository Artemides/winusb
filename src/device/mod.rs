use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::{Error, Result};

const SYS_BLOCK_PATH: &str = "/sys/class/block";
const SYSFS_SECTOR_SUZE: u64 = 512;

#[derive(Debug)]
pub struct BlockDevice {
    pub kernel_name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub removable: bool,
    pub model: Option<String>,
    pub serial: Option<String>,
}

pub fn list_block_devices() -> Result<Vec<BlockDevice>> {
    let mut devices = Vec::new();

    let entries = fs::read_dir(SYS_BLOCK_PATH).map_err(Error::DeviceIo)?;

    for entry in entries {
        let entry = entry.map_err(Error::DeviceIo)?;
        let sys_path = entry.path();

        if sys_path.join("partition").exists() {
            continue;
        }

        let kernel_name = entry.file_name().to_string_lossy().into_owned();

        devices.push(read_device(sys_path, kernel_name)?);
    }

    devices.sort_by(|l, r| l.kernel_name.cmp(&r.kernel_name));

    Ok(devices)
}

fn read_device(sys_path: PathBuf, kernel_name: String) -> Result<BlockDevice> {
    let sectors = read_required_u64(&sys_path.join("size"))?;

    let size_bytes = sectors
        .checked_mul(SYSFS_SECTOR_SUZE)
        .ok_or_else(|| Error::DeviceInvalid(String::from("device size overflows u64")))?;

    let removable = read_required_string(&sys_path.join("removable"))? == "1";

    Ok(BlockDevice {
        kernel_name: kernel_name.clone(),
        path: PathBuf::from("/dev").join(&kernel_name),
        size_bytes,
        removable,
        model: read_optional_string(&sys_path.join("device/model")),
        serial: read_optional_string(&sys_path.join("device/serial")),
    })
}

fn read_required_string(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .map_err(Error::DeviceIo)
}

fn read_required_u64(path: &Path) -> Result<u64> {
    let val = read_required_string(&path)?;

    val.parse().map_err(|_| {
        Error::DeviceInvalid(format!("invalid numeric sysfs value at {}", path.display()))
    })
}

fn read_optional_string(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
