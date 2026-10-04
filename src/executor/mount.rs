use std::path::Path;

use crate::executor::CommandSpec;

pub fn mount_commands(partition: &Path, mountpoint: &Path) -> Vec<CommandSpec> {
    vec![CommandSpec {
        program: "mount",
        args: vec![
            "-t".into(),
            "vfat".into(),
            partition.display().to_string(),
            mountpoint.display().to_string(),
        ],
    }]
}

pub fn unmount_commands(mountpoint: &Path) -> Vec<CommandSpec> {
    vec![CommandSpec {
        program: "umount",
        args: vec![mountpoint.display().to_string()],
    }]
}

pub fn sync_commands() -> Vec<CommandSpec> {
    vec![CommandSpec {
        program: "sync",
        args: Vec::new(),
    }]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn builds_mount_command() {
        assert_eq!(
            mount_commands(Path::new("/dev/sdb1"), Path::new("/mnt/winusb")),
            vec![CommandSpec {
                program: "mount",
                args: vec![
                    "-t".into(),
                    "vfat".into(),
                    "/dev/sdb1".into(),
                    "/mnt/winusb".into(),
                ],
            }]
        );
    }

    #[test]
    fn builds_unmount_command() {
        assert_eq!(
            unmount_commands(Path::new("/mnt/winusb")),
            vec![CommandSpec {
                program: "umount",
                args: vec!["/mnt/winusb".into()],
            }]
        );
    }

    #[test]
    fn builds_sync_command() {
        assert_eq!(
            sync_commands(),
            vec![CommandSpec {
                program: "sync",
                args: Vec::new(),
            }]
        );
    }
}
