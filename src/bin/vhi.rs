#![allow(unused)]

use clap::{arg, command, value_parser, ArgAction, Command};
use indicatif::{HumanDuration, MultiProgress, ProgressBar};
use render_as_tree::render;
use std::io::Write;
use std::path::Path;
use std::process::exit;
use std::time::Instant;
use std::{fs::File, path::PathBuf};
use version_history_inference::file_fetching::{load_file_versions, load_versions};
use version_history_inference::{
    engine::infer_version_tree,
    rendering::{produce_diff_tree, produce_label_tree},
    utils::PB_SPINNER_STYLE,
};

#[derive(Debug)]
enum Config {
    /// directory, file extension, recursive
    Infer(PathBuf, Option<String>, bool),
}

fn parse_args() -> Config {
    let matches = command!()
        .subcommand_required(true)
        .subcommand(
            Command::new("infer")
                .about("Infer a version tree for the different versions represented by folders in the provided directory")
                .arg(
                    arg!(<dir> "Directory containing folders where each folder represents a version")
                    .id("dir")
                    .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    arg!(-f --"files-as-versions" <extension> "Treat individual files as versions instead, with the specified extension")
                    .id("ext")
                    .value_parser(value_parser!(String))
                )
                .arg(
                    arg!(-r --recursive "Search all subfolders (only applies to files-as-versions mode)").action(ArgAction::SetTrue)
                )
        )
        .get_matches();

    let submatches = matches.subcommand_matches("infer").unwrap();

    let dir = submatches.get_one::<PathBuf>("dir").unwrap().to_path_buf();
    let ext = submatches.get_one::<String>("ext").cloned();
    let recursive = submatches.get_flag("recursive");

    Config::Infer(dir, ext, recursive)
}

fn infer(dir: &Path, extension: Option<String>, recursive: bool) {
    // Progress tracking
    let mp = MultiProgress::new();
    let started = Instant::now();

    let versions = match extension {
        Some(ext) => load_file_versions(dir, &ext, recursive, &mp),
        None => load_versions(dir, &mp),
    }
    .unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(1);
    });

    let version_tree = infer_version_tree(versions, &mp);

    let save_spinner = mp.add(ProgressBar::new_spinner());
    save_spinner.set_style(PB_SPINNER_STYLE.clone());
    save_spinner.set_prefix("Saving tree");

    // Save tree to JSON file
    let mut file = File::create(&dir.join("version_tree.json")).unwrap();
    let diff_tree = produce_diff_tree(&version_tree);
    let diff_tree_json = serde_json::to_string(&diff_tree).unwrap();
    file.write_all(diff_tree_json.as_bytes()).unwrap();

    save_spinner.finish();
    println!("Done in {}\n", HumanDuration(started.elapsed()));

    let label_tree = produce_label_tree(&diff_tree);
    print!("{}", render(&label_tree).join("\n"));
}

fn main() {
    let Config::Infer(dir, ext, recursive) = parse_args();
    infer(&dir, ext, recursive);
}
