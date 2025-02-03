use std::collections::HashMap;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

use crate::types::{FileData, Version};

fn walk_dir(dir: &Path, file_paths: &mut Vec<Box<Path>>) -> io::Result<()> {
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

fn dirs_in_dir(dir: &Path) -> io::Result<Vec<Box<Path>>> {
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

fn read_file_to_str_opt(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(Some(s)),
        Err(e) => match e.kind() {
            io::ErrorKind::InvalidData => Ok(None),
            _ => Err(e),
        },
    }
}

fn get_relative_path<'a>(path: &'a Path, base: &'a Path) -> &'a Path {
    path.strip_prefix(&base)
        .expect("Failed to strip path prefix")
}

pub fn load_versions(dir: &Path) -> io::Result<Vec<Version>> {
    let version_paths = dirs_in_dir(dir)?;
    let mut versions: Vec<Version> = Vec::new();

    for version_path in version_paths {
        let mut file_paths: Vec<Box<Path>> = Vec::new();
        walk_dir(&version_path, &mut file_paths);

        let mut files: HashMap<String, FileData> = HashMap::new();

        for file_path in file_paths {
            let file_rel_path = get_relative_path(&file_path, &version_path);
            files.insert(
                file_rel_path.to_string_lossy().to_string(),
                FileData {
                    text_content: read_file_to_str_opt(&file_path)?,
                },
            );
        }

        let version_rel_path = get_relative_path(&version_path, &dir);
        let version_name = version_rel_path.to_string_lossy().to_string();

        versions.push(Version {
            name: version_name,
            path: version_path,
            files,
        });
    }

    Ok(versions)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use tempdir::TempDir;

    use super::*;

    #[test]
    fn test_load_versions() {
        let tmp_dir = TempDir::new("test_temp").unwrap();
        let base = tmp_dir.path();

        fs::create_dir_all(base.join("version_1")).unwrap();
        fs::create_dir_all(base.join("version_2")).unwrap();
        fs::write(base.join("version_1/file_a.txt"), "file_a").unwrap();
        fs::write(base.join("version_1/file_b.txt"), "file_b").unwrap();
        fs::write(base.join("version_2/file_a.txt"), "file_a_new").unwrap();
        fs::write(base.join("version_2/file_b.txt"), "file_b_new").unwrap();

        let versions = load_versions(base).unwrap();

        assert_eq!(versions.len(), 2);

        let version_1 = &versions.iter().find(|v| v.name == "version_1").unwrap();
        assert_eq!(version_1.path, base.join("version_1").into());
        let files_1 = &version_1.files;
        assert_eq!(
            files_1["file_a.txt"].text_content.as_ref().unwrap(),
            "file_a"
        );
        assert_eq!(
            files_1["file_b.txt"].text_content.as_ref().unwrap(),
            "file_b"
        );

        let version_2 = &versions.iter().find(|v| v.name == "version_2").unwrap();
        assert_eq!(version_2.name, "version_2");
        assert_eq!(version_2.path, base.join("version_2").into());
        let files_2 = &version_2.files;
        assert_eq!(
            files_2["file_a.txt"].text_content.as_ref().unwrap(),
            "file_a_new"
        );
        assert_eq!(
            files_2["file_b.txt"].text_content.as_ref().unwrap(),
            "file_b_new"
        );

        tmp_dir.close().unwrap();
    }
}
