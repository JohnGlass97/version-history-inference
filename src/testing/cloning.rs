use std::{fs, path::Path};

use dircpy::copy_dir;
use git2::{Oid, Repository};
use tempdir::TempDir;

#[derive(Hash, PartialEq, Eq)]
pub struct Commit {
    pub handle: String,
    pub name: String,
}

pub fn clone_commits_drop_git<P: AsRef<Path>>(repo_url: &str, commits: &Vec<Commit>, dest_root: P) {
    let tmp_dir = TempDir::new("test_temp").unwrap();
    let base = tmp_dir.path();

    println!("Cloning {repo_url}");
    let repo = Repository::clone(repo_url, base).unwrap();
    println!("DONE");

    for commit in commits {
        let oid = match Oid::from_str(&commit.handle) {
            Ok(oid) => oid,
            Err(_) => {
                let branch = repo
                    .find_reference(&format!("refs/remotes/origin/{}", commit.handle))
                    .unwrap();
                branch.peel_to_commit().unwrap().id()
            }
        };

        let commit_obj = repo.find_commit(oid).unwrap();
        let tree = commit_obj.tree().unwrap();

        repo.checkout_tree(tree.as_object(), None).unwrap();
        repo.set_head_detached(oid).unwrap();

        let dest = dest_root.as_ref().join(&commit.name);
        copy_dir(base, &dest).unwrap();
        fs::remove_dir_all(&dest.join(".git")).unwrap();
    }
}
