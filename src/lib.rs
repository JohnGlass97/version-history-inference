#![allow(unused)]

use std::path::{Path, PathBuf};

use clap::{arg, command, value_parser, Command};
use engine::infer_version_tree;
use render_as_tree::render;
use rendering::produce_label_tree;

pub mod diffing;
pub mod edmonds;
pub mod engine;
pub mod file_fetching;
pub mod rendering;
pub mod types;

#[cfg(test)]
pub mod test_utils;
