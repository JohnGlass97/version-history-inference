use std::{collections::HashMap, io, path::Path};

use ndarray::{arr2, Array2};
use similar::ChangeTag;

use crate::{
    diffing::text_diff_versions,
    edmonds::find_msa,
    file_fetching::load_versions,
    types::{TextChange, TextualVersionDiff, TreeNode, Version},
};

fn distance_heuristic(
    added: usize,
    deleted: usize,
    add_delete_changes: usize,
    modified: usize,
    modify_add_changes: usize,
    modify_delete_changes: usize,
) -> f32 {
    0.0 + (added as f32) * 2.0
        + (deleted as f32) * 4.0
        + (modified as f32) * 1.0
        + (add_delete_changes.min(50) as f32) * 0.05
        + (modify_add_changes.min(50) as f32) * 0.05
        + (modify_delete_changes.min(50) as f32) * 0.1
}

fn count_tag(changes: &Vec<TextChange>, tag: ChangeTag) -> usize {
    changes.iter().filter(|c| c.tag == tag).count()
}

fn calculate_distances(text_diff: &TextualVersionDiff) -> (f32, f32) {
    let added = text_diff.added_files.len();
    let deleted = text_diff.deleted_files.len();
    let add_delete_changes = text_diff.add_delete_changes.len();
    let modified = text_diff.modified_files.len();

    let modify_add_changes = count_tag(&text_diff.modify_changes, ChangeTag::Insert);
    let modify_delete_changes = count_tag(&text_diff.modify_changes, ChangeTag::Delete);

    let forwards = distance_heuristic(
        added,
        deleted,
        add_delete_changes,
        modified,
        modify_add_changes,
        modify_delete_changes,
    );
    let backwards = distance_heuristic(
        deleted,
        added,
        add_delete_changes,
        modified,
        modify_delete_changes,
        modify_add_changes,
    );

    (forwards, backwards)
}

fn assemble_forest<T>(
    parents: &Vec<Option<usize>>,
    parent: Option<usize>,
    data: &mut Vec<Option<T>>,
) -> Vec<TreeNode<T>> {
    let mut forest: Vec<TreeNode<T>> = Vec::new();
    for (this, p) in parents.iter().enumerate() {
        if *p == parent {
            forest.push(TreeNode {
                value: std::mem::replace(&mut data[this], None).unwrap(),
                children: assemble_forest(parents, Some(this), data),
            });
        }
    }
    forest
}

pub fn infer_version_tree(dir: &Path) -> io::Result<TreeNode<Version>> {
    let mut versions = load_versions(dir)?;

    let null_version = Version {
        name: "Empty".to_string(),
        path: Path::new(".").into(), // TODO: Is this safe?
        files: HashMap::new(),
    };
    versions.insert(0, null_version);

    let n = versions.len();
    let mut distances: Array2<f32> = Array2::zeros((n, n));

    for j in 1..n {
        for i in 0..j {
            let version_a = &versions[i];
            let version_b = &versions[j];

            let text_diff = text_diff_versions(version_a, version_b);

            let (a_to_b, b_to_a) = calculate_distances(&text_diff);

            distances[(i, j)] = a_to_b;
            distances[(j, i)] = b_to_a;
        }
    }

    let msa = find_msa(distances.view(), 0);

    let mut data = versions.into_iter().map(|s| Some(s)).collect();
    let mut forest = assemble_forest(&msa, None, &mut data);

    assert_eq!(forest.len(), 1, "MSA is not tree");
    let tree = forest.remove(0);

    Ok(tree)
}
