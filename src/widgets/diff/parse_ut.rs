use super::parse::*;

const CONTEXT: &str = "\
--- a/file.rs
+++ b/file.rs
@@ -1,4 +1,4 @@
 one
-two
+three
 four
";

#[test]
fn parses_context_and_replacement() {
    let parsed = parse_unified(CONTEXT, 4);
    assert_eq!(
        parsed.stats,
        Stats {
            additions: 1,
            removals: 1
        }
    );
    assert_eq!(parsed.hunks.len(), 1);

    let hunk = &parsed.hunks[0];
    assert_eq!(hunk.lines.len(), 4);
    assert_eq!(
        hunk.lines.iter().map(|line| line.kind).collect::<Vec<_>>(),
        vec![Kind::Equal, Kind::Delete, Kind::Insert, Kind::Equal]
    );
    assert_eq!(hunk.lines[1].old_number, Some(2));
    assert_eq!(hunk.lines[2].new_number, Some(2));
    assert_eq!(hunk.lines[3].old_number, Some(3));
    assert_eq!(hunk.lines[3].new_number, Some(3));
}

#[test]
fn pairs_replacements_on_the_same_row() {
    let parsed = parse_unified(CONTEXT, 4);
    let rows = &parsed.hunks[0].rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].old.as_ref().unwrap().text, "two");
    assert_eq!(rows[1].new.as_ref().unwrap().text, "three");
    assert_eq!(rows[1].old.as_ref().unwrap().number, 2);
    assert_eq!(rows[1].new.as_ref().unwrap().number, 2);
}

#[test]
fn pairs_multiple_deletions_with_following_insertions() {
    let diff = "\
@@ -1,5 +1,5 @@
 same
-old one
-old two
+new one
+new two
 tail
";
    let parsed = parse_unified(diff, 4);
    let rows = &parsed.hunks[0].rows;
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[1].old.as_ref().unwrap().text, "old one");
    assert_eq!(rows[1].new.as_ref().unwrap().text, "new one");
    assert_eq!(rows[2].old.as_ref().unwrap().text, "old two");
    assert_eq!(rows[2].new.as_ref().unwrap().text, "new two");
}

#[test]
fn leaves_unpaired_insertions_on_the_right() {
    let diff = "\
@@ -1,2 +1,3 @@
 one
+two
+three
";
    let parsed = parse_unified(diff, 4);
    let rows = &parsed.hunks[0].rows;
    assert_eq!(rows.len(), 3);
    assert!(rows[1].old.is_none());
    assert_eq!(rows[1].new.as_ref().unwrap().text, "two");
    assert_eq!(rows[2].new.as_ref().unwrap().text, "three");
}

#[test]
fn leaves_unpaired_deletions_on_the_left() {
    let diff = "\
@@ -1,3 +1,1 @@
 one
-two
-three
";
    let parsed = parse_unified(diff, 4);
    let rows = &parsed.hunks[0].rows;
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].old.as_ref().unwrap().text, "two");
    assert!(rows[1].new.is_none());
    assert_eq!(rows[2].old.as_ref().unwrap().text, "three");
}

#[test]
fn parses_multiple_hunks() {
    let diff = "\
@@ -1,2 +1,2 @@
 one
-two
+three
@@ -40,2 +40,2 @@
 forty
-forty one
+forty two
";
    let parsed = parse_unified(diff, 4);
    assert_eq!(parsed.hunks.len(), 2);
    assert_eq!(parsed.hunks[0].lines[1].old_number, Some(2));
    assert_eq!(parsed.hunks[1].lines[1].old_number, Some(41));
    assert_eq!(parsed.hunks[1].lines[1].new_number, None);
    assert_eq!(parsed.hunks[1].lines[2].old_number, None);
    assert_eq!(parsed.hunks[1].lines[2].new_number, Some(41));
}

#[test]
fn parses_old_contextless_diff() {
    let diff = "\
--- a/file.rs
+++ b/file.rs
@@ -5,2 +5,2 @@
-old one
-old two
+new one
+new two
";
    let parsed = parse_unified(diff, 4);
    let rows = &parsed.hunks[0].rows;
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].old.as_ref().unwrap().number, 5);
    assert_eq!(rows[0].new.as_ref().unwrap().number, 5);
    assert_eq!(rows[1].old.as_ref().unwrap().number, 6);
    assert_eq!(rows[1].new.as_ref().unwrap().number, 6);
}

#[test]
fn expands_tabs_before_splitting() {
    let diff = "@@ -1,1 +1,1 @@\n-\told\n+\tnew\n";
    let parsed = parse_unified(diff, 4);
    assert_eq!(parsed.hunks[0].lines[0].text, "    old");
    assert_eq!(parsed.hunks[0].lines[1].text, "    new");
}

#[test]
fn ignores_no_newline_marker() {
    let diff = "@@ -1 +1 @@\n-one\n+two\n\\ No newline at end of file\n";
    let parsed = parse_unified(diff, 4);
    assert_eq!(parsed.hunks[0].lines.len(), 2);
}

#[test]
fn content_diff_uses_multiple_hunks() {
    let mut lines = (1..=80)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>();
    let before = lines.join("\n");
    lines[4] = "line five".to_string();
    lines[74] = "line seventy five".to_string();
    let after = lines.join("\n");
    let unified = unified_diff(&before, &after, 3);
    let parsed = parse_unified(&unified, 4);
    assert_eq!(parsed.hunks.len(), 2);
    assert_eq!(
        parsed.stats,
        Stats {
            additions: 2,
            removals: 2
        }
    );
}
