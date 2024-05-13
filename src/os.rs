use miette::Result;
use walkdir::DirEntry;

// Windows-specific constant used to check if a file is hidden.
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;

#[cfg(windows)]
pub fn is_hidden_dir_entry(file: &DirEntry) -> Result<bool> {
    Ok(file.metadata()?.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}
