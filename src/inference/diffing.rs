use similar::{ChangeTag, TextDiff};
use std::{collections::HashMap, path::Path};

use crate::types::{FileChange, FileData, TextChange, TextualVersionDiff, Version};

/// Find the text changes between `old` and `new` and push them to `buffer`
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

/// Find what files were added/removed and what text modifications were made
pub fn text_diff_versions(old: &Version, new: &Version) -> TextualVersionDiff {
    let mut added_files: Vec<FileChange> = Vec::new();
    let mut deleted_files: Vec<FileChange> = Vec::new();
    let mut modified_files: Vec<FileChange> = Vec::new();

    // Iterate through old files to find added or modified files
    for (file_name, old_file) in old.files.iter() {
        let old_text = old_file.text_content.as_deref().unwrap_or("");

        // Check for match in new files
        match new.files.get(file_name) {
            Some(new_file) => {
                let new_text = new_file.text_content.as_deref().unwrap_or("");

                if old_text != new_text {
                    let mut changes: Vec<TextChange> = Vec::new();
                    push_text_diff_changes(old_text, new_text, &mut changes);

                    modified_files.push(FileChange {
                        filename: file_name.to_string(),
                        changes,
                    });
                }
            }
            None => {
                // No match in new version, file was deleted (or renamed??)
                // TODO: Consider renamed files
                let mut changes: Vec<TextChange> = Vec::new();
                push_text_diff_changes(old_text, "", &mut changes);

                deleted_files.push(FileChange {
                    filename: file_name.to_string(),
                    changes,
                });
            }
        };
    }

    // Iterate through new files to find deleted files
    for (file_name, new_file) in new.files.iter() {
        match old.files.get(file_name) {
            Some(_) => (), // Already handled in previous for loop
            None => {
                // File must have been added
                let new_text = new_file.text_content.as_deref().unwrap_or("");

                let mut changes: Vec<TextChange> = Vec::new();
                push_text_diff_changes("", new_text, &mut changes);

                added_files.push(FileChange {
                    filename: file_name.to_string(),
                    changes,
                });
            }
        };
    }

    return TextualVersionDiff {
        added_files,
        deleted_files,
        modified_files,
    };
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_push_text_diff_changes_add_one() {
        let mut changes: Vec<TextChange> = Vec::new();
        push_text_diff_changes("abc\n", "abc\n123\n", &mut changes);

        assert_eq!(changes.len(), 1);

        assert_eq!(changes[0].tag, ChangeTag::Insert);
        assert_eq!(changes[0].value, "123\n");
    }

    #[test]
    fn test_push_text_diff_changes_delete_one() {
        let mut changes: Vec<TextChange> = Vec::new();
        push_text_diff_changes("abc\n123\n", "123\n", &mut changes);

        assert_eq!(changes.len(), 1);

        assert_eq!(changes[0].tag, ChangeTag::Delete);
        assert_eq!(changes[0].value, "abc\n");
    }

    #[test]
    fn test_push_text_diff_changes_replace() {
        let mut changes: Vec<TextChange> = Vec::new();
        push_text_diff_changes("abc\n123\n", "abc\ndef\n", &mut changes);

        assert_eq!(changes.len(), 2);

        assert_eq!(changes[0].tag, ChangeTag::Delete);
        assert_eq!(changes[0].value, "123\n");

        assert_eq!(changes[1].tag, ChangeTag::Insert);
        assert_eq!(changes[1].value, "def\n");
    }

    #[test]
    fn test_push_text_diff_changes_replace_two() {
        let mut changes: Vec<TextChange> = Vec::new();
        push_text_diff_changes(
            "abc\n123\nxyz\n456\nend\n",
            "abc\ndef\nxyz\nghi\nend\n",
            &mut changes,
        );

        assert_eq!(changes.len(), 4);

        assert_eq!(changes[0].tag, ChangeTag::Delete);
        assert_eq!(changes[0].value, "123\n");

        assert_eq!(changes[1].tag, ChangeTag::Insert);
        assert_eq!(changes[1].value, "def\n");

        assert_eq!(changes[2].tag, ChangeTag::Delete);
        assert_eq!(changes[2].value, "456\n");

        assert_eq!(changes[3].tag, ChangeTag::Insert);
        assert_eq!(changes[3].value, "ghi\n");
    }

    #[test]
    fn test_text_diff_versions() {
        let s = |x: &str| Some(x.to_string());

        let old = Version {
            name: "old".to_string(),
            path: Path::new(".").into(),
            files: HashMap::from([
                (
                    "modified".to_string(),
                    FileData {
                        text_content: s("ok_code\n"),
                    },
                ),
                (
                    "deleted".to_string(),
                    FileData {
                        text_content: s("bad_code\n"),
                    },
                ),
            ]),
        };
        let new = Version {
            name: "new".to_string(),
            path: Path::new(".").into(),
            files: HashMap::from([
                (
                    "modified".to_string(),
                    FileData {
                        text_content: s("better_code\n"),
                    },
                ),
                (
                    "added".to_string(),
                    FileData {
                        text_content: s("good_code\n"),
                    },
                ),
            ]),
        };

        let diff = text_diff_versions(&old, &new);

        assert_eq!(diff.added_files.len(), 1);
        assert_eq!(diff.added_files[0].filename, "added");
        assert_eq!(diff.added_files[0].changes.len(), 1);

        assert_eq!(diff.deleted_files.len(), 1);
        assert_eq!(diff.deleted_files[0].filename, "deleted");
        assert_eq!(diff.deleted_files[0].changes.len(), 1);

        assert_eq!(diff.modified_files.len(), 1);
        assert_eq!(diff.modified_files[0].filename, "modified");
        assert_eq!(diff.modified_files[0].changes.len(), 2);
    }
}
