use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

use crate::types::TreeNode;

pub struct UseTestTempDir;

impl Drop for UseTestTempDir {
    fn drop(&mut self) {
        fs::remove_dir_all("test_temp").unwrap();
    }
}

pub fn append_to_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(path)?;

    file.write_all(contents.as_ref())
}
