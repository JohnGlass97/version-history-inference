use crate::{
    inference::{
        diffing::text_diff_versions,
        edmonds::{assemble_forest, find_msa},
        file_fetching::load_versions,
    },
    types::{
        DiffInfo, DivCalcResult, FileChange, Pair, TextChange, TextualVersionDiff, TreeNode,
        Version,
    },
    utils::PB_BAR_STYLE,
};
use indicatif::{MultiProgress, ProgressBar};
use ndarray::Array2;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use similar::ChangeTag;
use std::{
    collections::HashMap,
    io,
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

fn file_heuristic(file_change: &FileChange) -> Pair {
    let adds = count_tag(&file_change.changes, ChangeTag::Insert).min(50) as f32;
    let deletes = count_tag(&file_change.changes, ChangeTag::Delete).min(50) as f32;

    Pair(
        adds * ADD_LINE_P + deletes * DELETE_LINE_P,
        adds * DELETE_LINE_P + deletes * ADD_LINE_P,
    )
}

pub fn calculate_divergences(text_diff: &TextualVersionDiff) -> (DivCalcResult, DivCalcResult) {
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

    let added = text_diff.added_files.len();
    let deleted = text_diff.deleted_files.len();
    let modified = text_diff.modified_files.len();

    let forward = DivCalcResult {
        added,
        deleted,
        modified,
        divergence: forward_backward.0,
    };

    let backward = DivCalcResult {
        added: deleted,
        deleted: added,
        modified,
        divergence: forward_backward.1,
    };

    (forward, backward)
}

pub fn infer_version_tree(
    mut versions: Vec<Version>,
    multithreading: bool,
    mp: &MultiProgress,
) -> TreeNode<DiffInfo> {
    let null_version = Version {
        name: "Empty".to_string(),
        path: Path::new(".").into(), // TODO: Is this safe?
        files: HashMap::new(),
    };
    versions.insert(0, null_version);

    let n = versions.len();

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

    let map_op = |&(i, j): &(usize, usize)| {
        let version_a = &versions_arc[i];
        let version_b = &versions_arc[j];

        let text_diff = text_diff_versions(version_a, version_b);
        cmp_pb.inc(1);
        (i, j, calculate_divergences(&text_diff))
    };

    let results = if multithreading {
        to_compare.par_iter().map(map_op).collect::<Vec<_>>()
    } else {
        to_compare.iter().map(map_op).collect::<Vec<_>>()
    };

    let mut divergences: Array2<f32> = Array2::zeros((n, n));

    // Will use full DivCalcResult for producing DiffInfo tree
    let default_res = DivCalcResult::new();
    let empty_res_vec = vec![default_res; n * n];
    let mut div_calc_res: Array2<DivCalcResult> =
        Array2::from_shape_vec((n, n), empty_res_vec).unwrap();

    for result in results {
        let (i, j, (forward, backward)) = result;

        divergences[(i, j)] = forward.divergence;
        divergences[(j, i)] = backward.divergence;

        div_calc_res[(i, j)] = forward;
        div_calc_res[(j, i)] = backward;
    }
    cmp_pb.finish();

    let versions = Arc::try_unwrap(versions_arc).unwrap();

    let msa = find_msa(divergences.view(), 0);
    let mut forest = assemble_forest(&msa, None);

    assert_eq!(forest.len(), 1, "MSA is not tree");
    let tree = forest.remove(0);

    // Convert tree of indexes to DiffInfo tree
    tree.map_with_parent(
        &|&i, parent| {
            // (i, i) will just give a null difference (all zeroes)
            let p = parent.cloned().unwrap_or(i);
            let forward = div_calc_res[(p, i)];
            DiffInfo {
                name: versions[i].name.to_owned(),
                added: forward.added,
                deleted: forward.deleted,
                modified: forward.modified,
                divergence: forward.divergence,
            }
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use dircpy::copy_dir;
    use render_as_tree::render;
    use std::fs;
    use tempdir::TempDir;

    use super::*;
    use crate::{test_utils::append_to_file, utils::produce_label_tree};

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

        let mp = &MultiProgress::new();
        let versions = load_versions(base, true, &mp).unwrap();
        let version_tree = infer_version_tree(versions, true, &mp);
        let name_tree = version_tree.map(&|v| v.name.to_owned());

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

        let mp = &MultiProgress::new();
        let versions = load_versions(base, true, &mp).unwrap();
        let version_tree = infer_version_tree(versions, true, &mp);
        let name_tree = version_tree.map(&|v| v.name.to_owned());

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
