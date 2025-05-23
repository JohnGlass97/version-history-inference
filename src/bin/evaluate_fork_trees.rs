#![allow(unused)]

use std::{
    collections::{HashMap, HashSet},
    fs,
};

use disjoint::DisjointSet;
use render_as_tree::render;
use version_history_inference::{
    evaluation::forks::{gen_version_name, VersionRef},
    types::{DiffInfo, TreeNode},
    utils::produce_label_tree,
};

fn gen_name_map(node: &TreeNode<DiffInfo>) -> HashMap<String, usize> {
    let mut stack = vec![node];
    let mut name_map = HashMap::new();
    while let Some(current) = stack.pop() {
        name_map.insert(current.value.name.to_owned(), name_map.len());
        stack.extend(&current.children);
    }
    name_map
}

fn gen_disjoint_set(node: &TreeNode<DiffInfo>, name_map: &HashMap<String, usize>) -> DisjointSet {
    let mut stack = vec![node];
    let mut disjoint_set = DisjointSet::with_len(name_map.len());
    while let Some(current) = stack.pop() {
        let &curr_idx = name_map.get(&current.value.name).unwrap();
        for child in &current.children {
            if child.value.no_changes() {
                let &child_idx = name_map.get(&child.value.name).unwrap();
                disjoint_set.join(curr_idx, child_idx);
                // println!("SAME: {} == {}", &current.value.name, &child.value.name);
            }
        }
        stack.extend(&current.children);
    }
    disjoint_set
}

fn convert_identical_clusters_to_siblings(
    tree: &TreeNode<String>,
    name_map: &HashMap<String, usize>,
    disjoint_set: &DisjointSet,
) -> TreeNode<String> {
    fn inner(
        node: &TreeNode<String>,
        name_map: &HashMap<String, usize>,
        disjoint_set: &DisjointSet,
    ) -> Vec<TreeNode<String>> {
        let mut identical_siblings = vec![];
        let mut new_children = vec![];

        let &curr_idx = name_map.get(&node.value).unwrap();

        for child in &node.children {
            let child_identical_siblings = inner(child, name_map, disjoint_set);

            let &child_idx = name_map.get(&child.value).unwrap();

            if disjoint_set.is_joined(curr_idx, child_idx) {
                // Make child and its identical siblings be identical siblings of the current node
                identical_siblings.extend(child_identical_siblings);
                // // Also correct the DiffInfo for this move
                // for sibling in child_identical_siblings {
                //     identical_siblings.push(TreeNode {
                //         value: DiffInfo {
                //             name: sibling.value.name,
                //             added: node.value.added,
                //             deleted: node.value.deleted,
                //             modified: node.value.modified,
                //         },
                //         children: sibling.children,
                //     })
                // }
            } else {
                new_children.extend(child_identical_siblings);
            }
        }

        identical_siblings.push(TreeNode {
            value: node.value.clone(),
            children: new_children,
        });
        identical_siblings
    }

    let converted_tree_vec = inner(tree, name_map, disjoint_set);
    assert_eq!(converted_tree_vec.len(), 1);
    converted_tree_vec.into_iter().next().unwrap()
}

fn normalise_identical(
    ground_tree: &TreeNode<String>,
    inferred_tree: &TreeNode<DiffInfo>,
) -> (TreeNode<String>, TreeNode<String>) {
    let name_map = gen_name_map(inferred_tree);
    let disjoint_set = gen_disjoint_set(inferred_tree, &name_map);

    let new_ground_tree =
        convert_identical_clusters_to_siblings(&ground_tree, &name_map, &disjoint_set);
    let new_inferred_tree = convert_identical_clusters_to_siblings(
        &inferred_tree.map(&|d| d.name.to_owned()),
        &name_map,
        &disjoint_set,
    );

    (new_ground_tree, new_inferred_tree)
}

fn make_ancestor_sets(tree: &TreeNode<String>) -> HashMap<String, HashSet<String>> {
    fn inner(
        node: &TreeNode<String>,
        ancestors: &HashSet<String>,
        map: &mut HashMap<String, HashSet<String>>,
    ) {
        map.insert(node.value.to_owned(), ancestors.clone());

        let mut including_self = ancestors.clone();
        including_self.insert(node.value.to_owned());

        for child in &node.children {
            inner(child, &including_self, map);
        }
    }
    let mut ancestor_sets = HashMap::new();
    inner(tree, &HashSet::new(), &mut ancestor_sets);
    ancestor_sets
}

