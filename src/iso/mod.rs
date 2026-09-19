mod inspect;

use std::{
    fs::File,
    path::{Path, PathBuf},
};

use crate::error::{Error, Result};
use hadris_optical::{
    OpenPolicy,
    cd::Borrowed,
    sync::OpenOpticalImage,
    udf::{UdfVolume, dir::UdfDirEntry},
};

#[derive(Debug)]
pub struct IsoEntry {
    pub path: PathBuf,
    pub size: u64,
    pub kind: IsoEntryKind,
}

#[derive(Debug)]
pub enum IsoEntryKind {
    File,
    Directory,
}

const WIN_BOOT_WIM: &str = "sources/boot.wim";
const WIN_INSTALL_WIM: &str = "sources/install.wim";
const WIN_SETUP_EXE: &str = "sources/setup.exe";
const WIN_BOOT_MGR_EFI: &str = "bootmgr.efi";

#[derive(Debug)]
pub struct WindowsMediaInfo {
    pub boot_wim: IsoEntry,
    pub install_image: IsoEntry,
    pub setup_exe: IsoEntry,
    pub uefi_boot: Option<IsoEntry>,
}

pub fn inspect(path: &Path) -> Result<WindowsMediaInfo> {
    let mut file = File::open(path).map_err(Error::IsoIo)?;

    let image = OpenOpticalImage::open(&mut file, OpenPolicy::PreferUdf)
        .map_err(|e| Error::IsoInvalid(e.to_string()))?;

    match image {
        OpenOpticalImage::Udf(udf) => {
            let win = WindowsMediaInfo {
                boot_wim: IsoEntry::find(&udf, PathBuf::from(WIN_BOOT_WIM))?,
                install_image: IsoEntry::find(&udf, PathBuf::from(WIN_INSTALL_WIM))?,
                setup_exe: IsoEntry::find(&udf, PathBuf::from(WIN_SETUP_EXE))?,
                uefi_boot: Some(IsoEntry::find(&udf, PathBuf::from(WIN_BOOT_MGR_EFI))?),
            };

            Ok(win)
        }
        _ => Err(Error::IsoInvalid(String::from("ISO UDF supported only"))),
    }
}

impl IsoEntry {
    fn find(volume: &UdfVolume<Borrowed<'_, File>>, target_path: PathBuf) -> Result<Self> {
        let mut components = target_path.components();

        let filename = components
            .next_back()
            .and_then(|cmp| cmp.as_os_str().to_str())
            .ok_or_else(|| Error::IsoInvalid(String::from("invalid iso")))?;

        let root = volume.root_dir().unwrap();
        let entry_target = components
            .filter_map(|c| c.as_os_str().to_str())
            .try_fold(root, |dir, entry_path| {
                let entry = dir
                    .find(entry_path)
                    .ok_or_else(|| Error::IsoInvalid(String::from("invalid iso")))?;

                let s = volume.read_directory(&entry.icb).unwrap();

                Ok(s)
            })
            .and_then(|target| {
                let entry = target
                    .find(filename)
                    .ok_or_else(|| Error::IsoInvalid(String::from("invalid iso")))?;

                Ok(entry.to_owned())
            })?;

        Ok(Self {
            path: target_path,
            size: entry_target.size,
            kind: IsoEntryKind::from(&entry_target),
        })
    }
}

impl IsoEntryKind {
    fn from(entry: &UdfDirEntry) -> Self {
        if entry.is_dir() {
            return Self::Directory;
        }

        Self::File
    }
}
