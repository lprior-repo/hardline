use crossterm::event::{KeyCode, KeyEvent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HunkAction {
    Stage,
    Unstage,
    Discard,
    NavigateNext,
    NavigatePrev,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputResult {
    Handled(HunkAction),
    Unhandled,
    Quit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputHandler {
    pub current_hunk: usize,
    pub total_hunks: usize,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            current_hunk: 0,
            total_hunks: 0,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> InputResult {
        match key.code {
            KeyCode::Char(' ') | KeyCode::Char('s') => InputResult::Handled(HunkAction::Stage),
            KeyCode::Char('u') => InputResult::Handled(HunkAction::Unstage),
            KeyCode::Char('d') | KeyCode::Char('D') => InputResult::Handled(HunkAction::Discard),
            KeyCode::Char('j') | KeyCode::Down => {
                self.navigate_next();
                InputResult::Handled(HunkAction::NavigateNext)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.navigate_prev();
                InputResult::Handled(HunkAction::NavigatePrev)
            }
            KeyCode::Char('b') | KeyCode::PageUp => InputResult::Handled(HunkAction::ScrollUp),
            KeyCode::Char('f') | KeyCode::PageDown => InputResult::Handled(HunkAction::ScrollDown),
            KeyCode::Char('q') | KeyCode::Esc => InputResult::Quit,
            _ => InputResult::Unhandled,
        }
    }

    fn navigate_next(&mut self) {
        if self.total_hunks > 0 {
            self.current_hunk = (self.current_hunk + 1) % self.total_hunks;
        }
    }

    fn navigate_prev(&mut self) {
        if self.total_hunks > 0 {
            self.current_hunk = if self.current_hunk == 0 {
                self.total_hunks - 1
            } else {
                self.current_hunk - 1
            };
        }
    }

    pub fn set_hunk_count(&mut self, count: usize) {
        self.total_hunks = count;
        if self.current_hunk >= count && count > 0 {
            self.current_hunk = count - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HunkAction, InputHandler, InputResult};

    #[test]
    fn input_handler_new_creates_instance() {
        let handler = InputHandler::new();
        assert_eq!(handler.current_hunk, 0);
        assert_eq!(handler.total_hunks, 0);
    }

    #[test]
    fn input_handler_default_creates_instance() {
        let handler = InputHandler::default();
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn input_handler_quit() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('q'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Quit);
    }

    #[test]
    fn input_handler_navigate_next() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 0;
        handler.navigate_next();
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn input_handler_navigate_prev() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 1;
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn input_handler_navigate_prev_wraps() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 0;
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 2);
    }

    #[test]
    fn input_handler_navigate_next_wraps() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 2;
        handler.navigate_next();
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn input_handler_set_hunk_count_adjusts_current() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 4;
        handler.set_hunk_count(2);
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn input_handler_set_hunk_count_zero_does_not_crash() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 2;
        handler.set_hunk_count(0);
        assert_eq!(handler.current_hunk, 2);
    }

    #[test]
    fn input_handler_partial_eq() {
        let a = InputHandler::new();
        let b = InputHandler::new();
        assert_eq!(a, b);
    }

    #[test]
    fn hunk_action_partial_eq() {
        assert_eq!(HunkAction::Stage, HunkAction::Stage);
        assert_eq!(HunkAction::Unstage, HunkAction::Unstage);
        assert_ne!(HunkAction::Stage, HunkAction::Unstage);
    }

    #[test]
    fn input_result_partial_eq() {
        assert_eq!(InputResult::Quit, InputResult::Quit);
        assert_eq!(
            InputResult::Handled(HunkAction::Stage),
            InputResult::Handled(HunkAction::Stage)
        );
        assert_ne!(InputResult::Quit, InputResult::Unhandled);
    }

    #[test]
    fn input_handler_debug_format() {
        let handler = InputHandler::new();
        let debug = format!("{handler:?}");
        assert!(!debug.is_empty());
        assert!(debug.contains("InputHandler"));
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
    fn input_handler_clone() {
        let handler = InputHandler::new();
        let cloned = handler.clone();
        assert_eq!(handler.current_hunk, cloned.current_hunk);
<<<<<<< HEAD
    }

    // ── Adversarial ──

    #[test]
    fn adv_navigate_with_zero_hunks() {
        let mut handler = InputHandler::new();
        // total_hunks = 0, navigation should be no-op
        handler.navigate_next();
        assert_eq!(handler.current_hunk, 0);
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn adv_set_hunk_count_one() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 3;
        handler.set_hunk_count(1);
        assert_eq!(handler.total_hunks, 1);
        assert_eq!(handler.current_hunk, 0); // clamped to count-1
    }

    #[test]
    fn adv_set_hunk_count_to_zero_then_navigate() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 3;
        handler.set_hunk_count(0);
        // current_hunk stays at 3, but navigation should be no-op
        handler.navigate_next();
        assert_eq!(handler.current_hunk, 3);
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 3);
    }

    #[test]
    fn adv_set_hunk_count_max() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(usize::MAX);
        handler.current_hunk = usize::MAX - 1;
        handler.navigate_next();
        // Should wrap to 0
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn adv_key_event_modifier_ignorance() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);

        // The handler matches on KeyCode only, ignoring KeyModifiers.
        // So Ctrl+q, Ctrl+Esc, Ctrl+PageUp, Ctrl+PageDown are all still handled.
        let ctrl_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key_event(ctrl_key), InputResult::Quit);

        let ctrl_esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key_event(ctrl_esc), InputResult::Quit);

        let ctrl_page_up = KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key_event(ctrl_page_up), InputResult::Handled(HunkAction::ScrollUp));

        let ctrl_page_down = KeyEvent::new(KeyCode::PageDown, KeyModifiers::CONTROL);
        assert_eq!(handler.handle_key_event(ctrl_page_down), InputResult::Handled(HunkAction::ScrollDown));

        // Truly unhandled: Ctrl+Enter, Ctrl+Backspace, Ctrl+Tab, Ctrl+F-keys, etc.
        let truly_unhandled = vec![
            KeyCode::Enter, KeyCode::Backspace, KeyCode::Tab,
            KeyCode::Insert, KeyCode::Delete, KeyCode::Home,
            KeyCode::End, KeyCode::F(1), KeyCode::F(12), KeyCode::Null,
        ];
        for code in truly_unhandled {
            let key = KeyEvent::new(code, KeyModifiers::CONTROL);
            let result = handler.handle_key_event(key);
            assert_eq!(result, InputResult::Unhandled, "Ctrl+{code:?} should be Unhandled");
        }
    }

    #[test]
    fn adv_navigation_stress_single_hunk() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(1);
        for _ in 0..1000 {
            handler.navigate_next();
            assert_eq!(handler.current_hunk, 0);
            handler.navigate_prev();
            assert_eq!(handler.current_hunk, 0);
        }
    }

    #[test]
    fn adv_navigation_stress_two_hunks() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(2);
        for _ in 0..100 {
            handler.navigate_next();
            assert!(handler.current_hunk < 2);
            handler.navigate_prev();
            assert!(handler.current_hunk < 2);
        }
    }

    #[test]
    fn adv_navigation_stress_large_count() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(10_000);
        handler.current_hunk = 9_999;
        handler.navigate_next();
        assert_eq!(handler.current_hunk, 0); // wraps
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 9_999); // wraps back
=======
>>>>>>> polecat/beta
    }
}
