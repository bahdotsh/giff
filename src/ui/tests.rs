use super::render::{align_lines, aligned_line_count, build_unified_lines, unified_line_count};
use crate::diff::LineChange;

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
