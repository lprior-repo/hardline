#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind, ItemKind};
use rustc_lint::LateContext;
use std::path::Path;

dylint_linting::declare_late_lint! {
    pub FUNCTION_LENGTH_LIMIT,
    Warn,
    "warns at 20 lines, denies at 25",
    FUNCTION_LENGTH_LIMIT_INTERNAL: () = (
        "complexity",
        "Warn",
        "function exceeds 20 lines; consider refactoring"
    )
}

dylint_linting::declare_late_lint! {
    pub PARAMETER_COUNT_LIMIT,
    Warn,
    "warns when function has >5 parameters",
    PARAMETER_COUNT_LIMIT_INTERNAL: () = (
        "complexity",
        "Warn",
        "function has more than 5 parameters; consider using a struct or config object"
    )
}

dylint_linting::declare_late_lint! {
    pub NO_DEEP_NESTING,
    Warn,
    "warns when nesting exceeds 2 levels",
    NO_DEEP_NESTING_INTERNAL: () = (
        "complexity",
        "Warn",
        "deep nesting detected (>2 levels); consider extracting functions"
    )
}

dylint_linting::declare_late_lint! {
    pub COGNITIVE_COMPLEXITY_LIMIT,
    Warn,
    "warns when cognitive complexity exceeds 15",
    COGNITIVE_COMPLEXITY_LIMIT_INTERNAL: () = (
        "complexity",
        "Warn",
        "cognitive complexity exceeds 15; function is too complex"
    )
}

fn count_lines_in_fn(cx: &LateContext<'_>, item: &rustc_hir::FnItem) -> usize {
    if let Some(body) = item.body {
        let span = body.span;
        let source_map = cx.tcx.sess.source_map();
        source_map.bytepos_to_file_charpos(span.hi() - span.lo()).0 as usize
    } else {
        0
    }
}

fn check_function_length(cx: &LateContext<'_>, item: &rustc_hir::FnItem<'_>) {
    let lines = count_lines_in_fn(cx, item);
    if lines > 25 {
        span_lint(
            cx,
            FUNCTION_LENGTH_LIMIT,
            item.span,
            "function exceeds 25 lines (deny level)",
        );
    } else if lines > 20 {
        span_lint(
            cx,
            FUNCTION_LENGTH_LIMIT,
            item.span,
            "function exceeds 20 lines",
        );
    }
}

fn check_parameter_count(cx: &LateContext<'_>, item: &rustc_hir::FnItem<'_>) {
    let param_count = item.decl.inputs.len();
    if param_count > 5 {
        span_lint(
            cx,
            PARAMETER_COUNT_LIMIT,
            item.span,
            "function has more than 5 parameters",
        );
    }
}

fn get_nesting_depth(expr: &Expr<'_>, cx: &LateContext<'_>) -> usize {
    let mut max_depth = 0;
    let mut current_depth = 0;

    fn walk_expr(expr: &Expr<'_>, cx: &LateContext<'_>, current: &mut usize, max: &mut usize) {
        match expr.kind {
            ExprKind::If(..) | ExprKind::Match(..) | ExprKind::Loop(..) => {
                *current += 1;
                *max = (*max).max(*current);
            }
            _ => {}
        }
        for child in cx.tcx.hir().body_exprs(expr.hir_id) {
            walk_expr(child, cx, current, max);
        }
    }

    walk_expr(expr, cx, &mut current_depth, &mut max_depth);
    max_depth
}

fn check_nesting_depth(cx: &LateContext<'_>, expr: &Expr<'_>) {
    let depth = get_nesting_depth(expr, cx);
    if depth > 2 {
        span_lint(
            cx,
            NO_DEEP_NESTING,
            expr.span,
            "deep nesting detected (>2 levels); consider extracting functions",
        );
    }
}

impl<'tcx> LateLintPass<'tcx> for Complexity {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: hir::intravisit::FnKind<'tcx>,
        _: &rustc_hir::FnDecl<'_>,
        item: &rustc_hir::ItemId,
        _: rustc_span::Span,
        _: rustc_hir::HirId,
    ) {
        if let hir::intravisit::FnKind::ItemFn(item_name, _, _, _) = kind {
            let item = cx.tcx.hir().item(*item);
            if let ItemKind::Fn(_) = item.kind {
                let fn_item = cx.tcx.hir().fn_item(*item);
                check_function_length(cx, fn_item);
                check_parameter_count(cx, fn_item);
            }
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        check_nesting_depth(cx, expr);
    }
}
