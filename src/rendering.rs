use crate::{
    diffing::text_diff_versions,
    types::{DiffInfo, TreeNode, Version},
};

pub fn produce_diff_tree(node: &TreeNode<Version>) -> TreeNode<DiffInfo> {
    fn f(v: &Version, parent: Option<&Version>) -> DiffInfo {
        match parent {
            Some(p) => {
                let diff = text_diff_versions(p, v);
                DiffInfo {
                    name: v.name.to_owned(),
                    added: diff.added_files.len(),
                    deleted: diff.deleted_files.len(),
                    modified: diff.modified_files.len(),
                }
            }
            None => DiffInfo {
                name: v.name.to_owned(),
                added: 0,
                deleted: 0,
                modified: 0,
            },
        }
    }
    node.map_with_parent(&f, None)
}

pub fn produce_label_tree(diff_tree: &TreeNode<DiffInfo>) -> TreeNode<String> {
    fn f(d: &DiffInfo) -> String {
        format!(
            "{} - FILES: {} A, {} D, {} M",
            d.name, d.added, d.deleted, d.modified,
        )
    }
    diff_tree.map(&f)
}
