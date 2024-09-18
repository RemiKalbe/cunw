#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[cfg(windows)]
use walkdir::DirEntry;

#[cfg(windows)]
use crate::error::{CunwError, Result};

// Windows-specific constant used to check if a file is hidden.+
#[cfg(windows)]
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

#[cfg(windows)]
pub fn is_hidden_dir_entry(file: &DirEntry) -> Result<bool> {
    Ok(file
        .metadata()
        .map_err(|err| CunwError::new(err.into()).with_file(file.clone().into_path()))?
        .file_attributes()
        & FILE_ATTRIBUTE_HIDDEN
        != 0)
}
