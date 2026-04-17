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
    }

    // ── Adversarial: handle_key_event through public API ──

    #[test]
    fn handle_key_event_j_navigates_next() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::NavigateNext));
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn handle_key_event_down_navigates_next() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Down, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 2;
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::NavigateNext));
        assert_eq!(handler.current_hunk, 3);
    }

    #[test]
    fn handle_key_event_k_navigates_prev() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('k'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 2;
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::NavigatePrev));
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn handle_key_event_space_stages() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char(' '), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::Stage));
    }

    #[test]
    fn handle_key_event_s_stages() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('s'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::Stage));
    }

    #[test]
    fn handle_key_event_uppercase_s_is_unhandled() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('S'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Unhandled);
    }

    #[test]
    fn handle_key_event_d_discards() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('d'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::Discard));
    }

    #[test]
    fn handle_key_event_uppercase_d_discards() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('D'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::Discard));
    }

    #[test]
    fn handle_key_event_u_unstages() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('u'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::Unstage));
    }

    #[test]
    fn handle_key_event_esc_quits() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Quit);
    }

    // ── Adversarial: unhandled keys ──

    #[test]
    fn handle_key_event_enter_unhandled() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Unhandled);
    }

    #[test]
    fn handle_key_event_tab_unhandled() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Tab, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Unhandled);
    }

    #[test]
    fn handle_key_event_backspace_unhandled() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Backspace, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Unhandled);
    }

    #[test]
    fn handle_key_event_delete_unhandled() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Delete, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Unhandled);
    }

    #[test]
    fn handle_key_event_f1_unhandled() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::F(1), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Unhandled);
    }

    #[test]
    fn handle_key_event_null_unhandled() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Null, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Unhandled);
    }

    // ── Adversarial: modifier keys ignored ──

    #[test]
    fn handle_key_event_ctrl_j_still_navigates() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        // Current behavior: modifiers are ignored, 'j' still maps to NavigateNext
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::NavigateNext));
    }

    // ── Adversarial: single hunk navigation ──

    #[test]
    fn single_hunk_navigation_stays_at_zero() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(1);
        for _ in 0..100 {
            handler.navigate_next();
        }
        assert_eq!(handler.current_hunk, 0);
        for _ in 0..100 {
            handler.navigate_prev();
        }
        assert_eq!(handler.current_hunk, 0);
    }

    // ── Adversarial: zero hunk navigation ──

    #[test]
    fn zero_hunk_navigation_is_noop() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(0);
        handler.navigate_next();
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 0);
    }

    // ── Adversarial: page up/down ──

    #[test]
    fn handle_key_event_pageup_scrolls_up() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::PageUp, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::ScrollUp));
    }

    #[test]
    fn handle_key_event_pagedown_scrolls_down() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::PageDown, crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::ScrollDown));
    }

    #[test]
    fn handle_key_event_b_scrolls_up() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('b'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::ScrollUp));
    }

    #[test]
    fn handle_key_event_f_scrolls_down() {
        use crossterm::event::{KeyCode, KeyEvent};
        let key = KeyEvent::new(KeyCode::Char('f'), crossterm::event::KeyModifiers::empty());
        let mut handler = InputHandler::new();
        assert_eq!(handler.handle_key_event(key), InputResult::Handled(HunkAction::ScrollDown));
    }

    // ── Proptests ──

    use proptest::proptest;

    proptest! {
        #[test]
        fn prop_handle_key_event_never_panics(
            code_byte in 0u8..=127u8,
        ) {
            use crossterm::event::{KeyCode, KeyEvent};
            let code = KeyCode::Char(code_byte as char);
            let key = KeyEvent::new(code, crossterm::event::KeyModifiers::empty());
            let mut handler = InputHandler::new();
            handler.set_hunk_count(5);
            let _result = handler.handle_key_event(key);
        }

        #[test]
        fn prop_set_hunk_count_arbitrary_usize(
            count in 0usize..1000usize,
        ) {
            let mut handler = InputHandler::new();
            handler.set_hunk_count(count);
            // Should never panic, even with count = 0
            handler.navigate_next();
            handler.navigate_prev();
        }
    }
}
