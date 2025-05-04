use std::collections::HashMap;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::types::{FileData, Version};
use crate::utils::PB_BAR_STYLE;

fn walk_dir(
    dir: &Path,
    file_paths: &mut Vec<Box<Path>>,
    extension: Option<&str>,
    recursive: bool,
) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if recursive {
                    walk_dir(&path, file_paths, extension, true)?;
                }
            } else {
                if let Some(expected_ext) = extension {
                    let Some(actual_ext) = path.extension().or(path.file_name()) else {
                        continue;
                    };
                    if actual_ext.to_string_lossy() != expected_ext {
                        continue;
                    }
                }
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

pub fn load_versions(
    dir: &Path,
    multithreading: bool,
    mp: &MultiProgress,
) -> io::Result<Vec<Version>> {
    let version_paths = dirs_in_dir(dir)?;
    let mut versions: Vec<Version> = Vec::new();

    let pb = mp.add(ProgressBar::new(version_paths.len() as u64));
    pb.set_style(PB_BAR_STYLE.clone());
    pb.set_prefix("Loading versions");
    pb.enable_steady_tick(Duration::from_millis(100));

    for (i, version_path) in version_paths.into_iter().enumerate() {
        let mut file_paths: Vec<Box<Path>> = Vec::new();
        walk_dir(&version_path, &mut file_paths, None, true)?;

        let version_pb = Arc::new(mp.add(ProgressBar::new(file_paths.len() as u64)));
        version_pb.set_style(PB_BAR_STYLE.clone());
        version_pb.set_prefix(format!("Version {}", i + 1));

        let map_op = |file_path: &Box<Path>| {
            let file_rel_path = get_relative_path(&file_path, &version_path);
            let file_data = FileData {
                text_content: read_file_to_str_opt(&file_path)?,
            };
            version_pb.inc(1);
            Ok((file_rel_path.to_string_lossy().to_string(), file_data))
        };

        let files: HashMap<String, FileData> = if multithreading {
            file_paths
                .par_iter()
                .map(map_op)
                .collect::<io::Result<_>>()?
        } else {
            file_paths.iter().map(map_op).collect::<io::Result<_>>()?
        };

        let version_rel_path = get_relative_path(&version_path, &dir);
        let version_name = version_rel_path.to_string_lossy().to_string();

        versions.push(Version {
            name: version_name,
            path: version_path,
            files,
        });

        pb.inc(1);
    }

    pb.finish();

    Ok(versions)
}

pub fn load_file_versions(
    dir: &Path,
    extension: &str,
    recursive: bool,
    multithreading: bool,
    mp: &MultiProgress,
) -> io::Result<Vec<Version>> {
    let norm_ext = extension.strip_prefix(".").unwrap_or(extension);

    let mut file_paths = vec![];
    walk_dir(dir, &mut file_paths, Some(norm_ext), recursive)?;

    let pb = mp.add(ProgressBar::new(file_paths.len() as u64));
    pb.set_style(PB_BAR_STYLE.clone());
    pb.set_prefix("Loading versions");
    pb.enable_steady_tick(Duration::from_millis(100));

    let files: Vec<Version> = file_paths
        .par_iter()
        .map(|file_path| {
            let version_rel_path = get_relative_path(&file_path, &dir);
            let version_name = version_rel_path.to_string_lossy().to_string();

            let file_data = FileData {
                text_content: read_file_to_str_opt(&file_path)?,
            };
            pb.inc(1);
            Ok(Version {
                name: version_name,
                path: file_path.clone(),
                files: HashMap::from([("main".to_string(), file_data)]),
            })
        })
        .collect::<io::Result<_>>()?;

    pb.finish();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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

        let versions = load_versions(base, true, &MultiProgress::new()).unwrap();

        assert_eq!(versions.len(), 2);

        let version_1 = versions.iter().find(|v| v.name == "version_1").unwrap();
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

        let version_2 = versions.iter().find(|v| v.name == "version_2").unwrap();
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

    #[test]
    fn test_load_file_versions() {
        let tmp_dir = TempDir::new("test_temp").unwrap();
        let base = tmp_dir.path();

        fs::create_dir_all(base.join("dir")).unwrap();
        fs::write(base.join("file_a.txt"), "file_a").unwrap();
        fs::write(base.join("dir/file_b.txt"), "file_b").unwrap();
        fs::write(base.join("excluded.abc"), "excluded").unwrap();

        let mp = MultiProgress::new();
        let versions = load_file_versions(base, "txt", true, true, &mp).unwrap();

        assert_eq!(versions.len(), 2);

        let v1 = versions.iter().find(|v| v.name == "file_a.txt").unwrap();
        assert_eq!(v1.path, base.join("file_a.txt").into());
        assert_eq!(v1.files["main"].text_content.as_ref().unwrap(), "file_a");

        let v2 = versions
            .iter()
            .find(|v| v.name == PathBuf::from("dir").join("file_b.txt").to_string_lossy())
            .unwrap();
        assert_eq!(v2.path, base.join("dir/file_b.txt").into());
        assert_eq!(v2.files["main"].text_content.as_ref().unwrap(), "file_b");

        tmp_dir.close().unwrap();
    }
}
