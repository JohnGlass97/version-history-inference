#![allow(unused)]

use clap::{arg, command, value_parser, ArgAction, Command};
use indicatif::{HumanDuration, MultiProgress, ProgressBar};
use render_as_tree::render;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::exit,
    time::Duration,
};
use version_history_inference::{
    git_generation::{build_instruction_trees, gen_git_repo, GitI},
    inference::{
        engine::infer_version_tree,
        file_fetching::{load_file_versions, load_versions},
    },
    types::{DiffInfo, TreeNode},
    utils::{produce_label_tree, InferencePerformanceTracker, PB_SPINNER_STYLE},
};

#[derive(Debug)]
enum Config {
    /// directory, file extension, recursive, multithreading, trace_perf filename, dry_run
    Infer(PathBuf, Option<String>, bool, bool, Option<String>, bool),
    /// directory
    View(PathBuf),
    /// directory, name
    GitGen(PathBuf, String),
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
                    arg!(-p --"trace-performance" <filename> "Produce a JSON file with runtime duration information")
                    .id("trace-perf")
                    .value_parser(value_parser!(String))
                )
                .arg(
                    arg!(-d --"dry-run" "Skip creation of version_tree.json").action(ArgAction::SetTrue)
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
        .subcommand(
            Command::new("git-gen")
                .about("Generate a Git repo from with the structure of the previously produced version tree for the specified directory")
                .arg(
                    arg!(<dir> "Directory containing version_tree.json and matching versions")
                    .id("dir")
                    .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    arg!(<name> "Name for the Git repo, this will be placed in the provided directory")
                    .id("name")
                    .value_parser(value_parser!(String)),
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("infer", submatches)) => {
            let dir = submatches.get_one::<PathBuf>("dir").unwrap().to_path_buf();
            let ext = submatches.get_one::<String>("ext").cloned();
            let recursive = submatches.get_flag("recursive");
            let multithreading = !submatches.get_flag("no-multithreading");
            let trace_perf = submatches.get_one::<String>("trace-perf").cloned();
            let dry_run = submatches.get_flag("dry-run");

            Config::Infer(dir, ext, recursive, multithreading, trace_perf, dry_run)
        }
        Some(("view", submatches)) => {
            let dir = submatches.get_one::<PathBuf>("dir").unwrap().to_path_buf();

            Config::View(dir)
        }
        Some(("git-gen", submatches)) => {
            let dir = submatches.get_one::<PathBuf>("dir").unwrap().to_path_buf();
            let name = submatches.get_one::<String>("name").unwrap().to_owned();

            Config::GitGen(dir, name)
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
    trace_perf: Option<String>,
    dry_run: bool,
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
    let version_tree = infer_version_tree(versions, multithreading, &mp);
    perf_tracker.done_inferring();

    // Save tree
    let save_spinner = mp.add(ProgressBar::new_spinner());
    save_spinner.set_style(PB_SPINNER_STYLE.clone());
    save_spinner.set_prefix("Saving tree");
    save_spinner.enable_steady_tick(Duration::from_millis(100));

    if !dry_run {
        save_version_tree(&dir, &version_tree).unwrap_or_else(|e| {
            eprintln!("Failed to save version tree: {e}");
            exit(1);
        });
    }
    perf_tracker.done_saving();

    save_spinner.finish();
    println!("Done in {}\n", HumanDuration(perf_tracker.elapsed()));

    // Output tree
    let label_tree = produce_label_tree(&version_tree);
    println!("{}", render(&label_tree).join("\n"));

    // Save performance trace
    if let Some(filename) = trace_perf {
        perf_tracker.finished(filename).unwrap_or_else(|e| {
            eprintln!("Failed to save performance trace: {e}");
            exit(1);
        });
    }
}

fn load_version_tree(dir: &Path) -> TreeNode<DiffInfo> {
    let version_tree_json = fs::read_to_string(dir.join("version_tree.json")).unwrap_or_else(|e| {
        eprintln!("Couldn't load version_tree.json from the specified directory: {e}");
        exit(1);
    });
    let version_tree: TreeNode<DiffInfo> =
        serde_json::from_str(&version_tree_json).unwrap_or_else(|e| {
            eprintln!("version_tree.json is malformed, maybe rerun the infer command: {e}");
            exit(1);
        });
    version_tree
}

fn view(dir: &Path) {
    let version_tree = load_version_tree(dir);

    // Output tree
    let label_tree = produce_label_tree(&version_tree);
    println!("{}", render(&label_tree).join("\n"));
}

fn git_gen(dir: &Path, name: &str) {
    let version_tree = load_version_tree(dir);

    let instruction_trees = build_instruction_trees(&version_tree);
    gen_git_repo(dir, &instruction_trees, name).unwrap_or_else(|e| {
        eprintln!("Failed to generate Git repository: {e}");
        exit(1);
    });

    // let tree = &TreeNode {
    //     value: GitI::CreateCommit(format!("Initial commit")),
    //     children: instruction_trees,
    // };

    // // Output tree
    // let label_tree = tree.map(&|i| format!("{i:?}"));
    // println!("{}", render(&label_tree).join("\n"));
    println!("Done!");
}

fn main() {
    match parse_args() {
        Config::Infer(dir, ext, recursive, multithreading, trace_perf, dry_run) => {
            infer(&dir, ext, recursive, multithreading, trace_perf, dry_run)
        }
        Config::View(dir) => view(&dir),
        Config::GitGen(dir, name) => git_gen(&dir, &name),
    };
}
