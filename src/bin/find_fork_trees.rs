#![allow(unused)]

use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Write,
};

use futures::future::join_all;
use octocrab::Octocrab;
use render_as_tree::render;
use version_history_inference::{
    evaluation::forks::{build_fork_tree, gen_version_display_name, split_full_name, VersionRef},
    types::TreeNode,
};

async fn find_forks(octo: &Octocrab, owner: &str, repo: &str) -> TreeNode<VersionRef> {
    let version_ref_tree = build_fork_tree(octo, owner, repo, 2, 3).await.unwrap();

    let label_tree = version_ref_tree.map(&|t| gen_version_display_name(t));
    print!("{}\n", render(&label_tree).join("\n"));

    version_ref_tree
}

#[tokio::main]
async fn main() {
    let token = env::args().nth(1).expect("Please provide access token");

    let octo = Octocrab::builder().personal_token(token).build().unwrap();

    let test_repo_file = File::open("./test_repos.json").unwrap();
    let list: serde_json::Value = serde_json::from_reader(test_repo_file).unwrap();

    let mut fork_tree_futures = vec![];

    for full_name in list.as_array().expect("Expected list of strings") {
        let full_name = full_name.as_str().expect("Expected list of strings");
        let (owner, repo) = split_full_name(full_name);

        let tree_future = find_forks(&octo, owner, repo);
        fork_tree_futures.push(async move { (format!("{repo}-forks"), tree_future.await) });
    }

    let fork_trees: HashMap<String, TreeNode<VersionRef>> =
        join_all(fork_tree_futures).await.into_iter().collect();

    fs::create_dir_all("./test_repos");
    let mut file = File::create("./test_repos/fork_trees.json").unwrap();

    let fork_trees_json = serde_json::to_string(&fork_trees).unwrap();
    file.write_all(fork_trees_json.as_bytes()).unwrap();
}
