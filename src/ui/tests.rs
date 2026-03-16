use super::render::{align_lines, aligned_line_count, build_unified_lines, unified_line_count};
use crate::diff::{apply_operations, ChangeOp, LineChange};

fn lc(num: usize, s: &str) -> LineChange {
    (num, s.to_string())
}

fn gap() -> LineChange {
    (0, String::new())
}

// ── align_lines ─────────────────────────────────────────────────────────

#[test]
fn align_empty_inputs() {
    let (base, head) = align_lines(&[], &[]);
    assert!(base.is_empty());
    assert!(head.is_empty());
}

#[test]
fn align_context_only() {
    let base = vec![lc(1, " foo"), lc(2, " bar")];
    let head = vec![lc(1, " foo"), lc(2, " bar")];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, base);
    assert_eq!(ah, head);
}

#[test]
fn align_pure_additions() {
    let base: Vec<LineChange> = vec![];
    let head = vec![lc(1, "+new1"), lc(2, "+new2")];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, vec![gap(), gap()]);
    assert_eq!(ah, head);
}

#[test]
fn align_pure_deletions() {
    let base = vec![lc(1, "-old1"), lc(2, "-old2")];
    let head: Vec<LineChange> = vec![];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, base);
    assert_eq!(ah, vec![gap(), gap()]);
}

#[test]
fn align_balanced_change() {
    let base = vec![lc(1, " ctx"), lc(2, "-old"), lc(3, " ctx2")];
    let head = vec![lc(1, " ctx"), lc(2, "+new"), lc(3, " ctx2")];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, base);
    assert_eq!(ah, head);
}

#[test]
fn align_more_additions_than_deletions() {
    let base = vec![lc(1, "-old")];
    let head = vec![lc(1, "+new1"), lc(2, "+new2"), lc(3, "+new3")];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, vec![lc(1, "-old"), gap(), gap()]);
    assert_eq!(ah, head);
}

#[test]
fn align_more_deletions_than_additions() {
    let base = vec![lc(1, "-old1"), lc(2, "-old2"), lc(3, "-old3")];
    let head = vec![lc(1, "+new")];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, base);
    assert_eq!(ah, vec![lc(1, "+new"), gap(), gap()]);
}

#[test]
fn align_adjacent_change_blocks() {
    // Two separate change blocks with no context in between:
    // base: [-a, ctx, -b]  head: [+x, ctx, +y, +z]
    let base = vec![lc(1, "-a"), lc(2, " ctx"), lc(4, "-b")];
    let head = vec![lc(1, "+x"), lc(2, " ctx"), lc(4, "+y"), lc(5, "+z")];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, vec![lc(1, "-a"), lc(2, " ctx"), lc(4, "-b"), gap()]);
    assert_eq!(
        ah,
        vec![lc(1, "+x"), lc(2, " ctx"), lc(4, "+y"), lc(5, "+z")]
    );
}

#[test]
fn align_single_line_change() {
    let base = vec![lc(5, "-old")];
    let head = vec![lc(5, "+new")];
    let (ab, ah) = align_lines(&base, &head);
    assert_eq!(ab, vec![lc(5, "-old")]);
    assert_eq!(ah, vec![lc(5, "+new")]);
}

// ── aligned_line_count consistency ──────────────────────────────────────

#[test]
fn aligned_count_matches_align_len() {
    let cases: Vec<(Vec<LineChange>, Vec<LineChange>)> = vec![
        (vec![], vec![]),
        (vec![lc(1, " ctx")], vec![lc(1, " ctx")]),
        (vec![], vec![lc(1, "+a"), lc(2, "+b")]),
        (vec![lc(1, "-a"), lc(2, "-b")], vec![]),
        (
            vec![lc(1, "-old"), lc(2, " ctx")],
            vec![lc(1, "+n1"), lc(2, "+n2"), lc(3, "+n3"), lc(4, " ctx")],
        ),
        (
            vec![
                lc(1, " a"),
                lc(2, "-b"),
                lc(3, " c"),
                lc(4, "-d"),
                lc(5, "-e"),
            ],
            vec![lc(1, " a"), lc(2, "+x"), lc(3, "+y"), lc(4, " c")],
        ),
    ];

    for (i, (base, head)) in cases.iter().enumerate() {
        let (ab, _) = align_lines(base, head);
        let count = aligned_line_count(base, head);
        assert_eq!(
            ab.len(),
            count,
            "case {}: align_lines len ({}) != aligned_line_count ({})",
            i,
            ab.len(),
            count,
        );
    }
}

// ── build_unified_lines ─────────────────────────────────────────────────

#[test]
fn unified_empty_inputs() {
    let result = build_unified_lines(&[], &[]);
    assert!(result.is_empty());
}

