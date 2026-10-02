use crate::iso::WindowsMediaInfo;

pub const FAT32_MAX_FILE_SIZE: u64 = 3_800 * 1024 * 1024;

#[derive(Debug)]
pub enum InstallImageHandling {
    CopyAsIs,
    Split { max_part_size: u64 },
}

#[derive(Debug)]
pub struct MediaPlan {
    pub install_image_handling: InstallImageHandling,
}

pub fn media_plan(media: &WindowsMediaInfo) -> MediaPlan {
    let install_image_handling = if media.install_image.size > FAT32_MAX_FILE_SIZE {
        InstallImageHandling::Split {
            max_part_size: FAT32_MAX_FILE_SIZE,
        }
    } else {
        InstallImageHandling::CopyAsIs
    };

    MediaPlan {
        install_image_handling,
    }
}
