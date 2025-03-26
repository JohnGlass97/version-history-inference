#![allow(unused)]

use std::{
    collections::{HashMap, HashSet},
    fs::{self},
};

use version_history_inference::{
    testing::{
        cloning::{clone_commits_drop_git, Commit},
        forks::{gen_version_name, split_full_name, VersionRef},
    },
    types::TreeNode,
};

async fn clone_forks(root_repo: String, version_ref_tree: TreeNode<VersionRef>) {
    // HashSet shouldn't be needed as commits shouldn't
    // be duplicated but better to be safe
    let mut commits_hash_map: HashMap<String, HashSet<Commit>> = HashMap::new();
    let mut stack = vec![version_ref_tree.clone()];

    // Depth first search of fork tree to get all commits that need to be cloned
    while !stack.is_empty() {
        let node = stack.pop().unwrap();
        let sha = &node.value.commit;
        let commit = Commit {
            handle: sha.to_owned(),
            name: gen_version_name(&node.value),
        };

        let key = format!("{}/{}", node.value.owner, node.value.repo);
        match commits_hash_map.get_mut(&key) {
            Some(set) => {
                set.insert(commit);
            }
            None => {
                commits_hash_map.insert(key, HashSet::from([commit]));
            }
        }

        stack.extend(node.children);
    }

    for (repo_full_name, commit_set) in commits_hash_map.into_iter() {
        let url = format!("https://github.com/{repo_full_name}.git");
        let commits: Vec<Commit> = commit_set.into_iter().collect();
        clone_commits_drop_git(&url, &commits, format!("./test_repos/{root_repo}-forks"));
    }
}

#[tokio::main]
async fn main() {
    let fork_trees_json = fs::read_to_string("./test_repos/fork_trees.json").unwrap();
    let fork_trees: HashMap<String, TreeNode<VersionRef>> =
        serde_json::from_str(&fork_trees_json).unwrap();

    for (full_name, fork_tree) in fork_trees {
        let root_repo = split_full_name(&full_name).1.to_owned();
        clone_forks(root_repo, fork_tree).await;
    }
}
