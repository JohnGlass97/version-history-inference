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
