use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{ConfirmAction, FocusedPane, InputAction, Mode};

/// Actions that mutate hunk-level state in the diff view.
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

/// Mode transitions that InputHandler can request.
#[derive(Debug, Clone, PartialEq)]
pub enum ModeAction {
    EnterSearch,
    EnterHelp,
    EnterConfirm(ConfirmAction),
    EnterInput(InputAction),
    EnterReorder,
    /// Return to Normal mode from any non-Normal mode.
    ReturnToNormal,
}

/// Discriminated result from handling a single key event.
#[derive(Debug, Clone, PartialEq)]
pub enum InputResult {
    Handled(HunkAction),
    /// Tab was pressed — caller should cycle `FocusedPane`.
    SwitchPane,
    /// A mode transition was requested.
    ModeTransition(ModeAction),
    /// Enter key pressed in Confirm/Input context — caller should confirm.
    Confirm,
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

    /// Dispatch a key event, respecting the current app mode.
    ///
    /// In non-Normal modes, only Escape and Enter are handled (plus mode-
    /// specific pass-through). In Normal mode, the full hunk and mode
    /// keybinding set is available.
    pub fn handle_key_event(&mut self, key: KeyEvent, mode: &Mode) -> InputResult {
        match mode {
            Mode::Normal => self.handle_normal(key),
            _ => self.handle_non_normal(key, mode),
        }
    }

    /// Full keymap available in Normal mode.
    fn handle_normal(&mut self, key: KeyEvent) -> InputResult {
        match key.code {
            KeyCode::Tab => InputResult::SwitchPane,
            KeyCode::Char('/') => InputResult::ModeTransition(ModeAction::EnterSearch),
            KeyCode::Char('?') => InputResult::ModeTransition(ModeAction::EnterHelp),
            KeyCode::Char('i') => InputResult::ModeTransition(ModeAction::EnterInput(InputAction::Rename)),
            KeyCode::Char('R') => InputResult::ModeTransition(ModeAction::EnterReorder),
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
            KeyCode::Enter => InputResult::Confirm,
            KeyCode::Char('q') | KeyCode::Esc => InputResult::Quit,
            _ => InputResult::Unhandled,
        }
    }

    /// Restricted keymap for non-Normal modes: Escape returns to Normal,
    /// Enter confirms, everything else is unhandled (caller handles text input).
    fn handle_non_normal(&self, key: KeyEvent, _mode: &Mode) -> InputResult {
        match key.code {
            KeyCode::Esc => InputResult::ModeTransition(ModeAction::ReturnToNormal),
            KeyCode::Enter => InputResult::Confirm,
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

impl FocusedPane {
    /// Cycle to the next pane in the display order: Stack → Diff → Worktrees → Stack.
    pub fn next(self) -> Self {
        match self {
            FocusedPane::Stack => FocusedPane::Diff,
            FocusedPane::Diff => FocusedPane::Worktrees,
            FocusedPane::Worktrees => FocusedPane::Stack,
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use proptest::proptest;

    use super::{FocusedPane, HunkAction, InputHandler, InputResult, ModeAction};
    use crate::app::{ConfirmAction, InputAction, Mode};

    fn normal() -> Mode {
        Mode::Normal
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())
    }

    // ── Constructor & defaults ──

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

    // ── Hunk navigation (unchanged behavior) ──

    #[test]
    fn navigate_next() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 0;
        handler.navigate_next();
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn navigate_prev() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 1;
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn navigate_prev_wraps() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 0;
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 2);
    }

    #[test]
    fn navigate_next_wraps() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(3);
        handler.current_hunk = 2;
        handler.navigate_next();
        assert_eq!(handler.current_hunk, 0);
    }

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

    #[test]
    fn zero_hunk_navigation_is_noop() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(0);
        handler.navigate_next();
        handler.navigate_prev();
        assert_eq!(handler.current_hunk, 0);
    }

