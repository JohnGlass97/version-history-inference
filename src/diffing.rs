use std::{collections::HashMap, fmt};

use similar::{ChangeTag, TextDiff};

struct TextChange {
    tag: ChangeTag,
    old_index: Option<usize>,
    new_index: Option<usize>,
    value: String,
}

pub struct TextualVersionDiff {
    added_files: Vec<String>,
    deleted_files: Vec<String>,
    modified_files: Vec<String>,
    changes: Vec<TextChange>,
}

struct FileData {
    text_content: Option<String>,
}

pub struct Version {
    files: HashMap<String, FileData>,
}

fn push_text_diff_changes(old: &str, new: &str, buffer: &mut Vec<TextChange>) {
    let diff = TextDiff::from_lines(old, new);
    buffer.extend(
        diff.iter_all_changes()
            .map(|c| TextChange {
                tag: c.tag(),
                old_index: c.old_index(),
                new_index: c.new_index(),
                value: c.value().to_string(),
            })
            .filter(|c| c.tag != ChangeTag::Equal),
    );
}

fn print_changes(changes: &Vec<TextChange>) {
    for c in changes {
        println!("{c}");
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

pub fn text_diff_versions(old: Version, new: Version) -> TextualVersionDiff {
    let mut added_files: Vec<String> = Vec::new();
    let mut deleted_files: Vec<String> = Vec::new();

    let mut modified_files: Vec<String> = Vec::new();
    let mut changes: Vec<TextChange> = Vec::new();

    for (file_name, old_file) in old.files.iter() {
        let old_text = old_file.text_content.as_deref().unwrap_or("");

        let new_text = match new.files.get(file_name) {
            Some(new_file) => {
                let new_text = new_file.text_content.as_deref().unwrap_or("");

                if old_text != new_text {
                    modified_files.push(file_name.to_string());
                }

                new_text
            }
            None => {
                // No match in new version, file was deleted (or renamed??)
                // TODO: Consider renamed files
                deleted_files.push(file_name.to_string());

                ""
            }
        };

        push_text_diff_changes(old_text, new_text, &mut changes);
    }

    for (file_name, new_file) in new.files.iter() {
        match new.files.get(file_name) {
            Some(_) => (),
            None => {
                // File must have been added
                let new_text = new_file.text_content.as_deref().unwrap_or("");

                added_files.push(file_name.to_string());

                push_text_diff_changes("", new_text, &mut changes);
            }
        };
    }

    return TextualVersionDiff {
        added_files,
        deleted_files,
        modified_files,
        changes,
    };
}