#[test]
fn unified_context_only() {
    let base = vec![lc(1, " foo"), lc(2, " bar")];
    let head = vec![lc(1, " foo"), lc(2, " bar")];
    let result = build_unified_lines(&base, &head);
    assert_eq!(result, vec![lc(1, " foo"), lc(2, " bar")]);
}

#[test]
fn unified_removals_before_additions() {
    let base = vec![lc(1, "-old1"), lc(2, "-old2")];
    let head = vec![lc(1, "+new1")];
    let result = build_unified_lines(&base, &head);
    assert_eq!(result, vec![lc(1, "-old1"), lc(2, "-old2"), lc(1, "+new1")]);
}

#[test]
fn unified_change_block_ordering() {
    // Within a change block, all removals come before all additions
    let base = vec![lc(1, " ctx"), lc(2, "-a"), lc(3, "-b"), lc(4, " end")];
    let head = vec![
        lc(1, " ctx"),
        lc(2, "+x"),
        lc(3, "+y"),
        lc(4, "+z"),
        lc(5, " end"),
    ];
    let result = build_unified_lines(&base, &head);
    assert_eq!(
        result,
        vec![
            lc(1, " ctx"),
            lc(2, "-a"),
            lc(3, "-b"),
            lc(2, "+x"),
            lc(3, "+y"),
            lc(4, "+z"),
            lc(4, " end"),
        ]
    );
}

#[test]
fn unified_pure_additions() {
    let base: Vec<LineChange> = vec![];
    let head = vec![lc(1, "+a"), lc(2, "+b")];
    let result = build_unified_lines(&base, &head);
    assert_eq!(result, vec![lc(1, "+a"), lc(2, "+b")]);
}

#[test]
fn unified_pure_deletions() {
    let base = vec![lc(1, "-a"), lc(2, "-b")];
    let head: Vec<LineChange> = vec![];
    let result = build_unified_lines(&base, &head);
    assert_eq!(result, vec![lc(1, "-a"), lc(2, "-b")]);
}

#[test]
fn unified_base_exhausted_head_has_context() {
    // Edge case: base is exhausted but head still has context lines
    let base = vec![lc(1, " ctx")];
    let head = vec![lc(1, " ctx"), lc(2, "+added"), lc(3, " trailing")];
    let result = build_unified_lines(&base, &head);
    // ctx paired, then +added from change block, then trailing from head
    assert_eq!(
        result,
        vec![lc(1, " ctx"), lc(2, "+added"), lc(3, " trailing")]
    );
}

// ── unified_line_count consistency ──────────────────────────────────────

#[test]
fn unified_count_matches_build_len() {
    let cases: Vec<(Vec<LineChange>, Vec<LineChange>)> = vec![
        (vec![], vec![]),
        (vec![lc(1, " ctx")], vec![lc(1, " ctx")]),
        (vec![], vec![lc(1, "+a"), lc(2, "+b")]),
        (vec![lc(1, "-a"), lc(2, "-b")], vec![]),
        (
            vec![lc(1, "-old"), lc(2, " ctx")],
            vec![lc(1, "+n1"), lc(2, "+n2"), lc(3, "+n3"), lc(4, " ctx")],
        ),
        (
            vec![
                lc(1, " a"),
                lc(2, "-b"),
                lc(3, " c"),
                lc(4, "-d"),
                lc(5, "-e"),
            ],
            vec![lc(1, " a"), lc(2, "+x"), lc(3, "+y"), lc(4, " c")],
        ),
        // Base exhausted, head has trailing context
        (
            vec![lc(1, " ctx")],
            vec![lc(1, " ctx"), lc(2, "+added"), lc(3, " trailing")],
        ),
    ];

    for (i, (base, head)) in cases.iter().enumerate() {
        let built = build_unified_lines(base, head);
        let count = unified_line_count(base, head);
        assert_eq!(
            built.len(),
            count,
            "case {}: build_unified_lines len ({}) != unified_line_count ({})",
            i,
            built.len(),
            count,
        );
    }
}

// ── Both panes equal length after alignment ─────────────────────────────

#[test]
fn align_produces_equal_length_sides() {
    let cases: Vec<(Vec<LineChange>, Vec<LineChange>)> = vec![
        (vec![], vec![]),
        (vec![lc(1, " x")], vec![lc(1, " x")]),
        (vec![lc(1, "-a")], vec![lc(1, "+b"), lc(2, "+c")]),
        (
            vec![lc(1, "-a"), lc(2, "-b"), lc(3, "-c")],
            vec![lc(1, "+x")],
        ),
        (
            vec![lc(1, " h"), lc(2, "-d"), lc(3, " t")],
            vec![
                lc(1, " h"),
                lc(2, "+a"),
                lc(3, "+b"),
                lc(4, "+c"),
                lc(5, " t"),
            ],
        ),
    ];

    for (i, (base, head)) in cases.iter().enumerate() {
        let (ab, ah) = align_lines(base, head);
        assert_eq!(
            ab.len(),
            ah.len(),
            "case {}: aligned base len ({}) != aligned head len ({})",
            i,
            ab.len(),
            ah.len(),
        );
    }
}

