#![allow(unused)]

use std::{
    collections::{HashMap, HashSet},
    fs,
};

use disjoint::DisjointSet;
use render_as_tree::render;
use version_history_inference::{
    evaluation::forks::{gen_version_name, VersionRef},
    rendering::produce_label_tree,
    types::{DiffInfo, TreeNode},
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
    let mut precisions = vec![];
    let mut recalls = vec![];
    let mut f1s = vec![];

    for (name, ground_set) in ground_sets {
        let inferred_set = inferred_sets.get(name).unwrap();
        println!("{ground_set:?} {inferred_set:?}");
        let intersection = ground_set.intersection(inferred_set).count() as f32 + 1.;

        let precision = intersection / (inferred_set.len() + 1) as f32;
        let recall = intersection / (ground_set.len() + 1) as f32;
        let f1 = 2. * precision * recall / (precision + recall);

        precisions.push(precision);
        recalls.push(recall);
        f1s.push(f1);
    }

    (
        precisions.iter().sum::<f32>() / precisions.len() as f32,
        recalls.iter().sum::<f32>() / recalls.len() as f32,
        f1s.iter().sum::<f32>() / f1s.len() as f32,
    )
}

fn main() {
    let fork_trees_json = fs::read_to_string("./test_repos/fork_trees.json").unwrap();
    let fork_trees: HashMap<String, TreeNode<VersionRef>> =
        serde_json::from_str(&fork_trees_json).unwrap();

    for (root_name, fork_tree) in fork_trees {
        // let (root_name, fork_tree) = fork_trees.get_key_value("imgui-forks").unwrap();
        let ground_fork_tree = fork_tree.map(&gen_version_name);

        println!("{root_name}");
        let fork_tree_json =
            fs::read_to_string(format!("./test_repos/{root_name}/version_tree.json")).unwrap();
        let inferred_fork_tree: TreeNode<DiffInfo> = serde_json::from_str(&fork_tree_json).unwrap();

        // println!("{}", render(&ground_fork_tree).join("\n"));
        // let label_tree = produce_label_tree(&inferred_fork_tree);
        // println!("{}", render(&label_tree).join("\n"));

        let (new_ground_tree, new_inferred_tree) =
            normalise_identical(&ground_fork_tree, &inferred_fork_tree);

        // println!("{}", render(&new_ground_tree).join("\n"));
        // println!("{}", render(&new_inferred_tree).join("\n"));

        let ground_sets = make_ancestor_sets(&new_ground_tree);
        let mut inferred_sets = make_ancestor_sets(&new_inferred_tree);

        for set in inferred_sets.values_mut() {
            set.remove("Empty");
        }

        let (precision, recall, f1) = compare_ancestor_sets(&ground_sets, &inferred_sets);
        println!("{precision} {recall} {f1}");
    }
}
