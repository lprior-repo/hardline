#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_ast::{ast::ItemKind, GenericArg, QPath};
use rustc_hir as hir;
use rustc_hir::{def::Res, Expr, ExprKind, LangItem, MatchSource};
use rustc_lint::LateContext;
use std::path::Path;

fn is_in_domain_context(cx: &LateContext<'_>) -> bool {
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

fn check_cross_crate_domain_import(cx: &LateContext<'_>, qpath: &QPath, span: Span) {
    if_chain! {
        if is_in_domain_context(cx);
        if let QPath::Resolved(_, path) = qpath;
        if path.segments.len() >= 2;
        if let Some(first_seg) = path.segments.first();
        let crate_name = first_seg.ident.name.as_str();
        if crate_name != "crate" && crate_name != "self";
        if let Some(last_seg) = path.segments.last();
        let member_name = last_seg.ident.name.as_str();
        if member_name == "domain" || member_name == "Domain" || member_name == "aggregate" || member_name == "value_object" || member_name == "entity" || member_name == "vo";
        then {
            span_lint(
                cx,
                DDD_BOUNDARY_VIOLATION,
                span,
                &format!("Cross-crate domain import detected: `{}::{}`", crate_name, member_name),
            );
        }
    }
}

dylint_linting::declare_late_lint! {
    pub DDD_BOUNDARY_VIOLATION,
    Warn,
    "checks for cross-crate domain imports violating DDD boundaries",
    DDD_BOUNDARY_VIOLATION_INTERNAL: () = (
        "ddd_boundary_violation",
        "Warn",
        "Cross-crate domain imports violate DDD bounded context boundaries"
    )
}

impl<'tcx> LateLintPass<'tcx> for DddBoundaryViolation {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Path(qpath) = expr.kind {
            check_cross_crate_domain_import(cx, &qpath, expr.span);
        }
    }
}
