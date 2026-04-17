pub mod stack_tree;
<<<<<<< HEAD
<<<<<<< HEAD
pub use stack_tree::{StackTreeWidget, TreeNode};

pub mod diff {
    /// A single line of diff output, with semantic tagging for styling.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum DiffLine {
        Header(String),
        HunkHeader(String),
        Addition(String),
        Deletion(String),
        Context(String),
    }

=======
pub use stack_tree::StackTreeWidget;

pub mod diff {
>>>>>>> polecat/beta
=======
pub use stack_tree::StackTreeWidget;

pub mod diff {
>>>>>>> polecat/theta
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

<<<<<<< HEAD
<<<<<<< HEAD
pub mod agents;
pub use agents::{AgentEntry, AgentsView};
=======
=======
>>>>>>> polecat/theta
pub mod agents {
    #[derive(Debug, Clone, Copy)]
    pub struct AgentsView;
}
<<<<<<< HEAD
>>>>>>> polecat/beta
=======
>>>>>>> polecat/theta

pub mod worktree;
pub use worktree::WorktreeItem;

#[cfg(test)]
mod tests {
    use super::{
<<<<<<< HEAD
<<<<<<< HEAD
        AgentsView, details::DetailsView, diff::DiffView, reorder_preview::ReorderPreview,
=======
        agents::AgentsView, details::DetailsView, diff::DiffView, reorder_preview::ReorderPreview,
>>>>>>> polecat/beta
=======
        agents::AgentsView, details::DetailsView, diff::DiffView, reorder_preview::ReorderPreview,
>>>>>>> polecat/theta
        StackTreeWidget,
    };

    #[test]
    fn widget_structs_are_constructible() {
        let _stack = StackTreeWidget::new(Vec::new());
        let _diff = DiffView;
        let _details = DetailsView;
        let _reorder = ReorderPreview;
<<<<<<< HEAD
        let _agents = AgentsView::new(Vec::new());
=======
        let _agents = AgentsView;
<<<<<<< HEAD
>>>>>>> polecat/beta
=======
>>>>>>> polecat/theta
    }

    #[test]
    fn widget_structs_are_send() {
        fn assert_send<T: Send>() {}
        assert_send::<StackTreeWidget>();
        assert_send::<DiffView>();
        assert_send::<DetailsView>();
        assert_send::<ReorderPreview>();
        assert_send::<AgentsView>();
    }

    #[test]
    fn widget_structs_are_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<StackTreeWidget>();
        assert_sync::<DiffView>();
        assert_sync::<DetailsView>();
        assert_sync::<ReorderPreview>();
        assert_sync::<AgentsView>();
    }

    #[test]
    fn widget_module_names_are_accessible() {
        let _ = std::any::type_name::<StackTreeWidget>();
        let _ = std::any::type_name::<DiffView>();
        let _ = std::any::type_name::<DetailsView>();
        let _ = std::any::type_name::<ReorderPreview>();
        let _ = std::any::type_name::<AgentsView>();
    }

    #[test]
    fn widget_structs_are_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<StackTreeWidget>();
        assert_clone::<DiffView>();
        assert_clone::<DetailsView>();
        assert_clone::<ReorderPreview>();
        assert_clone::<AgentsView>();
    }

    #[test]
    fn widget_structs_debug_format() {
        let _ = format!("{:?}", StackTreeWidget::new(Vec::new()));
        let _ = format!("{:?}", DiffView);
        let _ = format!("{:?}", DetailsView);
        let _ = format!("{:?}", ReorderPreview);
<<<<<<< HEAD
        let _ = format!("{:?}", AgentsView::new(Vec::new()));
=======
        let _ = format!("{:?}", AgentsView);
<<<<<<< HEAD
>>>>>>> polecat/beta
=======
>>>>>>> polecat/theta
    }

    #[test]
    fn widget_structs_in_option() {
        let _s: Option<StackTreeWidget> = Some(StackTreeWidget::new(Vec::new()));
        let _d: Option<DiffView> = None;
        let _dt: Option<DetailsView> = Some(DetailsView);
        let _r: Option<ReorderPreview> = None;
        let _a: Option<AgentsView> = Some(AgentsView::new(Vec::new()));
    }

    #[test]
    fn widget_structs_in_vec() {
        let _widgets: Vec<&str> = vec![];
        let _s = vec![
            StackTreeWidget::new(Vec::new()),
            StackTreeWidget::new(Vec::new()),
        ];
        let _d = vec![DiffView; 5];
        assert_eq!(_d.len(), 5);
    }

    #[test]
    fn widget_structs_in_box() {
        let _s = Box::new(StackTreeWidget::new(Vec::new()));
        let _d = Box::new(DiffView);
        let _dt = Box::new(DetailsView);
        let _r = Box::new(ReorderPreview);
        let _a = Box::new(AgentsView::new(Vec::new()));
    }

    #[test]
    fn widget_type_names_contain_expected_names() {
        let s = std::any::type_name::<StackTreeWidget>();
        let d = std::any::type_name::<DiffView>();
        let dt = std::any::type_name::<DetailsView>();
        let r = std::any::type_name::<ReorderPreview>();
        let a = std::any::type_name::<AgentsView>();
        assert!(s.contains("StackTreeWidget"), "got: {s}");
        assert!(d.contains("DiffView"), "got: {d}");
        assert!(dt.contains("DetailsView"), "got: {dt}");
        assert!(r.contains("ReorderPreview"), "got: {r}");
        assert!(a.contains("AgentsView"), "got: {a}");
    }

    #[test]
    fn widget_structs_have_distinct_type_names() {
        let names = vec![
            std::any::type_name::<StackTreeWidget>(),
            std::any::type_name::<DiffView>(),
            std::any::type_name::<DetailsView>(),
            std::any::type_name::<ReorderPreview>(),
            std::any::type_name::<AgentsView>(),
        ];
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
        let _tuple: (
            StackTreeWidget,
            DiffView,
            DetailsView,
            ReorderPreview,
            AgentsView,
        ) = (
            StackTreeWidget::new(Vec::new()),
            DiffView,
            DetailsView,
            ReorderPreview,
<<<<<<< HEAD
<<<<<<< HEAD
            AgentsView::new(Vec::new()),
=======
            AgentsView,
>>>>>>> polecat/beta
=======
            AgentsView,
>>>>>>> polecat/theta
        );
    }

    #[test]
    fn stack_tree_widget_new_and_with_selection() {
        let widget = StackTreeWidget::new(Vec::new());
        let selected = widget.with_selection(Some(0));
        assert_eq!(selected.selected_index, Some(0));
    }
}
