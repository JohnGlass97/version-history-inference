#![allow(unused)]

use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::Write,
};

use octocrab::Octocrab;
use render_as_tree::render;
use version_history_inference::{
    testing::forks::{build_fork_tree, split_full_name, VersionRef},
    types::TreeNode,
};

async fn find_forks(octo: &Octocrab, owner: &str, repo: &str) -> TreeNode<VersionRef> {
    let version_ref_tree = build_fork_tree(octo, owner, repo, 2, 2).await.unwrap();

    let label_tree = version_ref_tree.map(&|t| {
        format!(
            "{}/{} - {}: v{}",
            t.owner,
            t.repo,
            if t.is_head { "HEAD" } else { "OLD" },
            t.version_no
        )
    });
    print!("{}\n", render(&label_tree).join("\n"));

    version_ref_tree
}

#[tokio::main]
async fn main() {
    let token = env::args().nth(1).expect("Please provide access token");

    let octo = Octocrab::builder().personal_token(token).build().unwrap();

    let test_repo_file = File::open("./test_repos.json").unwrap();
    let list: serde_json::Value = serde_json::from_reader(test_repo_file).unwrap();

    let mut fork_trees: HashMap<String, TreeNode<VersionRef>> = HashMap::new();

    for full_name in list.as_array().expect("Expected list of strings") {
        let full_name = full_name.as_str().expect("Expected list of strings");
        let (owner, repo) = split_full_name(full_name);

        let tree = find_forks(&octo, owner, repo).await;
        fork_trees.insert(full_name.to_owned(), tree);
    }

    fs::create_dir_all("./test_repos");
    let mut file = File::create("./test_repos/fork_trees.json").unwrap();

    let fork_trees_json = serde_json::to_string(&fork_trees).unwrap();
    file.write_all(fork_trees_json.as_bytes()).unwrap();
}
