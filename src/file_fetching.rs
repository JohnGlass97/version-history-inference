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

fn get_relative_path<'a>(path: &'a Path, base: &'a Path) -> Result<&'a Path> {
    let rel_path = path
        .strip_prefix(&base)
        .map_err(|e| Error::new(ErrorKind::Other, e))?;
    Ok(rel_path)
}

pub fn load_versions(dir: &Path) -> Result<HashMap<String, Version>> {
    let version_paths = dirs_in_dir(dir)?;
    let mut versions: HashMap<String, Version> = HashMap::new();

    for version_path in version_paths {
        let mut file_paths: Vec<Box<Path>> = Vec::new();
        walk_dir(&version_path, &mut file_paths);

        let mut files: HashMap<String, FileData> = HashMap::new();

        for file_path in file_paths {
            let rel_path = get_relative_path(&file_path, &version_path)?;
            files.insert(
                rel_path.to_string_lossy().to_string(),
                FileData {
                    text_content: read_file_to_str_opt(&file_path)?,
                },
            );
        }

        let version_name = get_relative_path(&version_path, &dir)?
            .to_string_lossy()
            .to_string();

        versions.insert(
            version_name,
            Version {
                version_path,
                files,
            },
        );
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

        let version_1 = &versions["version_1"];
        assert_eq!(
            version_1.version_path,
            Path::new("test_temp/version_1").into()
        );
        let files_1 = &version_1.files;
        assert_eq!(
            files_1["file_a.txt"].text_content.as_ref().unwrap(),
            "file_a"
        );
        assert_eq!(
            files_1["file_b.txt"].text_content.as_ref().unwrap(),
            "file_b"
        );

        let version_2 = &versions["version_2"];
        assert_eq!(
            version_2.version_path,
            Path::new("test_temp/version_2").into()
        );
        let files_2 = &version_2.files;
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