fn compare_ancestor_sets(
    ground_sets: &HashMap<String, HashSet<String>>,
    inferred_sets: &HashMap<String, HashSet<String>>,
) -> (f32, f32, f32) {
    let mut total_precisions = 0.;
    let mut total_recalls = 0.;
    let mut total_f1s = 0.;

    for (name, ground_set) in ground_sets {
        let inferred_set = inferred_sets.get(name).unwrap();
        // println!("{ground_set:?} {inferred_set:?}");
        let intersection = ground_set.intersection(inferred_set).count() as f32 + 1.;

        let precision = intersection / (inferred_set.len() + 1) as f32;
        let recall = intersection / (ground_set.len() + 1) as f32;
        let f1 = 2. * precision * recall / (precision + recall);

        total_precisions += precision;
        total_recalls += recall;
        total_f1s += f1;
    }

    let n = ground_sets.len() as f32;
    (total_precisions / n, total_recalls / n, total_f1s / n)
}

fn remove_empty(ancestor_sets: &mut HashMap<String, HashSet<String>>) {
    for set in ancestor_sets.values_mut() {
        set.remove("Empty");
    }

    ancestor_sets.remove("Empty");
}

fn main() {
    let fork_trees_json = fs::read_to_string("./test_repos/fork_trees.json").unwrap();
    let fork_trees: HashMap<String, TreeNode<VersionRef>> =
        serde_json::from_str(&fork_trees_json).unwrap();

    let mut rows = vec![];

    let mut total_precisions = 0.;
    let mut total_recalls = 0.;
    let mut total_f1s = 0.;
    let mut total_versions = 0.;
    let mut n = 0.;

    rows.push((
        "Repo".to_owned(),
        "Precision".to_owned(),
        "Recall".to_owned(),
        "F1".to_owned(),
        "No. Versions".to_owned(),
    ));

    for (root_name, fork_tree) in fork_trees {
        let ground_fork_tree = TreeNode {
            value: "Empty".to_string(),
            children: vec![fork_tree.map(&gen_version_name)],
        };

        let Ok(fork_tree_json) =
            fs::read_to_string(format!("./test_repos/{root_name}/version_tree.json"))
        else {
            println!("Skipping {root_name} as could not load in version_tree.json");
            continue;
        };
        let inferred_fork_tree: TreeNode<DiffInfo> = serde_json::from_str(&fork_tree_json).unwrap();

        // println!("{}", render(&ground_fork_tree).join("\n"));
        // let label_tree = produce_label_tree(&inferred_fork_tree);
        // println!("{}", render(&label_tree).join("\n"));

        let (new_ground_tree, new_inferred_tree) =
            normalise_identical(&ground_fork_tree, &inferred_fork_tree);

        // println!("{}", render(&new_ground_tree).join("\n"));
        // println!("{}", render(&new_inferred_tree).join("\n"));

        let mut ground_sets = make_ancestor_sets(&new_ground_tree);
        let mut inferred_sets = make_ancestor_sets(&new_inferred_tree);

        remove_empty(&mut ground_sets);
        remove_empty(&mut inferred_sets);

        let n_versions = ground_sets.len();

        let (precision, recall, f1) = compare_ancestor_sets(&ground_sets, &inferred_sets);
        // println!("{root_name:20}    {precision:.2}    {recall:.2}    {f1:.2}");
        rows.push((
            root_name,
            format!("{precision:.2}"),
            format!("{recall:.2}"),
            format!("{f1:.2}"),
            n_versions.to_string(),
        ));

        total_precisions += precision;
        total_recalls += recall;
        total_f1s += f1;
        total_versions += n_versions as f32;
        n += 1.;
    }

    let avg_precision = total_precisions / n;
    let avg_recall = total_recalls / n;
    let avg_f1 = total_f1s / n;
    let avg_versions = total_versions / n;

    rows.push((
        "Average".to_owned(),
        format!("{avg_precision:.2}"),
        format!("{avg_recall:.2}"),
        format!("{avg_f1:.2}"),
        format!("{avg_versions:.0}"),
    ));

    for (i, (a, b, c, d, e)) in rows.into_iter().enumerate() {
        if i == 0 {
            println!("\\hline");
            println!("{a:20} & {b:10} & {c:10} & {d:10} & {e:10} \\\\");
        } else {
            println!("{a:20} & {b:>10} & {c:>10} & {d:>10} & {e:>10} \\\\");
        }
        println!("\\hline");
    }
}
