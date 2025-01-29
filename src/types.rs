use std::{collections::HashMap, fmt, path::Path};

use similar::ChangeTag;

#[derive(Debug)]
pub struct TextChange {
    pub tag: ChangeTag,
    pub old_index: Option<usize>,
    pub new_index: Option<usize>,
    pub value: String,
}

#[derive(Debug)]
pub struct TextualVersionDiff {
    pub added_files: Vec<String>,
    pub deleted_files: Vec<String>,
    pub add_delete_changes: Vec<TextChange>,
    pub modified_files: Vec<String>,
    pub modify_changes: Vec<TextChange>,
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

#[derive(Debug)]
pub struct TreeNode<T> {
    pub value: T,
    pub children: Vec<TreeNode<T>>,
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
