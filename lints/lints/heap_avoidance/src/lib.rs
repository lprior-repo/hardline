#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_hir::{def::Res, Expr, ExprKind};
use rustc_lint::LateContext;

dylint_linting::declare_late_lint! {
    pub NO_HEAP_IN_CALC,
    Warn,
    "checks for Vec/String/Box in calculation layer",
    NO_HEAP_IN_CALC_INTERNAL: () = (
        "heap_avoidance",
        "Warn",
        "heap allocation (Vec/String/Box) detected in calculation layer; use stack-allocated alternatives"
    )
}

dylint_linting::declare_late_lint! {
    pub UNNECESSARY_ALLOCATION,
    Warn,
    "checks for Box::new() when stack works",
    UNNECESSARY_ALLOCATION_INTERNAL: () = (
        "heap_avoidance",
        "Warn",
        "Box::new() used but stack allocation would suffice"
    )
}

dylint_linting::declare_late_lint! {
    pub VEC_ALLOC_WITHOUT_CAPACITY,
    Warn,
    "checks for Vec::new() with known size",
    VEC_ALLOC_WITHOUT_CAPACITY_INTERNAL: () = (
        "heap_avoidance",
        "Warn",
        "Vec::new() with known capacity; use Vec::with_capacity() to avoid reallocation"
    )
}

fn is_in_calc_dir(cx: &LateContext<'_>) -> bool {
    if let Some(path) = cx.tcx.sess.local_stable_crate_id().as_u64() {
        let source_map = cx.tcx.sess.source_map();
        if let Some(file) = source_map
            .span_to_snippet(cx.tcx.def_span(path.into()))
            .ok()
        {
            return std::path::Path::new(&file)
                .components()
                .any(|c| c.as_os_str() == "calc");
        }
    }
    false
}

fn check_heap_allocation(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Call(func, _) = expr.kind;
        if let ExprKind::Path(qpath) = func.kind;
        let res = cx.qpath_res(&qpath, func.hir_id);
        if let Res::Def(_, def_id) = res;
        let name = cx.tcx.item_name(def_id).as_str();
        if name == "new" || name == "with_capacity" || name == "with_capacity_and_align";
        if let Some(assoc) = cx.tcx.opt_associated_item(def_id);
        if let Some(trait_def_id) = assoc.trait_item_def_id;
        let trait_name = cx.tcx.item_name(trait_def_id).as_str();
        if trait_name == "Default" || trait_name == "Extend";
        then {
            span_lint(
                cx,
                NO_HEAP_IN_CALC,
                expr.span,
                "heap allocation in calculation layer; prefer stack-allocated alternatives",
            );
        }
    }
}

fn check_unnecessary_box(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Call(func, _) = expr.kind;
        if let ExprKind::Path(qpath) = func.kind;
        let res = cx.qpath_res(&qpath, func.hir_id);
        if let Res::Def(_, def_id) = res;
        let name = cx.tcx.item_name(def_id).as_str();
        if name == "new";
        if let Some(assoc) = cx.tcx.opt_associated_item(def_id);
        if assoc.container == hir::DefKind::Struct;
        then {
            span_lint(
                cx,
                UNNECESSARY_ALLOCATION,
                expr.span,
                "Box::new() used but stack allocation would suffice",
            );
        }
    }
}

fn check_vec_without_capacity(cx: &LateContext<'_>, expr: &Expr<'_>) {
    if_chain! {
        if let ExprKind::Call(func, _) = expr.kind;
        if let ExprKind::Path(qpath) = func.kind;
        let res = cx.qpath_res(&qpath, func.hir_id);
        if let Res::Def(_, def_id) = res;
        if cx.tcx.item_name(def_id).as_str() == "new";
        if let Some(assoc) = cx.tcx.opt_associated_item(def_id);
        if assoc.container == hir::DefKind::Struct;
        let type_name = cx.tcx.item_name(assoc.container_id()).as_str();
        if type_name == "Vec";
        then {
            span_lint(
                cx,
                VEC_ALLOC_WITHOUT_CAPACITY,
                expr.span,
                "Vec::new() called; if size is known, use Vec::with_capacity()",
            );
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for HeapAvoidance {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        if is_in_calc_dir(cx) {
            check_heap_allocation(cx, expr);
        }
        check_unnecessary_box(cx, expr);
        check_vec_without_capacity(cx, expr);
    }
}