    #[test]
    fn set_hunk_count_adjusts_current() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 4;
        handler.set_hunk_count(2);
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn set_hunk_count_zero_does_not_crash() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 2;
        handler.set_hunk_count(0);
        assert_eq!(handler.current_hunk, 2);
    }

    // ── Trait implementations ──

    #[test]
    fn input_handler_partial_eq() {
        let a = InputHandler::new();
        let b = InputHandler::new();
        assert_eq!(a, b);
    }

    #[test]
    fn hunk_action_partial_eq() {
        assert_eq!(HunkAction::Stage, HunkAction::Stage);
        assert_ne!(HunkAction::Stage, HunkAction::Unstage);
    }

    #[test]
    fn input_result_partial_eq() {
        assert_eq!(InputResult::Quit, InputResult::Quit);
        assert_eq!(InputResult::SwitchPane, InputResult::SwitchPane);
        assert_eq!(InputResult::Confirm, InputResult::Confirm);
        assert_eq!(
            InputResult::Handled(HunkAction::Stage),
            InputResult::Handled(HunkAction::Stage)
        );
        assert_ne!(InputResult::Quit, InputResult::Unhandled);
    }

    #[test]
    fn mode_action_partial_eq() {
        assert_eq!(ModeAction::EnterSearch, ModeAction::EnterSearch);
        assert_eq!(ModeAction::EnterHelp, ModeAction::EnterHelp);
        assert_eq!(ModeAction::ReturnToNormal, ModeAction::ReturnToNormal);
        assert_ne!(ModeAction::EnterSearch, ModeAction::EnterHelp);
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

    // ── Normal mode: hunk keybindings (preserved) ──

    #[test]
    fn normal_j_navigates_next() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        let result = handler.handle_key_event(key(KeyCode::Char('j')), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::NavigateNext));
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn normal_down_navigates_next() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 2;
        let result = handler.handle_key_event(key(KeyCode::Down), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::NavigateNext));
        assert_eq!(handler.current_hunk, 3);
    }

    #[test]
    fn normal_k_navigates_prev() {
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        handler.current_hunk = 2;
        let result = handler.handle_key_event(key(KeyCode::Char('k')), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::NavigatePrev));
        assert_eq!(handler.current_hunk, 1);
    }

    #[test]
    fn normal_space_stages() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Char(' ')), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::Stage));
    }

    #[test]
    fn normal_s_stages() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('s'), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::Stage));
    }

    #[test]
    fn normal_uppercase_s_is_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('S'), &normal());
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn normal_d_discards() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('d'), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::Discard));
    }

    #[test]
    fn normal_uppercase_d_discards() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('D'), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::Discard));
    }

    #[test]
    fn normal_u_unstages() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('u'), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::Unstage));
    }

    #[test]
    fn normal_pageup_scrolls_up() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::PageUp), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::ScrollUp));
    }

    #[test]
    fn normal_pagedown_scrolls_down() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::PageDown), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::ScrollDown));
    }

    #[test]
    fn normal_b_scrolls_up() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('b'), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::ScrollUp));
    }

    #[test]
    fn normal_f_scrolls_down() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('f'), &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::ScrollDown));
    }

    #[test]
    fn normal_q_quits() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('q'), &normal());
        assert_eq!(result, InputResult::Quit);
    }

    #[test]
    fn normal_esc_quits() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Esc), &normal());
        assert_eq!(result, InputResult::Quit);
    }

    // ── Normal mode: new keybindings ──

    #[test]
    fn normal_tab_switches_pane() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Tab), &normal());
        assert_eq!(result, InputResult::SwitchPane);
    }

    #[test]
    fn normal_slash_enters_search() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('/'), &normal());
        assert_eq!(result, InputResult::ModeTransition(ModeAction::EnterSearch));
    }

    #[test]
    fn normal_question_mark_enters_help() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('?'), &normal());
        assert_eq!(result, InputResult::ModeTransition(ModeAction::EnterHelp));
    }

    #[test]
    fn normal_enter_confirms() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Enter), &normal());
        assert_eq!(result, InputResult::Confirm);
    }

    #[test]
    fn normal_i_enters_input_rename() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('i'), &normal());
        assert_eq!(result, InputResult::ModeTransition(ModeAction::EnterInput(InputAction::Rename)));
    }

    #[test]
    fn normal_uppercase_r_enters_reorder() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('R'), &normal());
        assert_eq!(result, InputResult::ModeTransition(ModeAction::EnterReorder));
    }

    // ── Normal mode: unhandled keys ──

    #[test]
    fn normal_backspace_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Backspace), &normal());
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn normal_delete_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Delete), &normal());
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn normal_f1_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::F(1)), &normal());
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn normal_null_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Null), &normal());
        assert_eq!(result, InputResult::Unhandled);
    }

    // ── Normal mode: modifier keys ignored ──

    #[test]
    fn normal_ctrl_j_still_navigates() {
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL);
        let mut handler = InputHandler::new();
        handler.set_hunk_count(5);
        let result = handler.handle_key_event(key, &normal());
        assert_eq!(result, InputResult::Handled(HunkAction::NavigateNext));
    }

    // ── Non-Normal mode: Escape returns to Normal ──

    #[test]
    fn search_mode_esc_returns_to_normal() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Esc), &Mode::Search);
        assert_eq!(result, InputResult::ModeTransition(ModeAction::ReturnToNormal));
    }

    #[test]
    fn help_mode_esc_returns_to_normal() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Esc), &Mode::Help);
        assert_eq!(result, InputResult::ModeTransition(ModeAction::ReturnToNormal));
    }

    #[test]
    fn confirm_mode_esc_returns_to_normal() {
        let mut handler = InputHandler::new();
        let mode = Mode::Confirm(ConfirmAction::Delete("x".into()));
        let result = handler.handle_key_event(key(KeyCode::Esc), &mode);
        assert_eq!(result, InputResult::ModeTransition(ModeAction::ReturnToNormal));
    }

    #[test]
    fn input_mode_esc_returns_to_normal() {
        let mut handler = InputHandler::new();
        let mode = Mode::Input(InputAction::Rename);
        let result = handler.handle_key_event(key(KeyCode::Esc), &mode);
        assert_eq!(result, InputResult::ModeTransition(ModeAction::ReturnToNormal));
    }

    #[test]
    fn reorder_mode_esc_returns_to_normal() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Esc), &Mode::Reorder);
        assert_eq!(result, InputResult::ModeTransition(ModeAction::ReturnToNormal));
    }

    // ── Non-Normal mode: Enter confirms ──

    #[test]
    fn search_mode_enter_confirms() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Enter), &Mode::Search);
        assert_eq!(result, InputResult::Confirm);
    }

    #[test]
    fn confirm_mode_enter_confirms() {
        let mut handler = InputHandler::new();
        let mode = Mode::Confirm(ConfirmAction::RestackAll);
        let result = handler.handle_key_event(key(KeyCode::Enter), &mode);
        assert_eq!(result, InputResult::Confirm);
    }

    #[test]
    fn input_mode_enter_confirms() {
        let mut handler = InputHandler::new();
        let mode = Mode::Input(InputAction::NewBranch);
        let result = handler.handle_key_event(key(KeyCode::Enter), &mode);
        assert_eq!(result, InputResult::Confirm);
    }

    #[test]
    fn reorder_mode_enter_confirms() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Enter), &Mode::Reorder);
        assert_eq!(result, InputResult::Confirm);
    }

    // ── Non-Normal mode: normal-mode keys are unhandled ──

    #[test]
    fn search_mode_j_is_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('j'), &Mode::Search);
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn help_mode_tab_is_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(key(KeyCode::Tab), &Mode::Help);
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn confirm_mode_slash_is_unhandled() {
        let mut handler = InputHandler::new();
        let mode = Mode::Confirm(ConfirmAction::Delete("x".into()));
        let result = handler.handle_key_event(char_key('/'), &mode);
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn input_mode_question_mark_is_unhandled() {
        let mut handler = InputHandler::new();
        let mode = Mode::Input(InputAction::Rename);
        let result = handler.handle_key_event(char_key('?'), &mode);
        assert_eq!(result, InputResult::Unhandled);
    }

    #[test]
    fn reorder_mode_d_is_unhandled() {
        let mut handler = InputHandler::new();
        let result = handler.handle_key_event(char_key('d'), &Mode::Reorder);
        assert_eq!(result, InputResult::Unhandled);
    }

    // ── FocusedPane::next cycling ──

    #[test]
    fn focused_pane_next_stack_to_diff() {
        assert_eq!(FocusedPane::Stack.next(), FocusedPane::Diff);
    }

    #[test]
    fn focused_pane_next_diff_to_worktrees() {
        assert_eq!(FocusedPane::Diff.next(), FocusedPane::Worktrees);
    }

    #[test]
    fn focused_pane_next_worktrees_to_stack() {
        assert_eq!(FocusedPane::Worktrees.next(), FocusedPane::Stack);
    }

    #[test]
    fn focused_pane_next_full_cycle() {
        let start = FocusedPane::Stack;
        let second = start.next();
        assert_eq!(second, FocusedPane::Diff);
        let third = second.next();
        assert_eq!(third, FocusedPane::Worktrees);
        let back = third.next();
        assert_eq!(back, FocusedPane::Stack);
    }

    #[test]
    fn focused_pane_next_triple_cycle_returns_to_start() {
        let pane = FocusedPane::Diff;
        let after_three = pane.next().next().next();
        assert_eq!(after_three, FocusedPane::Diff);
    }

    // ── ModeAction discriminants ──

    #[test]
    fn mode_action_all_variants_distinct() {
        use std::mem::discriminant;
        // Only test discriminants between different variant constructors,
        // not between different payloads of the same variant.
        let enter_search = ModeAction::EnterSearch;
        let enter_help = ModeAction::EnterHelp;
        let enter_confirm = ModeAction::EnterConfirm(ConfirmAction::Delete("x".into()));
        let enter_input = ModeAction::EnterInput(InputAction::Rename);
        let enter_reorder = ModeAction::EnterReorder;
        let return_normal = ModeAction::ReturnToNormal;
        let top_level = [
            &enter_search,
            &enter_help,
            &enter_confirm,
            &enter_input,
            &enter_reorder,
            &return_normal,
        ];
        for i in 0..top_level.len() {
            for j in (i + 1)..top_level.len() {
                assert_ne!(
                    discriminant(top_level[i]),
                    discriminant(top_level[j]),
                    "actions at {i} and {j} should differ"
                );
            }
        }
    }

    #[test]
    fn mode_action_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ModeAction>();
    }

    #[test]
    fn mode_action_debug_format() {
        for action in [
            ModeAction::EnterSearch,
            ModeAction::EnterHelp,
            ModeAction::EnterReorder,
            ModeAction::ReturnToNormal,
        ] {
            let s = format!("{action:?}");
            assert!(!s.is_empty());
        }
    }

    // ── ModeAction with ConfirmAction::Delete ──

    #[test]
    fn mode_action_enter_confirm_delete_captures_name() {
        let action = ModeAction::EnterConfirm(ConfirmAction::Delete("branch".into()));
        if let ModeAction::EnterConfirm(ConfirmAction::Delete(name)) = action {
            assert_eq!(name, "branch");
        } else {
            panic!("expected EnterConfirm::Delete");
        }
    }

    #[test]
    fn mode_action_enter_confirm_restack_captures_name() {
        let action = ModeAction::EnterConfirm(ConfirmAction::Restack("feat".into()));
        if let ModeAction::EnterConfirm(ConfirmAction::Restack(name)) = action {
            assert_eq!(name, "feat");
        } else {
            panic!("expected EnterConfirm::Restack");
        }
    }

    // ── InputResult new variants Send + Sync ──

    #[test]
    fn input_result_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InputResult>();
    }

    #[test]
    fn input_result_debug_format_all_variants() {
        let results = [
            InputResult::Handled(HunkAction::Stage),
            InputResult::SwitchPane,
            InputResult::ModeTransition(ModeAction::EnterSearch),
            InputResult::Confirm,
            InputResult::Unhandled,
            InputResult::Quit,
        ];
        for r in &results {
            let s = format!("{r:?}");
            assert!(!s.is_empty());
        }
    }

    // ── Proptests ──

    proptest! {
        #[test]
        fn prop_handle_key_event_never_panics_normal(
            code_byte in 0u8..=127u8,
        ) {
            let code = KeyCode::Char(code_byte as char);
            let k = KeyEvent::new(code, KeyModifiers::empty());
            let mut handler = InputHandler::new();
            handler.set_hunk_count(5);
            let _result = handler.handle_key_event(k, &normal());
        }

        #[test]
        fn prop_handle_key_event_never_panics_search(
            code_byte in 0u8..=127u8,
        ) {
            let code = KeyCode::Char(code_byte as char);
            let k = KeyEvent::new(code, KeyModifiers::empty());
            let mut handler = InputHandler::new();
            let _result = handler.handle_key_event(k, &Mode::Search);
        }

        #[test]
        fn prop_set_hunk_count_arbitrary_usize(
            count in 0usize..1000usize,
        ) {
            let mut handler = InputHandler::new();
            handler.set_hunk_count(count);
            handler.navigate_next();
            handler.navigate_prev();
        }

        #[test]
        fn prop_handle_key_event_never_panics_all_modes(
            code_byte in 0u8..=127u8,
        ) {
            let code = KeyCode::Char(code_byte as char);
            let k = KeyEvent::new(code, KeyModifiers::empty());
            let mut handler = InputHandler::new();
            handler.set_hunk_count(3);
            let modes = [
                Mode::Normal,
                Mode::Search,
                Mode::Help,
                Mode::Reorder,
            ];
            for mode in &modes {
                let _result = handler.handle_key_event(k, mode);
            }
        }
    }
}
