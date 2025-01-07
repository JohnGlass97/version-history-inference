use std::collections::HashMap;
use std::fs::{self, DirEntry};
use std::io::{self, Error, ErrorKind, Result};
use std::path::Path;

use crate::types::{FileData, Version};

fn walk_dir(dir: &Path, file_paths: &mut Vec<Box<Path>>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk_dir(&path, file_paths)?;
            } else {
                file_paths.push(path.into());
            }
        }
    }
    Ok(())
}

fn dirs_in_dir(dir: &Path) -> Result<Vec<Box<Path>>> {
    let mut dir_paths: Vec<Box<Path>> = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            dir_paths.push(path.into());
        }
    }

    Ok(dir_paths)
}

fn read_file_to_str_opt(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) => match e.kind() {
            io::ErrorKind::InvalidData => Ok(None),
            _ => Err(e),
        },
    }
}

pub fn load_versions(dir: &Path) -> Result<Vec<Version>> {
    let version_paths = dirs_in_dir(dir)?;
    let mut versions: Vec<Version> = Vec::new();

    for version_path in version_paths {
        let mut file_paths: Vec<Box<Path>> = Vec::new();
        walk_dir(&version_path, &mut file_paths);

        let mut files: HashMap<String, FileData> = HashMap::new();

        for file_path in file_paths {
            let rel_path = file_path
                .strip_prefix(&version_path)
                .map_err(|e| Error::new(ErrorKind::Other, e))?;

            files.insert(
                rel_path.to_string_lossy().to_string(),
                FileData {
                    text_content: read_file_to_str_opt(&file_path)?,
                },
            );
        }

        versions.push(Version {
            version_path,
            files,
        });
    }

    Ok(versions)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCleanup;

    impl Drop for TestCleanup {
        fn drop(&mut self) {
            fs::remove_dir_all("test_temp").unwrap();
        }
    }

    #[test]
    fn test_load_versions() {
        let x = TestCleanup;

        fs::create_dir_all("test_temp/version_1").unwrap();
        fs::create_dir_all("test_temp/version_2").unwrap();
        fs::write("test_temp/version_1/file_a.txt", "file_a").unwrap();
        fs::write("test_temp/version_1/file_b.txt", "file_b").unwrap();
        fs::write("test_temp/version_2/file_a.txt", "file_a_new").unwrap();
        fs::write("test_temp/version_2/file_b.txt", "file_b_new").unwrap();

        let versions = load_versions(Path::new("test_temp")).unwrap();

        assert_eq!(versions.len(), 2);

        assert_eq!(
            versions[0].version_path,
            Path::new("test_temp/version_1").into()
        );
        let files_1 = &versions[0].files;
        assert_eq!(
            files_1["file_a.txt"].text_content.as_ref().unwrap(),
            "file_a"
        );
        assert_eq!(
            files_1["file_b.txt"].text_content.as_ref().unwrap(),
            "file_b"
        );

        assert_eq!(
            versions[1].version_path,
            Path::new("test_temp/version_2").into()
        );
        let files_2 = &versions[1].files;
        assert_eq!(
            files_2["file_a.txt"].text_content.as_ref().unwrap(),
            "file_a_new"
        );
        assert_eq!(
            files_2["file_b.txt"].text_content.as_ref().unwrap(),
            "file_b_new"
        );
    }
}
