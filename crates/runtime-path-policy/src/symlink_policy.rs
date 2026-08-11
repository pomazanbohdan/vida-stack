use std::fs::Metadata;

pub fn is_symlink(metadata: &Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(test)]
mod tests {
    use super::is_symlink;
    use std::fs;

    #[test]
    fn is_symlink_classifies_regular_files_and_platform_links() {
        let root = std::env::temp_dir().join(format!(
            "runtime-path-policy-symlink-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temporary root");
        let target = root.join("target.txt");
        let link = root.join("link.txt");
        fs::write(&target, "target").expect("target file");
        assert!(!is_symlink(
            &fs::symlink_metadata(&target).expect("target metadata")
        ));

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).expect("symlink creation");
        #[cfg(windows)]
        match std::os::windows::fs::symlink_file(&target, &link) {
            Ok(()) => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                let _ = fs::remove_dir_all(&root);
                return;
            }
            Err(error) => panic!("symlink creation failed: {error}"),
        }

        assert!(is_symlink(
            &fs::symlink_metadata(&link).expect("link metadata")
        ));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(windows)]
    #[test]
    fn is_symlink_classifies_existing_windows_reparse_point_without_creation_privilege() {
        let system_drive = std::env::var_os("SystemDrive").unwrap_or_else(|| "C:".into());
        let mut candidates = vec![
            std::path::PathBuf::from(&system_drive)
                .join("Users")
                .join("All Users"),
            std::path::PathBuf::from(&system_drive).join("Documents and Settings"),
            std::path::PathBuf::from(&system_drive)
                .join("Users")
                .join("Default User"),
        ];
        if let Some(program_data) = std::env::var_os("ProgramData") {
            candidates.push(std::path::PathBuf::from(program_data).join("Application Data"));
        }

        let Some(path) = candidates.into_iter().find(|path| {
            fs::symlink_metadata(path)
                .map(|metadata| metadata.file_type().is_symlink())
                .unwrap_or(false)
        }) else {
            return;
        };

        assert!(is_symlink(
            &fs::symlink_metadata(path).expect("reparse-point metadata")
        ));
    }
}
