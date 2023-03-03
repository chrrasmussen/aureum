use relative_path::{RelativePath, RelativePathBuf};
use std::path::{Path, PathBuf};

/// Get parent directory of path
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

/// Get a platform-independent version of a file path
pub fn display_path<P>(path: P) -> String
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if path.is_absolute() {
        if let Some(file_name) = path.file_name() {
            let display_name = file_name.to_string_lossy().to_string();

            // Workaround for Windows: Remove .exe suffix
            let display_name_without_exe: String = display_name
                .clone()
                .strip_suffix(".exe")
                .map_or(display_name, String::from);

            format!("<absolute path to '{}'>", display_name_without_exe)
        } else {
            String::from("<root directory>")
        }
    } else {
        match RelativePathBuf::from_path(path) {
            Ok(relative_path) => relative_path.to_string(),
            Err(_) => String::from("<invalid path>"),
        }
    }
}
