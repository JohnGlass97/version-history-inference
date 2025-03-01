#![allow(unused)]

use std::{
    collections::{HashMap, HashSet},
    env,
    hash::Hash,
    pin::Pin,
};

use octocrab::{models::Repository, params::repos::forks::Sort, Octocrab};
use render_as_tree::render;
use similar::DiffableStr;
use version_history_inference::{
    testing::{
        cloning::{clone_commits_drop_git, Commit},
        forks::build_fork_tree,
    },
    types::TreeNode,
};

#[tokio::main]
async fn main() {
    let token = env::args().nth(1).expect("Please provide access token");

    let octo = Octocrab::builder().personal_token(token).build().unwrap();

    let version_ref_tree = build_fork_tree(&octo, "ocornut", "imgui", 2, 2)
        .await
        .unwrap();

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

    // HashSet shouldn't be needed as commits shouldn't
    // be duplicated but better to be safe
    let mut commits_hash_map: HashMap<String, HashSet<Commit>> = HashMap::new();
    let mut stack = vec![version_ref_tree];

    while !stack.is_empty() {
        let node = stack.pop().unwrap();
        let key = format!("{}/{}", node.value.owner, node.value.repo);
        let sha = node.value.commit;
        let commit = Commit {
            handle: sha.to_owned(),
            name: format!(
                "{}-{}-v{}",
                key.replace("/", "-"),
                if node.value.is_head { "HEAD" } else { "OLD" },
                node.value.version_no,
            ),
        };

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
        clone_commits_drop_git(&url, &commits, "temp/imgui-forks");
    }
}
