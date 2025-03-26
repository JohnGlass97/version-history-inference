use std::collections::HashMap;

use octocrab::{models::Repository, params::repos::forks::Sort, Octocrab};
use serde::{Deserialize, Serialize};

use crate::types::TreeNode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRef {
    pub owner: String,
    pub repo: String,
    pub commit: String,
    pub is_head: bool,
    pub version_no: u8,
}

async fn get_forks(octo: &Octocrab, owner: &str, repo: &str, n: u8) -> Vec<Repository> {
    println!("Fetching {n} forks for {owner}/{repo}");

    octo.repos(owner, repo)
        .list_forks()
        .sort(Sort::Stargazers)
        .per_page(n)
        .send()
        .await
        .expect("Failed to fetch forks")
        .items
}

async fn find_fork_point(
    octo: &Octocrab,
    parent_owner: &str,
    parent_repo: &str,
    child_owner: &str,
) -> Option<(u64, String)> {
    println!("Finding fork point for {parent_owner}/{parent_repo} and child {child_owner}");

    let base = format!("{parent_owner}:HEAD");
    let head = format!("{child_owner}:HEAD");

    let comparison = octo
        .commits(parent_owner, parent_repo)
        .compare(base, head)
        .send()
        .await
        .ok();

    let comparison = match comparison {
        Some(c) => c,
        None => {
            println!("WARNING: Discarding {child_owner}/{parent_repo} (failed to find fork point)");
            return None;
        }
    };

    let behind_by = comparison.behind_by;
    assert!(behind_by >= 0);

    Some((behind_by as u64, comparison.merge_base_commit.sha))
}

async fn get_head_commit(octo: &Octocrab, owner: &str, repo: &str) -> Option<String> {
    println!("Finding head commit for {owner}/{repo}");

    let commits = octo
        .repos(owner, repo)
        .list_commits()
        .per_page(1)
        .send()
        .await
        .ok();

    match commits {
        Some(c) => Some(c.into_iter().next().expect("No commits found").sha),
        None => {
            println!("WARNING: Discarding {owner}/{repo} (failed to find head commit)");
            None
        }
    }
}

pub async fn build_fork_tree(
    octo: &Octocrab,
    owner: &str,
    repo: &str,
    depth: u8,
    breadth: u8,
) -> Option<TreeNode<VersionRef>> {
    let head_sha = get_head_commit(octo, owner, repo).await?;

    let head_tree_node = TreeNode {
        value: VersionRef {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
            commit: head_sha,
            is_head: true,
            version_no: 1,
        },
        children: vec![],
    };

    if depth == 0 {
        return Some(head_tree_node);
    }

    let forks = get_forks(octo, owner, repo, breadth).await;
    let mut fork_tree_map: HashMap<u64, TreeNode<VersionRef>> = HashMap::new();

    println!(
        "Forks: {:?}",
        forks
            .iter()
            .map(|r| r.full_name.as_ref().unwrap().to_owned())
            .collect::<Vec<String>>()
    );

    // HEAD is not a fork, but easier to treat it the same as one
    fork_tree_map.insert(0, head_tree_node);

    // Find fork point of each fork and recurse through its forks
    for fork in forks {
        let fork_owner = &fork.owner.expect("No owner for repo").login;
        let fork_repo = &fork.name;

        // behind_by use for ordering the fork points
        let fp = find_fork_point(octo, owner, repo, fork_owner).await;
        let (behind_by, sha) = match fp {
            Some((x, y)) => (x, y),
            None => continue,
        };

        let fork_tree = match Box::pin(build_fork_tree(
            octo,
            fork_owner,
            fork_repo,
            depth - 1,
            breadth,
        ))
        .await
        {
            Some(t) => t,
            None => continue,
        };

        match fork_tree_map.get_mut(&behind_by) {
            Some(t) => {
                // Fork point for current fork in loop matches the
                // fork point of a previous iteration
                assert_eq!(t.value.commit, sha);
                t.children.push(fork_tree);
            }
            None => {
                // New fork point
                fork_tree_map.insert(
                    behind_by,
                    TreeNode {
                        value: VersionRef {
                            owner: owner.to_owned(),
                            repo: repo.to_owned(),
                            commit: sha,
                            is_head: behind_by == 0,
                            version_no: 1,
                        },
                        children: vec![fork_tree],
                    },
                );
            }
        }
    }

    let mut map_pairs: Vec<(u64, TreeNode<VersionRef>)> = fork_tree_map.into_iter().collect();
    map_pairs.sort_by_key(|(x, _)| *x);

    let mut ordered_fork_trees = map_pairs.into_iter().map(|(_, t)| t);

    let mut next_version_no = ordered_fork_trees.len() as u8;
    let mut tree = ordered_fork_trees.next().unwrap();

    // Number the different fork points
    for mut fork_tree in ordered_fork_trees {
        tree.value.version_no = next_version_no;
        next_version_no -= 1;

        fork_tree.children.insert(0, tree);
        tree = fork_tree;
    }

    Some(tree)
}

pub fn gen_version_name(version_ref: &VersionRef) -> String {
    format!(
        "{}-{}-{}-v{}",
        version_ref.owner,
        version_ref.repo,
        if version_ref.is_head { "HEAD" } else { "OLD" },
        version_ref.version_no,
    )
}

pub fn gen_version_display_name(version_ref: &VersionRef) -> String {
    format!(
        "{}/{} - {}: v{}",
        version_ref.owner,
        version_ref.repo,
        if version_ref.is_head { "HEAD" } else { "OLD" },
        version_ref.version_no
    )
}

pub fn split_full_name(full_name: &str) -> (&str, &str) {
    let [owner, repo] = full_name
        .split("/")
        .collect::<Vec<_>>()
        .try_into()
        .expect("Repo full names must have exactly one '/'");
    (owner, repo)
}
