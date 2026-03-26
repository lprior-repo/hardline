#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_ast as ast;
use rustc_hir as hir;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::LateContext;

dylint_linting::declare_late_lint! {
    pub NO_STRING_ALLOC_IN_HOT_PATH,
    Warn,
    "checks for format!() in hot paths",
    NO_STRING_ALLOC_IN_HOT_PATH_INTERNAL: () = (
        "string_performance",
        "Warn",
        "format!() allocation detected in hot path; consider using stack buffers or lazy allocation"
    )
}

dylint_linting::declare_late_lint! {
    pub STRING_CONCAT_IN_LOOP,
    Warn,
    "checks for push_str/format in loop instead of join",
    STRING_CONCAT_IN_LOOP_INTERNAL: () = (
        "string_performance",
        "Warn",
        "string concatenation in loop detected; use Iterator::join() instead"
    )
}

dylint_linting::declare_late_lint! {
    pub NO_TO_STRING_INSTEAD_OF_DISPLAY,
    Warn,
    "checks for .to_string() when Display trait is available",
    NO_TO_STRING_INSTEAD_OF_DISPLAY_INTERNAL: () = (
        "string_performance",
        "Warn",
        ".to_string() called but Display trait is available; use write! or format with Display"
    )
}

dylint_linting::declare_late_lint! {
    pub UNNECESSARY_STRING_CONVERSION,
    Warn,
    "checks for .to_string() on already String type",
    UNNECESSARY_STRING_CONVERSION_INTERNAL: () = (
        "string_performance",
        "Warn",
        ".to_string() called on a String type; this is unnecessary"
    )
}

fn is_in_loop(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let mut parent = cx.tcx.hir().parent_iter(expr.hir_id);
    while let Some((id, node)) = parent.next() {
        match node {
            hir::Node::Expr(e)
                if matches!(
                    e.kind,
                    ExprKind::ForLoop { .. } | ExprKind::WhileLoop { .. } | ExprKind::Loop(..)
                ) =>
            {
                return true
            }
            hir::Node::Block(b)
                if b.rules == hir::BlockCheckMode::UncheckedSafeBlock
                    || b.rules == hir::BlockCheckMode::UnsafeBlock(_, _) => {}
            _ => {}
        }
        if matches!(
            node,
            hir::Node::Item(_) | hir::Node::TraitItem(_) | hir::Node::ImplItem(_)
        ) {
            break;
        }
    }
    false
}

fn check_format_in_hot_path(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Call(func, args) = expr.kind;
        if let ExprKind::Path(qpath) = func.kind;
        if cx.qpath_res(&qpath, func.hir_id).opt_def_id().is_some();
        let def_id = cx.qpath_res(&qpath, func.hir_id).def_id();
        if cx.tcx.item_name(def_id).as_str() == "format";
        then {
            span_lint(
                cx,
                NO_STRING_ALLOC_IN_HOT_PATH,
                expr.span,
                "format!() in hot path may cause performance issues",
            );
        }
    }
}

fn check_string_concat_in_loop(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if is_in_loop(cx, expr);
        if let ExprKind::MethodCall(method_name, receiver, args, _) = expr.kind;
        if method_name.ident.name.as_str() == "push_str" || method_name.ident.name.as_str() == "insert_str";
        then {
            span_lint(
                cx,
                STRING_CONCAT_IN_LOOP,
                expr.span,
                "string concatenation in loop; consider using Iterator::join()",
            );
        }
    }
}

fn check_to_string_on_string(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::MethodCall(method_name, receiver, _, _) = expr.kind;
        if method_name.ident.name.as_str() == "to_string";
        if let ExprKind::MethodCall(_, inner_receiver, _, _) = receiver.kind;
        if inner_receiver.ty(ctx).is_str();
        then {
            span_lint(
                cx,
                UNNECESSARY_STRING_CONVERSION,
                expr.span,
                "unnecessary .to_string() on &str",
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for StringPerformance {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        check_format_in_hot_path(cx, expr);
        check_string_concat_in_loop(cx, expr);
        check_to_string_on_string(cx, expr);
    }
}
