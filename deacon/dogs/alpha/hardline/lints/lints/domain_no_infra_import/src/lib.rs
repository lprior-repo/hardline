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

fn check_infra_import(cx: &LateContext<'_>, qpath: &QPath, span: Span) {
    if_chain! {
        if is_in_domain_dir(cx);
        if let QPath::Resolved(_, path) = qpath;
        if let Some(segment) = path.segments.last();
        let ident = segment.ident.name.as_str();
        if ident.starts_with("tokio")
            || ident.starts_with("sqlx")
            || ident.starts_with("reqwest")
            || ident == "async_trait";
        then {
            span_lint(
                cx,
                DOMAIN_NO_INFRA_IMPORT,
                span,
                &format!("Infrastructure crate `{}` imported in domain layer", ident),
            );
        }
    }
}

dylint_linting::declare_late_lint! {
    pub DOMAIN_NO_INFRA_IMPORT,
    Warn,
    "checks for tokio, sqlx, reqwest, async_trait imports in domain layer",
    DOMAIN_NO_INFRA_IMPORT_INTERNAL: () = (
        "domain_no_infra_import",
        "Warn",
        "Infrastructure crates (tokio, sqlx, reqwest, async_trait) should not be imported in domain layer"
    )
}

impl<'tcx> LateLintPass<'tcx> for DomainNoInfraImport {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if let ExprKind::Path(qpath) = expr.kind {
            check_infra_import(cx, &qpath, expr.span);
        }
    }

    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'_>) {
        if let ItemKind::Use(use_tree) = item.kind {
            // Check use statements
        }
    }
}
