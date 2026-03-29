//! Tests for incremental reparsing

use crate::edit;

/// Helper: parse old text, apply edit to get new text, reparse incrementally,
/// and verify the result matches a full parse of the new text.
fn assert_incremental_matches_full_patch(
    old_text: &str,
    edit_start: u32,
    edit_end: u32,
    replacement: &str,
) {
    let parsed = edit::parse(old_text);
    let mut new_text = old_text.to_string();
    new_text.replace_range(edit_start as usize..edit_end as usize, replacement);

    let new_edit_end = edit_start + replacement.len() as u32;
    let edit_range = rowan::TextRange::new(
        rowan::TextSize::from(edit_start),
        rowan::TextSize::from(new_edit_end),
    );

    let incremental = parsed.reparse(&new_text, edit_range, edit::parse);
    let full = edit::parse(&new_text);

    assert_eq!(
        incremental.green(),
        full.green(),
        "green node mismatch for patch edit [{edit_start}..{edit_end}] -> {replacement:?}"
    );
}

fn assert_incremental_matches_full_series(
    old_text: &str,
    edit_start: u32,
    edit_end: u32,
    replacement: &str,
) {
    let parsed = edit::series::parse(old_text);
    let mut new_text = old_text.to_string();
    new_text.replace_range(edit_start as usize..edit_end as usize, replacement);

    let new_edit_end = edit_start + replacement.len() as u32;
    let edit_range = rowan::TextRange::new(
        rowan::TextSize::from(edit_start),
        rowan::TextSize::from(new_edit_end),
    );

    let incremental = parsed.reparse(&new_text, edit_range, edit::series::parse);
    let full = edit::series::parse(&new_text);

    assert_eq!(
        incremental.green(),
        full.green(),
        "green node mismatch for series edit [{edit_start}..{edit_end}] -> {replacement:?}"
    );
}

// --- Patch file incremental reparse tests ---

#[test]
fn test_reparse_patch_edit_in_hunk() {
    let old = "\
--- a/file1.txt
+++ b/file1.txt
@@ -1,3 +1,3 @@
 line1
-old
+new
 line3
--- a/file2.txt
+++ b/file2.txt
@@ -1,3 +1,3 @@
 line1
-foo
+bar
 line3
";
    // Change "old" to "changed" in first file (byte offset of "old" in "-old\n")
    let start = old.find("-old").unwrap() as u32 + 1; // skip the -
    let end = start + 3; // "old"
    assert_incremental_matches_full_patch(old, start, end, "changed");
}

#[test]
fn test_reparse_patch_edit_in_second_file() {
    let old = "\
--- a/file1.txt
+++ b/file1.txt
@@ -1 +1 @@
-a
+b
--- a/file2.txt
+++ b/file2.txt
@@ -1 +1 @@
-c
+d
--- a/file3.txt
+++ b/file3.txt
@@ -1 +1 @@
-e
+f
";
    // Edit in second file: change "c" to "x"
    let start = old.find("-c\n").unwrap() as u32 + 1;
    let end = start + 1;
    assert_incremental_matches_full_patch(old, start, end, "x");
}

#[test]
fn test_reparse_patch_add_hunk() {
    let old = "\
--- a/file1.txt
+++ b/file1.txt
@@ -1 +1 @@
-a
+b
--- a/file2.txt
+++ b/file2.txt
@@ -1 +1 @@
-c
+d
";
    // Append a new hunk to file1
    let insert_pos = old.find("--- a/file2").unwrap() as u32;
    let new_hunk = "@@ -10 +10 @@\n-old10\n+new10\n";
    assert_incremental_matches_full_patch(old, insert_pos, insert_pos, new_hunk);
}

#[test]
fn test_reparse_patch_single_file_fallback() {
    // With only one PATCH_FILE child, should fall back to full reparse
    let old = "\
--- a/file.txt
+++ b/file.txt
@@ -1 +1 @@
-old
+new
";
    let start = old.find("old").unwrap() as u32;
    let end = start + 3;
    assert_incremental_matches_full_patch(old, start, end, "changed");
}

// --- Series file incremental reparse tests ---

#[test]
fn test_reparse_series_rename_patch() {
    let old = "patch1.patch\npatch2.patch\npatch3.patch\npatch4.patch\n";
    // Rename patch2 to something-else
    let start = old.find("patch2").unwrap() as u32;
    let end = start + "patch2.patch".len() as u32;
    assert_incremental_matches_full_series(old, start, end, "renamed.patch");
}

#[test]
fn test_reparse_series_add_entry() {
    let old = "patch1.patch\npatch2.patch\npatch3.patch\n";
    // Insert a new entry after patch1
    let insert_pos = old.find('\n').unwrap() as u32 + 1;
    assert_incremental_matches_full_series(old, insert_pos, insert_pos, "new.patch\n");
}

#[test]
fn test_reparse_series_delete_entry() {
    let old = "patch1.patch\npatch2.patch\npatch3.patch\npatch4.patch\n";
    // Delete patch2 line
    let start = old.find("patch2").unwrap() as u32;
    let end = start + "patch2.patch\n".len() as u32;
    assert_incremental_matches_full_series(old, start, end, "");
}

#[test]
fn test_reparse_series_add_options() {
    let old = "patch1.patch\npatch2.patch\npatch3.patch\npatch4.patch\n";
    // Add options to patch2
    let pos = old.find("patch2.patch").unwrap() as u32 + "patch2.patch".len() as u32;
    assert_incremental_matches_full_series(old, pos, pos, " -p1 --reverse");
}

#[test]
fn test_reparse_series_add_comment() {
    let old = "patch1.patch\npatch2.patch\npatch3.patch\n";
    let insert_pos = old.find("patch2").unwrap() as u32;
    assert_incremental_matches_full_series(old, insert_pos, insert_pos, "# Security fixes\n");
}

#[test]
fn test_reparse_roundtrip_text_preserved() {
    // Verify that incremental reparse produces text identical to the new input
    let old = "patch1.patch\npatch2.patch\npatch3.patch\npatch4.patch\n";
    let parsed = edit::series::parse(old);

    let mut new_text = old.to_string();
    let start = old.find("patch2").unwrap();
    let end = start + "patch2.patch".len();
    new_text.replace_range(start..end, "renamed.patch");

    let edit_range = rowan::TextRange::new(
        rowan::TextSize::from(start as u32),
        rowan::TextSize::from(start as u32 + "renamed.patch".len() as u32),
    );

    let incremental = parsed.reparse(&new_text, edit_range, edit::series::parse);
    let tree = incremental.tree();

    use rowan::ast::AstNode;
    assert_eq!(tree.syntax().to_string(), new_text);
}
