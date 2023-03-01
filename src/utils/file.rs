use relative_path::{RelativePath, RelativePathBuf};
use std::path::{Path, PathBuf};

pub fn parent_dir<P>(path: P) -> RelativePathBuf
where
    P: AsRef<RelativePath>,
{
    path.as_ref()
        .parent()
        .unwrap_or_else(|| RelativePath::new("."))
        .to_relative_path_buf()
}

/// Find absolute path to executable
///
/// First looks for executable in local directory (`in_dir`).
/// Otherwise, looks for executable in PATH.
pub fn find_executable_path<P>(binary_name: &str, in_dir: P) -> Result<PathBuf, which::Error>
where
    P: AsRef<Path>,
{
    let paths = in_dir.as_ref().as_os_str();

    // Search local directory
    let mut local_executables = which::which_in_global(&binary_name, Some(paths))?;
    if let Some(path) = local_executables.next() {
        return Ok(path);
    }

    // Search PATH
    which::which(binary_name)
}

/// Split file name on colon
///
/// This is especially important on Windows that uses colon to separate
/// disk name from the rest of the path.
pub fn split_file_name(p: &Path) -> (PathBuf, Option<String>) {
    if let Some(file_name) = p.file_name() {
        let mut new_path = PathBuf::from(p);

        if let Some((prefix, suffix)) = file_name.to_string_lossy().split_once(':') {
            new_path.set_file_name(prefix);

            return (new_path, Some(suffix.to_owned()));
        }
    }

    (p.to_path_buf(), None)
}
