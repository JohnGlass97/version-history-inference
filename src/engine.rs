use crate::{
    diffing::text_diff_versions,
    edmonds::find_msa,
    file_fetching::load_versions,
    types::{FileChange, TextChange, TextualVersionDiff, TreeNode, Version},
    utils::PB_BAR_STYLE,
};
use indicatif::{MultiProgress, ProgressBar};
use ndarray::Array2;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use similar::ChangeTag;
use std::{
    collections::HashMap,
    io,
    ops::AddAssign,
    path::Path,
    sync::Arc,
    thread::{self},
    time::Duration,
};

// Penalties
const ADD_FILE_P: f32 = 2.;
const DELETE_FILE_P: f32 = 4.;
const MODIFY_FILE_P: f32 = 1.;
const ADD_LINE_P: f32 = 0.02;
const DELETE_LINE_P: f32 = 0.05;

fn count_tag(changes: &Vec<TextChange>, tag: ChangeTag) -> usize {
    changes.iter().filter(|c| c.tag == tag).count()
}

struct Pair(f32, f32);

impl AddAssign for Pair {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

fn file_heuristic(file_change: &FileChange) -> Pair {
    let adds = count_tag(&file_change.changes, ChangeTag::Insert).min(50) as f32;
    let deletes = count_tag(&file_change.changes, ChangeTag::Delete).min(50) as f32;

    Pair(
        adds * ADD_LINE_P + deletes * DELETE_LINE_P,
        adds * DELETE_LINE_P + deletes * ADD_LINE_P,
    )
}

fn calculate_distances(text_diff: &TextualVersionDiff) -> Pair {
    let mut forward_backward = Pair(0., 0.);

    for file_change in &text_diff.added_files {
        forward_backward += Pair(ADD_FILE_P, DELETE_FILE_P);
        forward_backward += file_heuristic(file_change);
    }

    for file_change in &text_diff.deleted_files {
        forward_backward += Pair(DELETE_FILE_P, ADD_FILE_P);
        forward_backward += file_heuristic(file_change);
    }

    for file_change in &text_diff.modified_files {
        forward_backward += Pair(MODIFY_FILE_P, MODIFY_FILE_P);
        forward_backward += file_heuristic(file_change);
    }

    forward_backward
}

/// Combine tree in vector of parents form and data vector
/// to get TreeNode vector (a forest)
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

pub fn infer_version_tree(dir: &Path, mp: &MultiProgress) -> io::Result<TreeNode<Version>> {
    let mut versions = load_versions(dir, mp)?;

    let null_version = Version {
        name: "Empty".to_string(),
        path: Path::new(".").into(), // TODO: Is this safe?
        files: HashMap::new(),
    };
    versions.insert(0, null_version);

    let n = versions.len();

    let mut distances: Array2<f32> = Array2::zeros((n, n));

    let versions_arc = Arc::new(versions);

    let mut to_compare = vec![];
    for j in 1..n {
        for i in 0..j {
            to_compare.push((i, j));
        }
    }

    let cmp_pb = Arc::new(mp.add(ProgressBar::new(to_compare.len() as u64)));
    cmp_pb.set_style(PB_BAR_STYLE.clone());
    cmp_pb.set_prefix("Doing comparisons");
    cmp_pb.enable_steady_tick(Duration::from_millis(100));

    let results = to_compare
        .par_iter()
        .map(|&(i, j)| {
            let version_a = &versions_arc[i];
            let version_b = &versions_arc[j];

            let text_diff = text_diff_versions(version_a, version_b);
            cmp_pb.inc(1);
            (i, j, calculate_distances(&text_diff))
        })
        .collect::<Vec<_>>();

    for result in results {
        let (i, j, edge_pair) = result;
        let Pair(a_to_b, b_to_a) = edge_pair;

        distances[(i, j)] = a_to_b;
        distances[(j, i)] = b_to_a;
    }
    cmp_pb.finish();

    let versions = Arc::try_unwrap(versions_arc).unwrap();

    let msa = find_msa(distances.view(), 0);

    let mut data = versions.into_iter().map(|s| Some(s)).collect();
    let mut forest = assemble_forest(&msa, None, &mut data);

    assert_eq!(forest.len(), 1, "MSA is not tree");
    let tree = forest.remove(0);

    Ok(tree)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use dircpy::copy_dir;
    use render_as_tree::render;
    use std::fs;
    use tempdir::TempDir;

    use super::*;
    use crate::{rendering::produce_label_tree, test_utils::append_to_file};

    #[test]
    fn handcrafted_1() {
        let tmp_dir = TempDir::new("test_temp").unwrap();
        let base = tmp_dir.path();

        fs::create_dir_all(base.join("version_1")).unwrap();
        fs::write(base.join("version_1/file_a.txt"), "file_a\n").unwrap();
        fs::write(base.join("version_1/file_b.txt"), "file_b\n").unwrap();

        copy_dir(base.join("version_1"), base.join("version_2a")).unwrap();
        append_to_file(base.join("version_2a/file_a.txt"), "abc\n").unwrap();
        append_to_file(base.join("version_2a/file_b.txt"), "def\n").unwrap();

        copy_dir(base.join("version_1"), base.join("version_2b")).unwrap();
        append_to_file(base.join("version_2b/file_a.txt"), "123\n").unwrap();
        append_to_file(base.join("version_2b/file_b.txt"), "456\n").unwrap();

        copy_dir(base.join("version_2a"), base.join("version_3")).unwrap();
        append_to_file(base.join("version_3/file_a.txt"), "uvw\n").unwrap();
        append_to_file(base.join("version_3/file_b.txt"), "xyz\n").unwrap();

        let version_tree = infer_version_tree(base, &MultiProgress::new()).unwrap();
        let name_tree = version_tree.map(&|v: &Version| v.name.to_owned());

        let expected = TreeNode {
            value: "Empty".to_owned(),
            children: vec![TreeNode {
                value: "version_1".to_owned(),
                children: vec![
                    TreeNode {
                        value: "version_2a".to_owned(),
                        children: vec![TreeNode {
                            value: "version_3".to_owned(),
                            children: vec![],
                        }],
                    },
                    TreeNode {
                        value: "version_2b".to_owned(),
                        children: vec![],
                    },
                ],
            }],
        };

        assert_eq!(name_tree, expected);

        tmp_dir.close().unwrap();
    }

    #[test]
    fn handcrafted_2() {
        let tmp_dir = TempDir::new("test_temp").unwrap();
        let base = tmp_dir.path();

        fs::create_dir_all(base.join("version_1")).unwrap();
        fs::write(
            base.join("version_1/file_a.txt"),
            "This\nis the\nfirst\nversion\n",
        )
        .unwrap();

        fs::create_dir_all(base.join("version_2")).unwrap();
        fs::write(
            base.join("version_2/file_a.txt"),
            "This\nis\nthe\nsecond\nversion!\n",
        )
        .unwrap();

        fs::create_dir_all(base.join("version_3")).unwrap();
        fs::write(
            base.join("version_3/file_a.txt"),
            "Now\nthis\nis\nthe\nthird\nversion!\n",
        )
        .unwrap();

        fs::create_dir_all(base.join("version_4")).unwrap();
        fs::write(
            base.join("version_4/file_a.txt"),
            "Now\nthis\nis\nthe\nversion\nafter\nthe\nthird\n",
        )
        .unwrap();

        let version_tree = infer_version_tree(base, &MultiProgress::new()).unwrap();
        let name_tree = version_tree.map(&|v: &Version| v.name.to_owned());

        let expected = TreeNode {
            value: "Empty".to_owned(),
            children: vec![TreeNode {
                value: "version_1".to_owned(),
                children: vec![TreeNode {
                    value: "version_2".to_owned(),
                    children: vec![TreeNode {
                        value: "version_3".to_owned(),
                        children: vec![TreeNode {
                            value: "version_4".to_owned(),
                            children: vec![],
                        }],
                    }],
                }],
            }],
        };

        assert_eq!(name_tree, expected);

        tmp_dir.close().unwrap();
    }
}
