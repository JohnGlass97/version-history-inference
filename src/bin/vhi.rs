#![allow(unused)]

use clap::{arg, command, value_parser, ArgAction, Command};
use indicatif::{HumanDuration, MultiProgress, ProgressBar};
use render_as_tree::render;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::exit,
};
use version_history_inference::{
    engine::infer_version_tree,
    file_fetching::{load_file_versions, load_versions},
    rendering::{produce_diff_tree, produce_label_tree},
    test_utils::InferencePerformanceTracker,
    types::{DiffInfo, TreeNode},
    utils::PB_SPINNER_STYLE,
};

#[derive(Debug)]
enum Config {
    /// directory, file extension, recursive, multithreading, trace_perf
    Infer(PathBuf, Option<String>, bool, bool, bool),
    /// directory
    View(PathBuf),
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
                .arg(
                    arg!(--"no-multithreading" "Disable multithreading").action(ArgAction::SetTrue)
                )
                .arg(
                    arg!(-p --"trace-performance" "Produce a JSON file with runtime duration information").action(ArgAction::SetTrue)
                )
        )
        .subcommand(
            Command::new("view")
                .about("View the previously produced version tree for the specified directory")
                .arg(
                    arg!(<dir> "Directory containing version_tree.json")
                    .id("dir")
                    .value_parser(value_parser!(PathBuf)),
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("infer", submatches)) => {
            let dir = submatches.get_one::<PathBuf>("dir").unwrap().to_path_buf();
            let ext = submatches.get_one::<String>("ext").cloned();
            let recursive = submatches.get_flag("recursive");
            let multithreading = !submatches.get_flag("no-multithreading");
            let trace_perf = submatches.get_flag("trace-performance");

            Config::Infer(dir, ext, recursive, multithreading, trace_perf)
        }
        Some(("view", submatches)) => {
            let dir = submatches.get_one::<PathBuf>("dir").unwrap().to_path_buf();

            Config::View(dir)
        }
        _ => panic!("Command not recognised"), // This shouldn't happen with .subcommand_required(true)
    }
}

fn save_version_tree(dir: &Path, diff_tree: &TreeNode<DiffInfo>) -> Result<(), String> {
    let file = File::create(dir.join("version_tree.json")).map_err(|e| format!("{e}"))?;
    serde_json::to_writer(file, &diff_tree).map_err(|e| format!("{e}"))
}

fn infer(
    dir: &Path,
    extension: Option<String>,
    recursive: bool,
    multithreading: bool,
    trace_perf: bool,
) {
    // Progress tracking
    let mp = MultiProgress::new();
    let mut perf_tracker = InferencePerformanceTracker::new(dir);

    // Load versions
    let versions = match extension {
        Some(ext) => load_file_versions(dir, &ext, recursive, multithreading, &mp),
        None => load_versions(dir, multithreading, &mp),
    }
    .unwrap_or_else(|e| {
        eprintln!("Failed to load versions: {e}");
        exit(1);
    });
    perf_tracker.done_loading(&versions);

    // Infer version tree
    let version_tree = infer_version_tree(versions, &mp);
    perf_tracker.done_inferring();

    // Save tree
    let save_spinner = mp.add(ProgressBar::new_spinner());
    save_spinner.set_style(PB_SPINNER_STYLE.clone());
    save_spinner.set_prefix("Saving tree");

    let diff_tree = produce_diff_tree(&version_tree);
    save_version_tree(&dir, &diff_tree).unwrap_or_else(|e| {
        eprintln!("Failed to save version tree: {e}");
        exit(1);
    });
    perf_tracker.done_saving();

    save_spinner.finish();
    println!("Done in {}\n", HumanDuration(perf_tracker.elapsed()));

    // Output tree
    let label_tree = produce_label_tree(&diff_tree);
    print!("{}", render(&label_tree).join("\n"));

    // Save performance trace
    if (trace_perf) {
        perf_tracker.finished().unwrap_or_else(|e| {
            eprintln!("\nFailed to save performance trace: {e}");
            exit(1);
        });
    }
}

fn view(dir: &Path) {
    // Load tree
    let version_tree_json = fs::read_to_string(dir.join("version_tree.json")).unwrap_or_else(|e| {
        eprintln!("Couldn't load version_tree.json from the specified directory: {e}");
        exit(1);
    });
    let version_tree: TreeNode<DiffInfo> =
        serde_json::from_str(&version_tree_json).unwrap_or_else(|e| {
            eprintln!("version_tree.json is malformed, maybe rerun the infer command: {e}");
            exit(1);
        });

    // Output tree
    let label_tree = produce_label_tree(&version_tree);
    print!("{}", render(&label_tree).join("\n"));
}

fn main() {
    match parse_args() {
        Config::Infer(dir, ext, recursive, multithreading, trace_perf) => {
            infer(&dir, ext, recursive, multithreading, trace_perf)
        }
        Config::View(dir) => view(&dir),
    };
}
