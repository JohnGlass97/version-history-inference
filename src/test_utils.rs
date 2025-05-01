use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::Path,
};

use crate::types::Version;

pub fn append_to_file<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let mut file = OpenOptions::new().append(true).open(path)?;

    file.write_all(contents.as_ref())
}
