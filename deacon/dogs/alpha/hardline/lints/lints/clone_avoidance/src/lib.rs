#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir as hir;
use rustc_hir::{def::Res, Expr, ExprKind};
use rustc_lint::LateContext;

dylint_linting::declare_late_lint! {
    pub NO_CLONE_WITHOUT_NEED,
    Warn,
    "checks for .clone() calls without necessity",
    NO_CLONE_WITHOUT_NEED_INTERNAL: () = (
        "clone_avoidance",
        "Warn",
        "unnecessary .clone() detected; consider using references or owned value directly"
    )
}

dylint_linting::declare_late_lint! {
    pub EXCESSIVE_CLONE_IN_LOOP,
    Warn,
    "checks for clone() inside loops",
    EXCESSIVE_CLONE_IN_LOOP_INTERNAL: () = (
        "clone_avoidance",
        "Warn",
        "clone() called inside loop; this may cause performance issues"
    )
}

dylint_linting::declare_late_lint! {
    pub CLONE_ON_HOT_PATH,
    Warn,
    "checks for .clone() in frequently called code",
    CLONE_ON_HOT_PATH_INTERNAL: () = (
        "clone_avoidance",
        "Warn",
        ".clone() in hot path detected; consider using Rc, Arc, or borrowing"
    )
}

dylint_linting::declare_late_lint! {
    pub NO_ARC_WHERE_REFS_WORKS,
    Warn,
    "checks for Arc::clone() when &T would work",
    NO_ARC_WHERE_REFS_WORKS_INTERNAL: () = (
        "clone_avoidance",
        "Warn",
        "Arc::clone() used but &T would suffice; consider removing Arc"
    )
}

fn is_in_loop(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let mut parent = cx.tcx.hir().parent_iter(expr.hir_id);
    while let Some((_, node)) = parent.next() {
        if matches!(node, hir::Node::Expr(e) if matches!(e.kind, ExprKind::ForLoop { .. } | ExprKind::WhileLoop { .. } | ExprKind::Loop(..)))
        {
            return true;
        }
    }
    false
}

fn check_clone_method(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::MethodCall(method_name, receiver, _, _) = expr.kind;
        if method_name.ident.name.as_str() == "clone";
        then {
            if is_in_loop(cx, expr) {
                span_lint(
                    cx,
                    EXCESSIVE_CLONE_IN_LOOP,
                    expr.span,
                    "clone() inside loop; consider refactoring to avoid repeated allocations",
                );
            } else {
                span_lint(
                    cx,
                    NO_CLONE_WITHOUT_NEED,
                    expr.span,
                    "unnecessary clone detected; consider using a reference",
                );
            }
        }
    }
}

fn check_arc_clone(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Call(func, _) = expr.kind;
        if let ExprKind::Path(qpath) = func.kind;
        let res = cx.qpath_res(&qpath, func.hir_id);
        if let Res::Def(_, def_id) = res;
        if cx.tcx.item_name(def_id).as_str() == "clone";
        if let Some(assoc) = cx.tcx.opt_associated_item(def_id);
        if assoc.trait_item_def_id.is_some();
        then {
            span_lint(
                cx,
                NO_ARC_WHERE_REFS_WORKS,
                expr.span,
                "Arc::clone() used but &T would work; consider removing Arc",
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for CloneAvoidance {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        check_clone_method(cx, expr);
        check_arc_clone(cx, expr);
    }
}
