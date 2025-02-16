#![allow(unused)]

use std::{collections::HashMap, env, pin::Pin};

use octocrab::{models::Repository, params::repos::forks::Sort, Octocrab};
use render_as_tree::render;
use version_history_inference::types::TreeNode;

#[derive(Debug)]
struct VersionRef {
    owner: String,
    repo: String,
    commit: String,
    is_head: bool,
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

async fn build_fork_tree(
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

    for fork in forks {
        let fork_owner = &fork.owner.expect("No owner for repo").login;
        let fork_repo = &fork.name;

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
                assert_eq!(t.value.commit, sha);
                t.children.push(fork_tree);
            }
            None => {
                fork_tree_map.insert(
                    behind_by,
                    TreeNode {
                        value: VersionRef {
                            owner: owner.to_owned(),
                            repo: repo.to_owned(),
                            commit: sha,
                            is_head: behind_by == 0,
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

    let mut tree = ordered_fork_trees.next().unwrap();
    for mut fork_tree in ordered_fork_trees {
        fork_tree.children.insert(0, tree);
        tree = fork_tree;
    }

    Some(tree)
}

#[tokio::main]
async fn main() {
    let token = env::args().nth(1).expect("Please provide access token");

    let octo = Octocrab::builder().personal_token(token).build().unwrap();

    let version_ref_tree = build_fork_tree(&octo, "torvalds", "linux", 2, 2)
        .await
        .unwrap();

    let label_tree = version_ref_tree.map(&|t| {
        format!(
            "{}/{} - {}: {}",
            t.owner,
            t.repo,
            if t.is_head { "HEAD" } else { "OLD" },
            t.commit
        )
    });
    print!("{}", render(&label_tree).join("\n"));
}
