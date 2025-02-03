use similar::{ChangeTag, TextDiff};
use std::{collections::HashMap, path::Path};

use crate::types::{FileData, TextChange, TextualVersionDiff, Version};

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

pub fn text_diff_versions(old: &Version, new: &Version) -> TextualVersionDiff {
    let mut added_files: Vec<String> = Vec::new();
    let mut deleted_files: Vec<String> = Vec::new();
    let mut add_delete_changes: Vec<TextChange> = Vec::new();

    let mut modified_files: Vec<String> = Vec::new();
    let mut modify_changes: Vec<TextChange> = Vec::new();

    for (file_name, old_file) in old.files.iter() {
        let old_text = old_file.text_content.as_deref().unwrap_or("");

        match new.files.get(file_name) {
            Some(new_file) => {
                let new_text = new_file.text_content.as_deref().unwrap_or("");

                if old_text != new_text {
                    modified_files.push(file_name.to_string());
                }

                push_text_diff_changes(old_text, new_text, &mut modify_changes);
            }
            None => {
                // No match in new version, file was deleted (or renamed??)
                // TODO: Consider renamed files
                deleted_files.push(file_name.to_string());

                push_text_diff_changes(old_text, "", &mut add_delete_changes);
            }
        };
    }

    for (file_name, new_file) in new.files.iter() {
        match old.files.get(file_name) {
            Some(_) => (),
            None => {
                // File must have been added
                let new_text = new_file.text_content.as_deref().unwrap_or("");

                added_files.push(file_name.to_string());

                push_text_diff_changes("", new_text, &mut add_delete_changes);
            }
        };
    }

    return TextualVersionDiff {
        added_files,
        deleted_files,
        add_delete_changes,
        modified_files,
        modify_changes,
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

        assert_eq!(diff.added_files, ["added"]);
        assert_eq!(diff.deleted_files, ["deleted"]);
        assert_eq!(diff.modified_files, ["modified"]);
        assert_eq!(diff.add_delete_changes.len(), 2);
        assert_eq!(diff.modify_changes.len(), 2);
    }
}
