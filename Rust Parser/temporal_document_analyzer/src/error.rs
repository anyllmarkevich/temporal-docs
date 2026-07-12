//! Services to check whether the input file tree is formatted correctly.

use anyhow::{anyhow, Result};
use itertools::Itertools;
use std::ffi::OsStr;

/// Make sure the input directory is approximately formatted correctly.
pub fn check_file_tree(tree: Vec<ignore::DirEntry>) -> Result<Vec<ignore::DirEntry>> {
    // Check to make sure the input directory has files or directories inside it.
    if tree.len() == 0 {
        return Err(anyhow!(
            "Input data structure is missing. Input directory must contain folders, which must themselves contain documents"
        ));
    } else if !tree.iter().any(|e| e.depth() == 2) {
        // Check to ensure the input directory has files or directories in the subdirectories
        return Err(anyhow!(
            "All documents are missing. Sub-folders in input directory must contain documents"
        ));
    } else if tree
        .iter()
        .any(|e| e.depth() == 2 && e.file_type().map_or(false, |f| f.is_dir()))
    {
        // Ensure that there are no further directories in the subdirectories, effectively capping the depth at 2
        return Err(anyhow!("Subdirectories should not contain any further directories. The following directories are within subdirectories and cause a violation of this requirement:\n{}", tree.iter().filter(|e| e.depth() == 2 && e.file_type().map_or(false, |f| f.is_dir()) ).map(|e| e.path().as_os_str().to_string_lossy()).join("\n") ));
    } else if tree
        .iter()
        .any(|e| e.depth() == 1 && e.file_type().map_or(false, |f| f.is_file()))
    {
        // Make sure input directory only directly contains directories and no files
        return Err(anyhow!("Input directory should only contain subdirectories, which themslves contain documents. The following files violate this condition by being in the input directory as opposed to subdirectories:\n{}", tree.iter().filter(|e| e.depth() == 1 && e.file_type().map_or(false, |f| f.is_file()) ).map(|e| e.path().as_os_str().to_string_lossy()).join("\n") ));
    } else if tree
        .iter()
        .filter(|e| e.file_type().map_or(false, |f| f.is_file()))
        .any(|e| {
            e.path()
                .extension()
                .and_then(OsStr::to_str)
                .expect("Could not get file extension")
                != "docx".to_string()
        })
    {
        // Make sure that all documents have a ".docx" extension
        return Err(anyhow!("All documents must be Microsoft Word files with the \".docx\" file extension. The following files violated this requirement:\n{}", tree.iter().filter(|e| e.file_type().map_or(false, |f| f.is_file())).filter(|e| e.path().extension().and_then(OsStr::to_str).expect("Could not get file extension") != "docx").map(|e| e.path().as_os_str().to_string_lossy()).join("\n") ));
    }
    Ok(tree)
}
