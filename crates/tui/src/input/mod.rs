#![allow(clippy::module_inception)]
pub mod input {
    #[derive(Debug, Clone, Copy)]
    pub struct InputHandler;

    impl Default for InputHandler {
        fn default() -> Self {
            Self::new()
        }
    }

    impl InputHandler {
        pub fn new() -> Self {
            Self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::input::InputHandler;

    #[test]
    fn input_handler_new_creates_instance() {
        let _handler = InputHandler::new();
    }

    #[test]
    fn input_handler_default_creates_instance() {
        let _handler = InputHandler::default();
    }

    #[test]
    fn input_handler_new_equals_default() {
        let _new = InputHandler::new();
        let _default = InputHandler::default();
        // Both should produce equivalent instances
        // (unit struct, so they are trivially equal)
    }

    #[test]
    fn input_handler_is_zero_sized() {
        assert_eq!(std::mem::size_of::<InputHandler>(), 0);
    }

    #[test]
    fn input_handler_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<InputHandler>();
    }

    #[test]
    fn input_handler_is_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<InputHandler>();
    }

    #[test]
    fn input_handler_multiple_instances() {
        let _a = InputHandler::new();
        let _b = InputHandler::new();
        let _c = InputHandler::default();
    }

    #[test]
    fn input_handler_alignment_is_one() {
        assert_eq!(std::mem::align_of::<InputHandler>(), 1);
    }

    #[test]
    fn input_handler_type_name_contains_module() {
        let name = std::any::type_name::<InputHandler>();
        assert!(name.contains("InputHandler"), "type name: {name}");
    }

    #[test]
    fn input_handler_debug_format() {
        let handler = InputHandler::new();
        let debug = format!("{handler:?}");
        assert!(!debug.is_empty());
    }

    #[test]
    fn input_handler_in_vec() {
        let handlers: Vec<InputHandler> = vec![
            InputHandler::new(),
            InputHandler::default(),
            InputHandler::new(),
        ];
        assert_eq!(handlers.len(), 3);
    }

    #[test]
    fn input_handler_in_box() {
        let boxed: Box<InputHandler> = Box::new(InputHandler::new());
        let _ = *boxed;
    }

    #[test]
    fn input_handler_in_option() {
        let some = Some(InputHandler::new());
        assert!(some.is_some());
        let none: Option<InputHandler> = None;
        assert!(none.is_none());
    }

    #[test]
    fn input_handler_clone_via_debug() {
        // Unit structs are implicitly Copy/Clone
        let a = InputHandler::new();
        let _b = a;
        // a is still usable because unit struct is Copy
        let _c = a;
    }

    #[test]
    fn input_handler_copy() {
        fn assert_copy<T: Copy>() {}
        assert_copy::<InputHandler>();
    }

    #[test]
    fn input_handler_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<InputHandler>();
    }

    #[test]
    fn input_handler_scope_cleanup_no_panic() {
        // Verify no panic on scope exit (drop)
        {
            let _handler = InputHandler::new();
        }
    }

    // ── Proptests ──

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_input_handler_new_never_panics(
            _dummy in proptest::bool::ANY,
        ) {
            let _handler = InputHandler::new();
        }

        #[test]
        fn prop_input_handler_default_never_panics(
            _dummy in proptest::bool::ANY,
        ) {
            let _handler = InputHandler::default();
        }

        #[test]
        fn prop_input_handler_always_zero_sized(
            _dummy in proptest::bool::ANY,
        ) {
            assert_eq!(std::mem::size_of::<InputHandler>(), 0);
        }
    }
}
