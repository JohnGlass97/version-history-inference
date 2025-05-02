use crate::types::{DiffInfo, TreeNode};

/// Git instruction
#[derive(Debug)]
pub enum GitI {
    /// Commit message
    CreateCommit(String),
    /// Commit message, branch name
    CreateBranch(String, String),
}

pub fn build_instruction_trees(version_tree: &TreeNode<DiffInfo>) -> Vec<TreeNode<GitI>> {
    fn inner(node: &TreeNode<DiffInfo>) -> (usize, String, String, Vec<TreeNode<GitI>>) {
        // Do recursive calls
        let mut children_tuples: Vec<_> = node.children.iter().map(inner).collect();

        // Choose the child with the deepest subtree to be the next commit on the same branch
        let Some((next_commit_idx, _)) = children_tuples.iter().enumerate().max_by_key(|&t| t.0)
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

        // First child of this version/commit is the next commit
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
