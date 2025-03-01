use crate::{
    diffing::text_diff_versions,
    types::{TreeNode, Version},
};

pub fn produce_label_tree(node: &TreeNode<Version>) -> TreeNode<String> {
    fn f(v: &Version, parent: Option<&Version>) -> String {
        match parent {
            Some(p) => {
                let diff = text_diff_versions(p, v);
                format!(
                    "{} - FILES: {} A, {} D, {} M",
                    v.name,
                    diff.added_files.len(),
                    diff.deleted_files.len(),
                    diff.modified_files.len()
                )
            }
            None => v.name.to_string(),
        }
    }
    node.map_with_parent(&f, None)
}
