use relative_path::{RelativePath, RelativePathBuf};

pub fn parent_dir<P>(path: P) -> RelativePathBuf
where
    P: AsRef<RelativePath>,
{
    path.as_ref()
        .parent()
        .unwrap_or(RelativePath::new("."))
        .to_relative_path_buf()
}
