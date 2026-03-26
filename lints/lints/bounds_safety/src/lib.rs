#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, QPath};
use rustc_lint::LateContext;

dylint_linting::declare_late_lint! {
    pub OFF_BY_ONE_INDEX,
    Warn,
    "checks for arr[i+1] access without bounds check",
    OFF_BY_ONE_INDEX_INTERNAL: () = (
        "bounds_safety",
        "Warn",
        "off-by-one index access detected; ensure bounds are checked before access"
    )
}

dylint_linting::declare_late_lint! {
    pub INDEX_OUT_OF_BOUNDS,
    Deny,
    "checks for direct index access without bounds check",
    INDEX_OUT_OF_BOUNDS_INTERNAL: () = (
        "bounds_safety",
        "Deny",
        "index access without bounds check can panic; use .get() or .get_mut()"
    )
}

dylint_linting::declare_late_lint! {
    pub SLICE_RANGE_OUT_OF_BOUNDS,
    Warn,
    "checks for [start..end] range without validation",
    SLICE_RANGE_OUT_OF_BOUNDS_INTERNAL: () = (
        "bounds_safety",
        "Warn",
        "slice range access without validation; ensure start <= end and within bounds"
    )
}

dylint_linting::declare_late_lint! {
    pub NO_RANGE_VALIDATION,
    Warn,
    "checks for usize parameter without validation",
    NO_RANGE_VALIDATION_INTERNAL: () = (
        "bounds_safety",
        "Warn",
        "usize parameter used in indexing without validation; add bounds check"
    )
}

fn check_index_expr(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Index(_, index_expr, _) = expr.kind;
        if let ExprKind::Binary(bin_op, _, _) = index_expr.kind;
        if bin_op.node == hir::BinOpKind::Add || bin_op.node == hir::BinOpKind::Sub;
        then {
            span_lint(
                cx,
                OFF_BY_ONE_INDEX,
                expr.span,
                "off-by-one index calculation detected; verify bounds are sufficient",
            );
        }
    }
}

fn check_direct_index(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Index(_, _, _) = expr.kind;
        then {
            span_lint(
                cx,
                INDEX_OUT_OF_BOUNDS,
                expr.span,
                "direct index access without bounds check; use .get() for safe access",
            );
        }
    }
}

fn check_range_index(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Range(_, _, _) = expr.kind;
        then {
            span_lint(
                cx,
                SLICE_RANGE_OUT_OF_BOUNDS,
                expr.span,
                "slice range without validation; ensure range is within bounds",
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for BoundsSafety {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        check_index_expr(cx, expr);
        check_direct_index(cx, expr);
        check_range_index(cx, expr);
    }
}
