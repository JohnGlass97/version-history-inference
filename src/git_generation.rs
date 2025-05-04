use std::{fs, path::Path};

use dircpy::copy_dir;
use futures::io;
use git2::{Commit, IndexAddOption, Repository, Signature};

use crate::types::{DiffInfo, TreeNode};

/// Git instruction
#[derive(Debug)]
pub enum GitI {
    /// Version name
    CreateCommit(String),
    /// Version name, branch name
    CreateBranch(String, String),
}

pub fn build_instruction_trees(version_tree: &TreeNode<DiffInfo>) -> Vec<TreeNode<GitI>> {
    fn inner(node: &TreeNode<DiffInfo>) -> (usize, String, String, Vec<TreeNode<GitI>>) {
        // Do recursive calls
        let mut children_tuples: Vec<_> = node.children.iter().map(inner).collect();

        // Choose the child with the deepest subtree to be the next commit on the same branch
        let Some((next_commit_idx, _)) = children_tuples
            .iter()
            .enumerate()
            .max_by_key(|&(_, (depth, _, _, _))| depth)
        else {
            // This node must have no children
            let depth = 0;
            // This version will be the head commit for its branch, use it to name the branch too
            let commit = &node.value.name;
            return (depth, commit.to_owned(), commit.to_owned(), vec![]);
        };

        // Use the branch name of the next commit for this branch too
        let (depth, next_commit, branch, next_children) =
            children_tuples.swap_remove(next_commit_idx);

        // First child of this version/commit is the next commit (this is essential so that
        // `gen_git_repo` knows where to find the one and only CreateCommit child and execute it last)
        let mut new_children = vec![TreeNode {
            value: GitI::CreateCommit(next_commit),
            children: next_children,
        }];

        // Other children are all branches as there can only be one next commit
        for (_, child_commit, child_branch, children) in children_tuples {
            new_children.push(TreeNode {
                value: GitI::CreateBranch(child_commit, child_branch),
                children,
            })
        }

        let commit = node.value.name.to_owned();
        (depth + 1, commit, branch, new_children)
    }

    // Version trees should always have "Empty" as root
    assert_eq!(version_tree.value.name, "Empty");

    version_tree
        .children
        .iter()
        .map(|tree| {
            let (_, commit, branch, children) = inner(tree);
            TreeNode {
                value: GitI::CreateBranch(commit, branch),
                children,
            }
        })
        .collect()
}

fn curr_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    repo.find_commit(repo.head()?.target().unwrap())
}

fn create_branch(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
    let branch = repo.branch(&branch_name, &curr_commit(&repo)?, false)?;
    repo.set_head(&format!("refs/heads/{branch_name}"))
}

fn goto_branch(repo: &Repository, branch_name: &str) -> Result<(), git2::Error> {
    repo.set_head(&format!("refs/heads/{branch_name}"))
}

fn copy_version(dir: &Path, repo_name: &str, version_name: &str) -> io::Result<()> {
    for entry in fs::read_dir(dir.join(repo_name))? {
        let path = entry?.path();

        if let Some(".git") = path.file_name().and_then(std::ffi::OsStr::to_str) {
            continue;
        }

        if path.is_file() {
            fs::remove_file(path)?;
        } else {
            fs::remove_dir_all(&path)?;
        }
    }

    copy_dir(dir.join(version_name), dir.join(repo_name))
}

fn gen_sig(repo: &Repository) -> Result<Signature, git2::Error> {
    let config = repo.config()?;
    let name = config.get_string("user.name")?;
    let email = config.get_string("user.email")?;
    Signature::now(&name, &email)
}

fn commit_all(repo: &Repository, message: &str, no_parent: bool) -> Result<(), git2::Error> {
    let mut index = repo.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree = &repo.find_tree(index.write_tree()?)?;
    let sig = &gen_sig(repo)?;
    let parents: &[&Commit<'_>] = if no_parent {
        &[]
    } else {
        &[&curr_commit(repo)?]
    };

    repo.commit(Some("HEAD"), sig, sig, message, tree, parents);
    Ok(())
}

fn io_to_git(e: io::Error) -> git2::Error {
    git2::Error::from_str(&format!("{e}"))
}

pub fn gen_git_repo(
    dir: &Path,
    instruction_trees: &Vec<TreeNode<GitI>>,
    repo_name: &str,
) -> Result<(), git2::Error> {
    if fs::exists(dir.join(repo_name)).map_err(io_to_git)? {
        return Err(git2::Error::from_str(&format!(
            "{} already exists",
            dir.join(repo_name).display()
        )));
    }

    let repo = Repository::init(dir.join(repo_name))?;
    commit_all(&repo, "Initial commit", true)?;

    fn execute_tree(
        node: &TreeNode<GitI>,
        repo: &Repository,
        orig_branch: &str,
        dir: &Path,
        repo_name: &str,
    ) -> Result<(), git2::Error> {
        goto_branch(&repo, orig_branch)?;

        let (version_name, curr_branch) = match &node.value {
            GitI::CreateCommit(v) => (v.as_str(), orig_branch),
            GitI::CreateBranch(v, branch_name) => {
                create_branch(repo, branch_name)?;
                (v.as_str(), branch_name.as_str())
            }
        };

        copy_version(dir, repo_name, &version_name).map_err(io_to_git)?;
        commit_all(&repo, &version_name, false)?;

        for child in node.children.iter().rev() {
            execute_tree(child, repo, curr_branch, dir, repo_name)?;
        }

        Ok(())
    }

    let head = repo.head()?;
    let main_branch_name = head
        .shorthand()
        .ok_or(git2::Error::from_str("Couldn't get branch name"))?;

    for tree in instruction_trees {
        execute_tree(tree, &repo, main_branch_name, dir, repo_name)?;
    }

    Ok(())
}
