use similar::{ChangeTag, TextDiff};

use crate::types::{TextChange, TextualVersionDiff, Version};

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
        match old.files.get(file_name) {
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

#[cfg(test)]
mod tests {
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
}
