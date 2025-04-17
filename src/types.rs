use std::{collections::HashMap, fmt, path::Path};

use serde::{Deserialize, Serialize};
use similar::ChangeTag;

#[derive(Debug)]
pub struct TextChange {
    pub tag: ChangeTag,
    pub old_index: Option<usize>,
    pub new_index: Option<usize>,
    pub value: String,
}

#[derive(Debug)]
pub struct FileChange {
    pub filename: String,
    pub changes: Vec<TextChange>,
}

#[derive(Debug)]
pub struct TextualVersionDiff {
    pub added_files: Vec<FileChange>,
    pub deleted_files: Vec<FileChange>,
    pub modified_files: Vec<FileChange>,
}

#[derive(Debug)]
pub struct FileData {
    pub text_content: Option<String>,
}

#[derive(Debug)]
pub struct Version {
    pub name: String,
    pub path: Box<Path>,
    pub files: HashMap<String, FileData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode<T> {
    pub value: T,
    pub children: Vec<TreeNode<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffInfo {
    pub name: String,
    pub added: usize,
    pub deleted: usize,
    pub modified: usize,
}

impl<T: Eq> PartialEq for TreeNode<T> {
    fn eq(&self, other: &Self) -> bool {
        if self.value != other.value || self.children.len() != other.children.len() {
            return false;
        }

        'outer: for child_a in &self.children {
            for child_b in &other.children {
                if child_a == child_b {
                    continue 'outer;
                }
            }
            return false;
        }
        true
    }
}

impl<T: Eq> Eq for TreeNode<T> {}

impl<T> TreeNode<T> {
    pub fn map_with_parent<F, U>(&self, f: &F, parent: Option<&T>) -> TreeNode<U>
    where
        F: Fn(&T, Option<&T>) -> U,
    {
        TreeNode {
            value: f(&self.value, parent),
            children: self
                .children
                .iter()
                .map(|c| c.map_with_parent(f, Some(&self.value)))
                .collect(),
        }
    }

    pub fn map<F, U>(&self, f: &F) -> TreeNode<U>
    where
        F: Fn(&T) -> U,
    {
        self.map_with_parent(&|x, _| f(x), None)
    }
}

impl render_as_tree::Node for TreeNode<String> {
    type Iter<'a> = std::slice::Iter<'a, Self>;

    fn name(&self) -> &str {
        &self.value
    }

    fn children(&self) -> Self::Iter<'_> {
        self.children.iter()
    }
}

impl fmt::Display for TextChange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}",
            match self.tag {
                ChangeTag::Equal => "=",
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
            },
            self.value.strip_suffix("\n").unwrap_or(&self.value)
        )
    }
}
