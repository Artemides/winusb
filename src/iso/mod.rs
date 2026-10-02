mod inspect;

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

use crate::{
    error::{Error, Result},
    extent_reader::{Extent, ExtentReader},
};
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
    pub install_images: Vec<WindowsInstallImage>,
}

#[derive(Debug)]
pub struct WindowsInstallImage {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub architechture: Option<String>,
    pub version: Option<String>,
}

pub fn inspect(path: &Path) -> Result<WindowsMediaInfo> {
    let mut file = File::open(path).map_err(Error::IsoIo)?;

    let image = OpenOpticalImage::open(&mut file, OpenPolicy::PreferUdf)
        .map_err(|e| Error::IsoInvalid(e.to_string()))?;

    match image {
        OpenOpticalImage::Udf(udf) => {
            let (boot_wim, _) = IsoEntry::find(&udf, PathBuf::from(WIN_BOOT_WIM))?;
            let (install_image, install_entry) =
                IsoEntry::find(&udf, PathBuf::from(WIN_INSTALL_WIM))?;
            let (setup_exe, _) = IsoEntry::find(&udf, PathBuf::from(WIN_SETUP_EXE))?;
            let (uefi_boot, _) = IsoEntry::find(&udf, PathBuf::from(WIN_BOOT_MGR_EFI))?;

            let install_images = inspect_wim_metadata(path, &udf, &install_entry)?;

            let win = WindowsMediaInfo {
                boot_wim,
                install_image,
                setup_exe,
                uefi_boot: Some(uefi_boot),
                install_images,
            };

            Ok(win)
        }
        _ => Err(Error::IsoInvalid(String::from("ISO UDF supported only"))),
    }
}

impl IsoEntry {
    fn find(
        volume: &UdfVolume<Borrowed<'_, File>>,
        target_path: PathBuf,
    ) -> Result<(Self, UdfDirEntry)> {
        let mut components = target_path.components();

        let filename = components
            .next_back()
            .and_then(|cmp| cmp.as_os_str().to_str())
            .ok_or_else(|| Error::IsoInvalid(String::from("invalid iso")))?;

        let root = volume
            .root_dir()
            .map_err(|error| Error::IsoInvalid(error.to_string()))?;

        let entry_target = components
            .filter_map(|c| c.as_os_str().to_str())
            .try_fold(root, |dir, entry_path| {
                let entry = dir
                    .find(entry_path)
                    .ok_or_else(|| Error::IsoInvalid(String::from("invalid iso")))?;

                volume
                    .read_directory(&entry.icb)
                    .map_err(|error| Error::IsoInvalid(error.to_string()))
            })
            .and_then(|target| {
                let entry = target
                    .find(filename)
                    .ok_or_else(|| Error::IsoInvalid(String::from("invalid iso")))?;

                Ok(entry.to_owned())
            })?;

        Ok((
            Self {
                path: target_path,
                size: entry_target.size,
                kind: IsoEntryKind::from(&entry_target),
            },
            entry_target,
        ))
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

fn inspect_wim_metadata(
    iso_path: &Path,
    volume: &UdfVolume<Borrowed<'_, File>>,
    entry: &UdfDirEntry,
) -> Result<Vec<WindowsInstallImage>> {
    let layout = volume
        .file_layout(entry)
        .map_err(|error| Error::IsoInvalid(format!("cannot map install image: {error}")))?;
    let extents = layout
        .extents
        .into_iter()
        .map(|extent| Extent {
            source_offset: extent.source_offset,
            len: extent.length,
        })
        .collect();

    let iso_file = File::open(iso_path).map_err(Error::IsoIo)?;
    let mut reader = ExtentReader::new(iso_file, extents).map_err(Error::IsoIo)?;

    let mut header = [0_u8; 8];
    reader.read_exact(&mut header).map_err(Error::IsoIo)?;

    if header != *b"MSWIM\0\0\0" {
        return Err(Error::IsoInvalid(String::from(
            "install image not a wim/esd container",
        )));
    }

    reader.seek(SeekFrom::Start(0)).map_err(Error::IsoIo)?;

    let mut parser = wim_parser::WimParser::from_reader(reader);

    parser
        .parse_full()
        .map_err(|error| Error::IsoInvalid(format!("cannot parse wim metadata: {error}")))?;

    Ok(parser
        .get_images()
        .iter()
        .map(|image| WindowsInstallImage {
            index: image.index,
            name: image.name.clone(),
            description: image.description.clone(),
            architechture: image.architecture.clone(),
            version: image.version.clone(),
        })
        .collect())
}
