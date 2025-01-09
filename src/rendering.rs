use crate::{
    diffing::text_diff_versions,
    types::{TreeNode, Version},
};

pub fn produce_label_tree(node: &TreeNode<Version>, parent: Option<&Version>) -> TreeNode<String> {
    let v = &node.value;
    TreeNode {
        value: match parent {
            Some(p) => {
                let diff = text_diff_versions(p, v);
                format!(
                    "{} - FILES: {} added, {} removed, {} modified",
                    v.name,
                    diff.added_files.len(),
                    diff.deleted_files.len(),
                    diff.modified_files.len()
                )
            }
            None => v.name.to_string(),
        },
        children: node
            .children
            .iter()
            .map(|c| produce_label_tree(c, Some(v)))
            .collect(),
    }
}
