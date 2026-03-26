#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_ast::{ast::ItemKind, GenericArg, QPath};
use rustc_hir as hir;
use rustc_hir::{def::Res, Expr, ExprKind, LangItem, MatchSource};
use rustc_lint::LateContext;
use std::path::Path;

fn is_in_domain_dir(cx: &LateContext<'_>) -> bool {
    if let Some(path) = cx.tcx.sess.local_stable_crate_id().as_u64() {
        let source_map = cx.tcx.sess.source_map();
        if let Some(file) = source_map
            .span_to_snippet(cx.tcx.def_span(path.into()))
            .ok()
        {
            return Path::new(&file)
                .components()
                .any(|c| c.as_os_str() == "domain");
        }
    }
    false
}

fn check_unwrap_method(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if !is_in_domain_dir(cx);
        if let ExprKind::MethodCall(method_name, receiver, _, _) = expr.kind;
        if method_name.ident.name.as_str() == "unwrap" || method_name.ident.name.as_str() == "expect";
        then {
            span_lint(
                cx,
                NO_UNWRAP_IN_DOMAIN,
                expr.span,
                "`.unwrap()` or `.expect()` should not be used in domain layer",
            );
        }
    }
}

fn check_call_expr(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if is_in_domain_dir(cx);
        if let ExprKind::Call(func, _) = expr.kind;
        if let ExprKind::Path(qpath) = func.kind;
        let res = cx.qpath_res(&qpath, func.hir_id);
        if let Res::Def(_, def_id) = res;
        let item_def_id = def_id;
        if cx.tcx.item_name(item_def_id).as_str() == "unwrap"
            || cx.tcx.item_name(item_def_id).as_str() == "expect";
        then {
            span_lint(
                cx,
                NO_UNWRAP_IN_DOMAIN,
                expr.span,
                "`.unwrap()` or `.expect()` should not be used in domain layer",
            );
        }
    }
}

dylint_linting::declare_late_lint! {
    pub NO_UNWRAP_IN_DOMAIN,
    Warn,
    "checks for .unwrap() and .expect() calls in domain layer",
    NO_UNWRAP_IN_DOMAIN_INTERNAL: () = (
        "no_unwrap_in_domain",
        "Warn",
        "`.unwrap()` or `.expect()` should not be used in domain layer"
    )
}

impl<'tcx> LateLintPass<'tcx> for NoUnwrapInDomain {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        check_unwrap_method(cx, expr);
        check_call_expr(cx, expr);
    }
}
