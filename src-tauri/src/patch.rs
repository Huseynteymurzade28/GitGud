//! Builds patches that contain only some lines of a diff, for staging,
//! unstaging or discarding individual lines and hunks.
//!
//! Lines are identified by their index in the diff text (split on '\n'),
//! which is also how the frontend numbers them.

use std::collections::HashSet;

/// Which way the patch will be applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    /// `git apply`: the diff's old side is the target (staging from the
    /// working tree into the index).
    Forward,
    /// `git apply --reverse`: the diff's new side is the target (unstaging,
    /// or discarding working tree changes).
    Reverse,
}

#[derive(Debug, PartialEq)]
pub enum PatchError {
    NothingSelected,
    /// New, deleted and binary files can only be handled as a whole.
    WholeFileOnly,
}

/// Returns a patch with only the `selected` changed lines of `diff`.
///
/// Unselected changes are turned into whatever the target already has: an
/// unselected removal stays as context when applying forward (the line is
/// still in the index), an unselected addition stays as context when
/// applying in reverse. The hunk headers keep their start positions and are
/// meant to be applied with `--recount`, which fixes up the line counts.
pub fn partial(
    diff: &str,
    selected: &HashSet<usize>,
    direction: Direction,
) -> Result<String, PatchError> {
    let lines: Vec<&str> = diff.split('\n').collect();
    let first_hunk = lines
        .iter()
        .position(|l| l.starts_with("@@"))
        .ok_or(PatchError::NothingSelected)?;

    let header = &lines[..first_hunk];
    if header.iter().any(|l| {
        l.starts_with("new file mode")
            || l.starts_with("deleted file mode")
            || l.starts_with("Binary files")
    }) {
        return Err(PatchError::WholeFileOnly);
    }

    let mut patch: Vec<String> = header.iter().map(|l| l.to_string()).collect();
    let mut hunk: Vec<String> = Vec::new();
    let mut hunk_has_change = false;
    let mut any_change = false;
    // Whether the previous diff line made it into the patch, so a following
    // "\ No newline at end of file" marker is kept or dropped along with it.
    let mut last_kept = false;

    for (i, line) in lines.iter().enumerate().skip(first_hunk) {
        let kept = match line.chars().next() {
            Some('@') => {
                if hunk_has_change {
                    patch.append(&mut hunk);
                }
                hunk.clear();
                hunk_has_change = false;
                Some(line.to_string())
            }
            Some(' ') => Some(line.to_string()),
            Some(sign @ ('+' | '-')) => {
                // The side the patch is applied onto already has this line.
                let in_target = match direction {
                    Direction::Forward => sign == '-',
                    Direction::Reverse => sign == '+',
                };
                if selected.contains(&i) {
                    hunk_has_change = true;
                    any_change = true;
                    Some(line.to_string())
                } else if in_target {
                    Some(format!(" {}", &line[1..]))
                } else {
                    None
                }
            }
            Some('\\') => {
                if last_kept {
                    hunk.push(line.to_string());
                }
                continue;
            }
            // The empty string after the final newline.
            _ => None,
        };
        last_kept = kept.is_some();
        hunk.extend(kept);
    }
    if hunk_has_change {
        patch.append(&mut hunk);
    }

    if !any_change {
        return Err(PatchError::NothingSelected);
    }
    Ok(patch.join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIFF: &str = "\
diff --git a/f.txt b/f.txt
index 1111111..2222222 100644
--- a/f.txt
+++ b/f.txt
@@ -1,4 +1,4 @@
 one
-two
+TWO
 three
-four
+FOUR
@@ -10,2 +10,3 @@ ten
 ten
+ten and a half
 eleven
";

    fn set(ids: &[usize]) -> HashSet<usize> {
        ids.iter().copied().collect()
    }

    #[test]
    fn stages_only_selected_lines() {
        // Select "-two" (6) and "+TWO" (7) only.
        let p = partial(DIFF, &set(&[6, 7]), Direction::Forward).unwrap();
        assert!(p.contains("-two\n+TWO\n"));
        assert!(p.contains(" four\n"), "unselected removal stays as context");
        assert!(!p.contains("FOUR"), "unselected addition is dropped");
        assert!(!p.contains("@@ -10"), "untouched hunk is left out");
    }

    #[test]
    fn reverse_keeps_unselected_additions_as_context() {
        // Unstage only "+ten and a half" (13).
        let p = partial(DIFF, &set(&[13]), Direction::Reverse).unwrap();
        assert!(!p.contains("@@ -1,4"), "first hunk has nothing selected");
        assert!(p.contains("+ten and a half\n"));

        let p = partial(DIFF, &set(&[7]), Direction::Reverse).unwrap();
        assert!(p.contains("+TWO\n"));
        assert!(
            !p.contains("-two"),
            "unselected removal is dropped in reverse"
        );
        assert!(
            p.contains(" FOUR\n"),
            "unselected addition stays as context"
        );
    }

    #[test]
    fn rejects_empty_selection_and_whole_file_changes() {
        assert_eq!(
            partial(DIFF, &set(&[5]), Direction::Forward),
            Err(PatchError::NothingSelected),
            "context lines don't count"
        );
        let new_file =
            "diff --git a/n b/n\nnew file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1 @@\n+x\n";
        assert_eq!(
            partial(new_file, &set(&[5]), Direction::Forward),
            Err(PatchError::WholeFileOnly)
        );
    }

    #[test]
    fn no_newline_marker_follows_its_line() {
        let diff = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n+new\n\\ No newline at end of file\n";
        // Only the removal: its marker stays, the dropped addition's goes.
        let p = partial(diff, &set(&[4]), Direction::Forward).unwrap();
        assert_eq!(
            p,
            "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n\\ No newline at end of file\n"
        );
    }
}
