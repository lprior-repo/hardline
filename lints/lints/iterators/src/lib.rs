#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;

dylint_linting::declare_late_lint! {
    pub NO_IMPERATIVE_LOOPS,
    Warn,
    "checks for for/while/loop in favor of iterators",
    NO_IMPERATIVE_LOOPS_INTERNAL: () = (
        "iterators",
        "Warn",
        "imperative loop detected; consider using iterator methods (map, filter, collect)"
    )
}

dylint_linting::declare_late_lint! {
    pub ITERATOR_PIPELINE_ENCOURAGED,
    Warn,
    "suggests iterator chains over loops",
    ITERATOR_PIPELINE_ENCOURAGED_INTERNAL: () = (
        "iterators",
        "Warn",
        "loop detected that could be an iterator pipeline; consider .map().filter().collect()"
    )
}

fn check_imperative_loop(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if matches!(expr.kind, ExprKind::ForLoop { .. } | ExprKind::WhileLoop { .. } | ExprKind::Loop(..));
        then {
            span_lint(
                cx,
                NO_IMPERATIVE_LOOPS,
                expr.span,
                "imperative loop detected; prefer iterator methods for better composability",
            );
        }
    }
}

fn check_loop_collection(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::ForLoop { body, .. } = expr.kind;
        if let ExprKind::MethodCall(method_name, _, _, _) = body.kind;
        if method_name.ident.name.as_str() == "push" || method_name.ident.name.as_str() == "insert";
        then {
            span_lint(
                cx,
                ITERATOR_PIPELINE_ENCOURAGED,
                expr.span,
                "loop with push/insert detected; consider iterator .collect() or .extend()",
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for Iterators {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        check_imperative_loop(cx, expr);
        check_loop_collection(cx, expr);
    }
}
