use std::os::windows::fs::MetadataExt;

use walkdir::DirEntry;

use crate::error::{CunwError, Result};

// Windows-specific constant used to check if a file is hidden.
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

#[cfg(windows)]
pub fn is_hidden_dir_entry(file: &DirEntry) -> Result<bool> {
    Ok(file.metadata().map_err(|err| CunwError::from(err.into()).with_file(file.into_path()))?.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}
