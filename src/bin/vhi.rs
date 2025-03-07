#![allow(unused)]

use clap::{arg, command, value_parser, Command};
use render_as_tree::render;
use std::path::{Path, PathBuf};

use version_history_inference::{engine::infer_version_tree, rendering::produce_label_tree};

fn parse_args() -> PathBuf {
    let matches = command!()
        .subcommand(
            Command::new("infer")
                .about("Infer a version tree for the different versions represented by folders in the provided directory")
                .arg(
                    arg!(<dir> "Directory containing folders where each folder represents a version")
                    .id("dir")
                    .value_parser(value_parser!(PathBuf)),
                )
        )
        .get_matches();

    let submatches = matches.subcommand_matches("infer").unwrap();
    submatches.get_one::<PathBuf>("dir").unwrap().to_path_buf()
}

fn main() {
    let dir = parse_args();

    let version_tree = infer_version_tree(&dir).unwrap();

    let label_tree = produce_label_tree(&version_tree);
    print!("{}", render(&label_tree).join("\n"));
}
