#![allow(unused)]

use std::path::Path;

use engine::infer_version_tree;
use render_as_tree::render;
use rendering::produce_label_tree;

mod diffing;
mod edmonds;
mod engine;
mod file_fetching;
mod rendering;
mod types;

fn main() {
    let version_tree = infer_version_tree(Path::new("test_temp")).unwrap();

    let label_tree = produce_label_tree(&version_tree, None);
    print!("{}", render(&label_tree).join("\n"));
}
