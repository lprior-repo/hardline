#![feature(rustc_private)]
#![recursion_limit = "512"]

use clippy_utils::diagnostics::span_lint;
use if_chain::if_chain;
use rustc_ast::{ast::ItemKind, GenericArg, QPath};
use rustc_hir as hir;
use rustc_hir::{def::Res, Expr, ExprKind, LangItem, MatchSource};
use rustc_lint::LateContext;
use std::path::Path;

fn check_state_enum(cx: &LateContext<'_>, adt: &hir::AdtDef, span: Span) {
    if_chain! {
        if adt.is_enum();
        let enum_name = cx.tcx.item_name(adt.did()).as_str();
        if enum_name.starts_with("State") || enum_name.ends_with("State") || enum_name.ends_with("Status");
        then {
            let has_phantom = adt.variants().any(|variant| {
                variant.fields.iter().any(|field| {
                    if let hir::TyKind::Path(hir::QPath::Resolved(_, path)) = field.ty.kind {
                        if let Some(segment) = path.segments.last() {
                            return segment.ident.name.as_str() == "PhantomData";
                        }
                    }
                    false
                })
            });
            if !has_phantom {
                span_lint(
                    cx,
                    TYPESTATE_REQUIRED,
                    span,
                    &format!("State enum `{}` should have a PhantomData marker for type safety", enum_name),
                );
            }
        }
    }
}

dylint_linting::declare_late_lint! {
    pub TYPESTATE_REQUIRED,
    Warn,
    "checks for state enums without PhantomData marker",
    TYPESTATE_REQUIRED_INTERNAL: () = (
        "typestate_required",
        "Warn",
        "State enums (named State*, *State, *Status) should have PhantomData marker for typestate pattern"
    )
}

impl<'tcx> LateLintPass<'tcx> for TypestateRequired {
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx hir::Item<'_>) {
        if let ItemKind::Enum(enum_def, _) = item.kind {
            let adt = cx.tcx.adt_def(item.def_id);
            check_state_enum(cx, adt, item.span);
        }
    }
}
