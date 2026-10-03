use std::{
    collections::HashSet,
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

pub fn find_block_device(path: &Path) -> Result<BlockDevice> {
    let canonical_path = fs::canonicalize(path).map_err(Error::DeviceIo)?;
    let kernel_name = canonical_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            Error::DeviceInvalid(format!("invalid block device-path: {}", path.display()))
        })?
        .to_owned();

    let sys_path = Path::new(SYS_BLOCK_PATH).join(&kernel_name);

    if !sys_path.exists() {
        return Err(Error::DeviceInvalid(format!(
            "not a known linux block device: {}",
            path.display()
        )));
    };

    if sys_path.join("partition").exists() {
        return Err(Error::DeviceInvalid(format!(
            "target must be a whole device, not a partition: {}",
            path.display()
        )));
    };

    read_device(sys_path, kernel_name)
}

pub fn ensure_unmounted(device: &BlockDevice) -> Result<()> {
    let sys_path = Path::new(SYS_BLOCK_PATH).join(&device.kernel_name);

    let device_numbers = collect_related_device_numbers(&sys_path, &mut HashSet::new())?;

    let mountinfo = fs::read_to_string("/proc/self/mountinfo").map_err(Error::DeviceIo)?;

    for line in mountinfo.lines() {
        if let Some(device_number) = mount_device_number(line)
            && device_numbers.contains(device_number)
        {
            return Err(Error::TargetMounted {
                path: device.path.clone(),
            });
        }
    }

    Ok(())
}

fn collect_related_device_numbers(
    sys_path: &Path,
    visited: &mut HashSet<String>,
) -> Result<HashSet<String>> {
    let kernel_name = sys_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::DeviceInvalid(String::from("invalid sysfs block-device path")))?
        .to_owned();

    if !visited.insert(kernel_name) {
        return Ok(HashSet::new());
    }

    let mut nums = HashSet::new();

    nums.insert(read_required_string(&sys_path.join("dev"))?);

    // include partitions
    for entry in fs::read_dir(sys_path).map_err(Error::DeviceIo)? {
        let entry = entry.map_err(Error::DeviceIo)?;
        let child_path = entry.path();

        if child_path.join("partition").exists() {
            nums.extend(collect_related_device_numbers(&child_path, visited)?);
        }
    }

    // include device mapper

    let holders_path = sys_path.join("holders");
    if holders_path.exists() {
        for entry in fs::read_dir(holders_path).map_err(Error::DeviceIo)? {
            let entry = entry.map_err(Error::DeviceIo)?;
            let holder_name = entry.file_name();

            let holder_path = Path::new(SYS_BLOCK_PATH).join(holder_name);

            nums.extend(collect_related_device_numbers(&holder_path, visited)?);
        }
    }

    Ok(nums)
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

fn mount_device_number(line: &str) -> Option<&str> {
    // mountinfo format begins:
    // mount-id parent-id major:minor root mount-point ...
    line.split_whitespace().nth(2)
}