// ── apply_operations ────────────────────────────────────────────────────

fn lines(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn apply_empty_operations() {
    let input = lines(&["a", "b", "c"]);
    let result = apply_operations(&input, &[]);
    assert_eq!(result, input);
}

#[test]
fn apply_single_replace() {
    let input = lines(&["alpha", "beta", "gamma"]);
    let ops = vec![ChangeOp::Replace(2, "BETA".to_string())];
    let result = apply_operations(&input, &ops);
    assert_eq!(result, lines(&["alpha", "BETA", "gamma"]));
}

#[test]
fn apply_single_delete() {
    let input = lines(&["a", "b", "c", "d"]);
    let ops = vec![ChangeOp::Delete(2)];
    let result = apply_operations(&input, &ops);
    assert_eq!(result, lines(&["a", "c", "d"]));
}

#[test]
fn apply_single_insert() {
    let input = lines(&["a", "b", "c"]);
    let ops = vec![ChangeOp::Insert {
        base_pos: 2,
        order: 1,
        content: "NEW".to_string(),
    }];
    let result = apply_operations(&input, &ops);
    assert_eq!(result, lines(&["a", "NEW", "b", "c"]));
}

#[test]
fn apply_multiple_deletes_descending() {
    // Deleting lines 2 and 4 from [a, b, c, d, e]
    let input = lines(&["a", "b", "c", "d", "e"]);
    let ops = vec![ChangeOp::Delete(2), ChangeOp::Delete(4)];
    let result = apply_operations(&input, &ops);
    assert_eq!(result, lines(&["a", "c", "e"]));
}

#[test]
fn apply_delete_and_insert_at_same_position() {
    // Delete line 3 and insert at base_pos 3
    let input = lines(&["a", "b", "c", "d"]);
    let ops = vec![
        ChangeOp::Delete(3),
        ChangeOp::Insert {
            base_pos: 3,
            order: 1,
            content: "NEW".to_string(),
        },
    ];
    let result = apply_operations(&input, &ops);
    // Line 3 ("c") deleted, then "NEW" inserted at the same spot
    assert_eq!(result, lines(&["a", "b", "NEW", "d"]));
}

#[test]
fn apply_multiple_inserts_same_position_preserve_order() {
    // Two inserts at the same base position should preserve source order
    let input = lines(&["a", "b", "c"]);
    let ops = vec![
        ChangeOp::Insert {
            base_pos: 2,
            order: 10,
            content: "FIRST".to_string(),
        },
        ChangeOp::Insert {
            base_pos: 2,
            order: 11,
            content: "SECOND".to_string(),
        },
    ];
    let result = apply_operations(&input, &ops);
    assert_eq!(result, lines(&["a", "FIRST", "SECOND", "b", "c"]));
}

#[test]
fn apply_delete_then_insert_adjusts_position() {
    // Delete line 2, then insert at base_pos 4 — the insert should
    // account for the earlier deletion
    let input = lines(&["a", "b", "c", "d", "e"]);
    let ops = vec![
        ChangeOp::Delete(2),
        ChangeOp::Insert {
            base_pos: 4,
            order: 1,
            content: "NEW".to_string(),
        },
    ];
    let result = apply_operations(&input, &ops);
    // After delete: [a, c, d, e]. Insert adjusts: base_pos 4, 1 delete before it,
    // adjusted=3, idx=2 → insert before "d"
    assert_eq!(result, lines(&["a", "c", "NEW", "d", "e"]));
}

#[test]
fn apply_replace_and_insert_mixed() {
    let input = lines(&["a", "b", "c", "d"]);
    let ops = vec![
        ChangeOp::Replace(2, "B2".to_string()),
        ChangeOp::Insert {
            base_pos: 4,
            order: 1,
            content: "NEW".to_string(),
        },
    ];
    let result = apply_operations(&input, &ops);
    assert_eq!(result, lines(&["a", "B2", "c", "NEW", "d"]));
}

#[test]
fn apply_skips_zero_line_numbers() {
    let input = lines(&["a", "b"]);
    let ops = vec![
        ChangeOp::Delete(0),
        ChangeOp::Replace(0, "X".to_string()),
        ChangeOp::Insert {
            base_pos: 0,
            order: 1,
            content: "Y".to_string(),
        },
    ];
    let result = apply_operations(&input, &ops);
    assert_eq!(result, input, "zero line numbers should be no-ops");
}

// ── syntax highlighting ─────────────────────────────────────────────────

#[test]
fn syntax_theme_exists() {
    use super::syntax::{SYNTAX_SET, THEME_SET};
    // Verify the themes referenced by our Theme structs actually exist
    assert!(
        THEME_SET.themes.contains_key("base16-ocean.dark"),
        "missing base16-ocean.dark in syntect themes: {:?}",
        THEME_SET.themes.keys().collect::<Vec<_>>()
    );
    assert!(
        THEME_SET.themes.contains_key("base16-ocean.light"),
        "missing base16-ocean.light in syntect themes: {:?}",
        THEME_SET.themes.keys().collect::<Vec<_>>()
    );
    // Verify Rust syntax is available
    assert!(
        SYNTAX_SET
            .find_syntax_for_file("main.rs")
            .ok()
            .flatten()
            .is_some(),
        "Rust syntax not found in syntect defaults"
    );
}

#[test]
fn highlight_produces_colored_spans() {
    use super::syntax::highlight_line_changes;
    use super::theme::Theme;
    let theme = Theme::dark();
    let lines = vec![
        lc(1, " fn main() {"),
        lc(2, "-    let x = 5;"),
        lc(3, "+    let y = 10;"),
        lc(4, " }"),
    ];

    let result = highlight_line_changes(&lines, "test.rs", &theme);
    assert_eq!(result.len(), 4);

    // Context line (line 1) should have multiple spans with syntax colors
    let ctx_spans = &result[0].spans;
    assert!(
        ctx_spans.len() >= 3,
        "context line should have line_num + spacer + code spans, got {}",
        ctx_spans.len()
    );

    // Removed line (line 2) should have bg_removed on code spans
    let rem_spans = &result[1].spans;
    assert!(
        rem_spans.len() >= 3,
        "removed line should have line_num + marker + code spans"
    );
    // The marker span should have the removed marker color
    assert_eq!(rem_spans[1].style.fg, Some(theme.fg_removed_marker));

    // Added line (line 3) should have bg_added on code spans
    let add_spans = &result[2].spans;
    assert!(
        add_spans.len() >= 3,
        "added line should have line_num + marker + code spans"
    );
    assert_eq!(add_spans[1].style.fg, Some(theme.fg_added_marker));

    // Verify at least some syntax highlighting colors differ from plain text
    let code_spans_line1: Vec<_> = ctx_spans.iter().skip(2).collect();
    let has_varied_colors = code_spans_line1
        .windows(2)
        .any(|w| w[0].style.fg != w[1].style.fg);
    // For ` fn main() {` (note leading space from diff format),
    // syntect should color `fn` differently from `main`
    assert!(
        has_varied_colors || code_spans_line1.len() > 1,
        "syntax highlighting should produce varied colors for Rust code: {:?}",
        code_spans_line1
            .iter()
            .map(|s| (&s.content, s.style.fg))
            .collect::<Vec<_>>()
    );

    // Verify changed lines also have syntax coloring (not just plain text)
    let rem_code_spans: Vec<_> = result[1].spans.iter().skip(2).collect();
    assert!(
        rem_code_spans.len() > 1 || rem_code_spans.iter().any(|s| s.style.fg.is_some()),
        "removed line should have syntax-colored spans: {:?}",
        rem_code_spans
            .iter()
            .map(|s| (&s.content, s.style.fg))
            .collect::<Vec<_>>()
    );
    let add_code_spans: Vec<_> = result[2].spans.iter().skip(2).collect();
    assert!(
        add_code_spans.len() > 1 || add_code_spans.iter().any(|s| s.style.fg.is_some()),
        "added line should have syntax-colored spans: {:?}",
        add_code_spans
            .iter()
            .map(|s| (&s.content, s.style.fg))
            .collect::<Vec<_>>()
    );

    // Context lines should have the diff-format leading space stripped,
    // so "fn" appears right after the spacer (no extra space span).
    let ctx_code_start = &ctx_spans[2];
    assert!(
        !ctx_code_start.content.starts_with(' '),
        "context line code should not start with diff-format leading space, got {:?}",
        ctx_code_start.content
    );
}

#[test]
fn highlight_gap_lines_are_empty() {
    use super::syntax::highlight_line_changes;
    use super::theme::Theme;

    let theme = Theme::dark();
    let lines = vec![gap(), lc(1, "+added"), gap()];
    let result = highlight_line_changes(&lines, "test.rs", &theme);
    assert_eq!(result.len(), 3);
    // Gap lines should be empty
    assert_eq!(result[0].spans.len(), 1);
    assert_eq!(result[0].spans[0].content.as_ref(), "");
    assert_eq!(result[2].spans.len(), 1);
    assert_eq!(result[2].spans[0].content.as_ref(), "");
}
