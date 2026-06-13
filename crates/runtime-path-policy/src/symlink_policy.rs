use std::fs::Metadata;

pub fn is_symlink(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}
