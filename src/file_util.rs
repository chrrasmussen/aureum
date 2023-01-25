use std::path::{Path, PathBuf};

pub fn parent_dir<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let parent_dir = path.as_ref().parent().unwrap_or(Path::new("."));
    if parent_dir == Path::new("") {
        PathBuf::from(".")
    } else {
        parent_dir.to_path_buf()
    }
}
