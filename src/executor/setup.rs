use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    error::{Error, Result},
    executor::{prepare::SetupCustomization, staging_path},
};

const AUTO_UNATTEND_FILE: &str = "Autounattend.xml";

const WIN11_BYPASS_ANSWER_FILE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<unattend xmlns="urn:schemas-microsoft-com:unattend"
          xmlns:wcm="http://schemas.microsoft.com/WMIConfig/2002/State">
  <settings pass="windowsPE">
    <component name="Microsoft-Windows-Setup"
               processorArchitecture="amd64"
               publicKeyToken="31bf3856ad364e35"
               language="neutral"
               versionScope="nonSxS">
      <RunSynchronous>
        <RunSynchronousCommand wcm:action="add">
          <Order>1</Order>
          <Description>Bypass TPM requirement</Description>
          <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassTPMCheck /t REG_DWORD /d 1 /f</Path>
        </RunSynchronousCommand>
        <RunSynchronousCommand wcm:action="add">
          <Order>2</Order>
          <Description>Bypass Secure Boot requirement</Description>
          <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassSecureBootCheck /t REG_DWORD /d 1 /f</Path>
        </RunSynchronousCommand>
        <RunSynchronousCommand wcm:action="add">
          <Order>3</Order>
          <Description>Bypass RAM requirement</Description>
          <Path>reg add HKLM\SYSTEM\Setup\LabConfig /v BypassRAMCheck /t REG_DWORD /d 1 /f</Path>
        </RunSynchronousCommand>
      </RunSynchronous>
    </component>
  </settings>
</unattend>
"#;

pub fn setup_customization(dest: &Path, custom: &SetupCustomization) -> Result<Option<PathBuf>> {
    let SetupCustomization::BypassChecks = custom else {
        return Ok(None);
    };

    let file = staging_path(dest, Path::new(AUTO_UNATTEND_FILE))?;
    let mut out = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file)
        .map_err(Error::PayloadIo)?;

    out.write_all(WIN11_BYPASS_ANSWER_FILE.as_bytes())
        .map_err(Error::PayloadIo)?;

    out.sync_all().map_err(Error::PayloadIo)?;

    Ok(Some(file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_mode_writes_nothing() {
        let temp = tempfile::tempdir().unwrap();

        let answer_file = setup_customization(temp.path(), &SetupCustomization::Standard).unwrap();

        assert_eq!(answer_file, None);
    }

    #[test]
    fn bypass_mode_writes_root_answer_file() {
        let temp = tempfile::tempdir().unwrap();

        let answer_file = setup_customization(temp.path(), &SetupCustomization::BypassChecks)
            .unwrap()
            .unwrap();

        assert_eq!(answer_file, temp.path().join(AUTO_UNATTEND_FILE));

        let contents = std::fs::read_to_string(answer_file).unwrap();
        assert!(contents.contains("BypassTPMCheck"));
        assert!(contents.contains("BypassSecureBootCheck"));
        assert!(contents.contains("BypassRAMCheck"));
    }
}
