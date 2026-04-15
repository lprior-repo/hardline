pub mod stack_tree {
    #[derive(Debug, Clone, Copy)]
    pub struct StackTree;
}

pub mod diff {
    #[derive(Debug, Clone, Copy)]
    pub struct DiffView;
}

pub mod details {
    #[derive(Debug, Clone, Copy)]
    pub struct DetailsView;
}

pub mod reorder_preview {
    #[derive(Debug, Clone, Copy)]
    pub struct ReorderPreview;
}

pub mod agents {
    #[derive(Debug, Clone, Copy)]
    pub struct AgentsView;
}

pub mod worktree;
pub use worktree::WorktreeItem;

#[cfg(test)]
mod tests {
    use super::{
        agents::AgentsView, details::DetailsView, diff::DiffView, reorder_preview::ReorderPreview,
        stack_tree::StackTree,
    };

    #[test]
    fn widget_structs_are_constructible() {
        let _stack = StackTree;
        let _diff = DiffView;
        let _details = DetailsView;
        let _reorder = ReorderPreview;
        let _agents = AgentsView;
    }

    #[test]
    fn widget_structs_are_zero_sized() {
        assert_eq!(std::mem::size_of::<StackTree>(), 0);
        assert_eq!(std::mem::size_of::<DiffView>(), 0);
        assert_eq!(std::mem::size_of::<DetailsView>(), 0);
        assert_eq!(std::mem::size_of::<ReorderPreview>(), 0);
        assert_eq!(std::mem::size_of::<AgentsView>(), 0);
    }

    #[test]
    fn widget_structs_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<StackTree>();
        assert_send::<DiffView>();
        assert_send::<DetailsView>();
        assert_send::<ReorderPreview>();
        assert_send::<AgentsView>();
    }

    #[test]
    fn widget_structs_are_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<StackTree>();
        assert_sync::<DiffView>();
        assert_sync::<DetailsView>();
        assert_sync::<ReorderPreview>();
        assert_sync::<AgentsView>();
    }

    #[test]
    fn widget_module_names_are_accessible() {
        // Verify the module hierarchy is properly organized
        let _ = std::any::type_name::<StackTree>();
        let _ = std::any::type_name::<DiffView>();
        let _ = std::any::type_name::<DetailsView>();
        let _ = std::any::type_name::<ReorderPreview>();
        let _ = std::any::type_name::<AgentsView>();
    }

    #[test]
    fn widget_structs_are_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<StackTree>();
        assert_copy::<DiffView>();
        assert_copy::<DetailsView>();
        assert_copy::<ReorderPreview>();
        assert_copy::<AgentsView>();
    }

    #[test]
    fn widget_structs_are_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<StackTree>();
        assert_clone::<DiffView>();
        assert_clone::<DetailsView>();
        assert_clone::<ReorderPreview>();
        assert_clone::<AgentsView>();
    }

    #[test]
    fn widget_structs_debug_format() {
        let _ = format!("{:?}", StackTree);
        let _ = format!("{:?}", DiffView);
        let _ = format!("{:?}", DetailsView);
        let _ = format!("{:?}", ReorderPreview);
        let _ = format!("{:?}", AgentsView);
    }

    #[test]
    fn widget_structs_alignment_is_one() {
        assert_eq!(std::mem::align_of::<StackTree>(), 1);
        assert_eq!(std::mem::align_of::<DiffView>(), 1);
        assert_eq!(std::mem::align_of::<DetailsView>(), 1);
        assert_eq!(std::mem::align_of::<ReorderPreview>(), 1);
        assert_eq!(std::mem::align_of::<AgentsView>(), 1);
    }

    #[test]
    fn widget_structs_in_option() {
        let _s: Option<StackTree> = Some(StackTree);
        let _d: Option<DiffView> = None;
        let _dt: Option<DetailsView> = Some(DetailsView);
        let _r: Option<ReorderPreview> = None;
        let _a: Option<AgentsView> = Some(AgentsView);
    }

    #[test]
    fn widget_structs_in_vec() {
        let _widgets: Vec<&str> = vec![];
        // Each widget can be constructed many times
        let _s = vec![StackTree, StackTree, StackTree];
        let _d = vec![DiffView; 5];
        assert_eq!(_d.len(), 5);
    }

    #[test]
    fn widget_structs_in_box() {
        let _s = Box::new(StackTree);
        let _d = Box::new(DiffView);
        let _dt = Box::new(DetailsView);
        let _r = Box::new(ReorderPreview);
        let _a = Box::new(AgentsView);
    }

    #[test]
    fn widget_type_names_contain_expected_names() {
        let s = std::any::type_name::<StackTree>();
        let d = std::any::type_name::<DiffView>();
        let dt = std::any::type_name::<DetailsView>();
        let r = std::any::type_name::<ReorderPreview>();
        let a = std::any::type_name::<AgentsView>();
        assert!(s.contains("StackTree"), "got: {s}");
        assert!(d.contains("DiffView"), "got: {d}");
        assert!(dt.contains("DetailsView"), "got: {dt}");
        assert!(r.contains("ReorderPreview"), "got: {r}");
        assert!(a.contains("AgentsView"), "got: {a}");
    }

    #[test]
    fn widget_structs_have_distinct_type_names() {
        let names = vec![
            std::any::type_name::<StackTree>(),
            std::any::type_name::<DiffView>(),
            std::any::type_name::<DetailsView>(),
            std::any::type_name::<ReorderPreview>(),
            std::any::type_name::<AgentsView>(),
        ];
        // All type names should be unique
        for i in 0..names.len() {
            for j in (i + 1)..names.len() {
                assert_ne!(
                    names[i], names[j],
                    "type names at {i} and {j} should differ"
                );
            }
        }
    }

    #[test]
    fn widget_structs_in_tuple() {
        let _tuple: (StackTree, DiffView, DetailsView, ReorderPreview, AgentsView) =
            (StackTree, DiffView, DetailsView, ReorderPreview, AgentsView);
    }

    #[test]
    fn widget_structs_in_array() {
        let _arr: [StackTree; 3] = [StackTree, StackTree, StackTree];
    }
}
