use std::{collections::HashMap, env, fs};

use render_as_tree::render;
use version_history_inference::{
    testing::forks::{gen_version_display_name, VersionRef},
    types::TreeNode,
};

fn main() {
    let fork_trees_json = fs::read_to_string("./test_repos/fork_trees.json").unwrap();
    let fork_trees: HashMap<String, TreeNode<VersionRef>> =
        serde_json::from_str(&fork_trees_json).unwrap();

    let name = env::args()
        .nth(1)
        .expect("Please provide name of fork tree");

    let version_ref_tree = fork_trees.get(&name).expect("No such fork tree found");

    let label_tree = version_ref_tree.map(&|t| gen_version_display_name(t));
    print!("{}\n", render(&label_tree).join("\n"));
}
