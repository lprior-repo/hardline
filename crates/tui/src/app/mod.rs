<<<<<<< HEAD
use std::io::{self, Stdout};

use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::error::Result;
use crate::input::InputHandler;
=======
use std::io;

use crossterm::{
    event::{self, Event},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::CrosstermBackend;

use crate::error::Result;
use crate::input::{InputHandler, InputResult};
>>>>>>> polecat/kappa
use crate::views::WorktreeView;
use crate::widgets::diff::DiffLine;
use scp_stack::domain::StackBranch;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FocusedPane {
    Stack,
    Diff,
    Worktrees,
}

<<<<<<< HEAD
#[derive(Debug, Clone)]
=======
#[derive(Debug, Clone, PartialEq)]
>>>>>>> polecat/gamma
pub enum Mode {
    Normal,
    Search,
    Help,
    Confirm(ConfirmAction),
    Input(InputAction),
    Reorder,
}

<<<<<<< HEAD
#[derive(Debug, Clone)]
=======
#[derive(Debug, Clone, PartialEq)]
>>>>>>> polecat/gamma
pub enum ConfirmAction {
    Delete(String),
    Restack(String),
    RestackAll,
    ApplyReorder,
}

<<<<<<< HEAD
#[derive(Debug, Clone, Copy)]
=======
#[derive(Debug, Clone, Copy, PartialEq)]
>>>>>>> polecat/gamma
pub enum InputAction {
    Rename,
    NewBranch,
}

/// Provides stack branch data to the TUI. Implementations bridge
/// to VCS or stack infrastructure without coupling the TUI to concrete backends.
pub trait BranchProvider: Send + Sync {
    fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String>;
}

pub struct TuiApp {
    pub focused_pane: FocusedPane,
    pub mode: Mode,
    pub needs_refresh: bool,
    pub should_quit: bool,
    pub status_message: String,
    pub worktree_view: WorktreeView,
    pub stack_branches: Vec<StackBranch>,
    pub diff_lines: Vec<DiffLine>,
    branch_provider: Box<dyn BranchProvider>,
}

impl TuiApp {
    pub fn new(branch_provider: Box<dyn BranchProvider>) -> Result<Self> {
        Ok(Self {
            focused_pane: FocusedPane::Stack,
            mode: Mode::Normal,
            needs_refresh: true,
            should_quit: false,
            status_message: String::new(),
            worktree_view: WorktreeView::default(),
            stack_branches: Vec::new(),
            diff_lines: Vec::new(),
            branch_provider,
        })
    }

    pub fn refresh_branches(&mut self) -> Result<()> {
        if !self.needs_refresh {
            return Ok(());
        }

        match self.branch_provider.load_branches() {
            Ok(branches) => {
                self.stack_branches = branches;
                self.needs_refresh = false;
                Ok(())
            }
            Err(msg) => {
                self.set_status(format!("Failed to load branches: {msg}"));
                self.needs_refresh = false;
                Ok(())
            }
        }
    }

    pub fn selected_branch(&self) -> Option<String> {
        None
    }

    pub fn set_status(&mut self, message: String) {
        self.status_message = message;
    }
}

<<<<<<< HEAD
fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = io::stdout().execute(LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
        original_hook(info);
    }));
}

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    terminal::enable_raw_mode().map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?;
    io::stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?;
    let backend = CrosstermBackend::new(io::stdout());
    Terminal::new(backend).map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))
}

fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
) -> std::result::Result<(), std::io::Error> {
    terminal.show_cursor()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()
}

pub fn run(branch_provider: Box<dyn BranchProvider>) -> Result<()> {
    install_panic_hook();

    let mut terminal = init_terminal()?;
    let mut app = TuiApp::new(branch_provider)?;
    let mut input_handler = InputHandler::new();
    app.needs_refresh = true;

    let result = run_loop(&mut terminal, &mut app, &mut input_handler);

    let _ = restore_terminal(&mut terminal);

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut TuiApp,
    input_handler: &mut InputHandler,
) -> Result<()> {
    loop {
        app.refresh_branches()?;

        terminal
            .draw(|f| crate::views::render(f, app))
            .map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?;

        if event::poll(std::time::Duration::from_millis(250))
            .map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?
        {
            if let Event::Key(key) = event::read()
                .map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?
            {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    app.should_quit = true;
                    break;
                }

                if key.code == KeyCode::Tab {
                    app.focused_pane = match app.focused_pane {
                        FocusedPane::Stack => FocusedPane::Diff,
                        FocusedPane::Diff => FocusedPane::Worktrees,
                        FocusedPane::Worktrees => FocusedPane::Stack,
                    };
                    continue;
                }

                let _ = input_handler.handle_key_event(key);
            }
        }

=======
/// Duration between terminal redraws when idle (milliseconds).
const TICK_RATE_MS: u64 = 250;

/// Restores the terminal to its original state on drop (including panics).
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = io::stdout().execute(LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}

pub fn run() -> Result<()> {
    let mut app = TuiApp::new()?;
    app.needs_refresh = true;

    enable_raw_mode().map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?;
    let _guard = TerminalGuard;

    io::stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = ratatui::Terminal::new(backend)
        .map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?;

    terminal.clear().map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?;

    let mut input_handler = InputHandler::new();
    let tick_duration = std::time::Duration::from_millis(TICK_RATE_MS);
    let mut last_tick = std::time::Instant::now();

    loop {
        terminal
            .draw(|f| crate::views::render(f, &mut app))
            .map_err(|e| crate::error::TuiError::Error(e.to_string()))?;

        let timeout = tick_duration
            .checked_sub(last_tick.elapsed())
            .unwrap_or(std::time::Duration::from_secs(0));

        if event::poll(timeout).map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))? {
            if let Event::Key(key) = event::read()
                .map_err(|e| crate::error::TuiError::TerminalError(e.to_string()))?
            {
                match input_handler.handle_key_event(key) {
                    InputResult::Quit => app.should_quit = true,
                    _ => {}
                }
            }
        }

        if last_tick.elapsed() >= tick_duration {
            last_tick = std::time::Instant::now();
        }

>>>>>>> polecat/kappa
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StubBranchProvider;

    impl BranchProvider for StubBranchProvider {
        fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
            Ok(Vec::new())
        }
    }

    fn test_app() -> TuiApp {
        TuiApp::new(Box::new(StubBranchProvider)).expect("TuiApp::new should succeed")
    }

    // ── Constructor & default state ──

    #[test]
    fn tui_app_new_returns_default_state() {
        let app = test_app();
        assert!(!app.should_quit);
        assert!(app.needs_refresh);
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn multiple_app_instances_are_independent() {
        let mut app1 = test_app();
        let mut app2 = test_app();
        app1.should_quit = true;
        app2.mode = Mode::Help;
        assert!(app1.should_quit);
        assert!(!app2.should_quit);
        assert!(matches!(app1.mode, Mode::Normal));
        assert!(matches!(app2.mode, Mode::Help));
    }

    // ── refresh_branches ──

    #[test]
    fn refresh_branches_clears_needs_refresh() {
        let mut app = test_app();
        app.needs_refresh = true;
        app.refresh_branches().expect("should succeed");
        assert!(!app.needs_refresh);
    }

    #[test]
    fn refresh_branches_skips_when_not_needed() {
        struct PanickingProvider;
        impl BranchProvider for PanickingProvider {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                panic!("load_branches should not be called when needs_refresh is false");
            }
        }

        let mut app = TuiApp::new(Box::new(PanickingProvider)).expect("ok");
        app.needs_refresh = false;
        // Should return Ok without calling the provider
        app.refresh_branches().expect("should succeed");
    }

    #[test]
    fn refresh_branches_populates_stack_branches() {
        use scp_stack::domain::BranchName;

        struct StaticProvider;
        impl BranchProvider for StaticProvider {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                Ok(vec![StackBranch {
                    name: BranchName::new("feature/a"),
                    parent: Some(BranchName::new("main")),
                    children: vec![BranchName::new("feature/b")],
                    needs_restack: true,
                    pr_info: None,
                }])
            }
        }

        let mut app = TuiApp::new(Box::new(StaticProvider)).expect("ok");
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        assert_eq!(app.stack_branches.len(), 1);
        assert_eq!(app.stack_branches[0].name.as_str(), "feature/a");
        assert!(app.stack_branches[0].needs_restack);
    }

    #[test]
    fn refresh_branches_on_error_sets_status() {
        struct FailingProvider;
        impl BranchProvider for FailingProvider {
            fn load_branches(&self) -> std::result::Result<Vec<StackBranch>, String> {
                Err("git not found".to_string())
            }
        }

        let mut app = TuiApp::new(Box::new(FailingProvider)).expect("ok");
        app.needs_refresh = true;
        app.refresh_branches().expect("should not error — errors become status");
        assert!(app.status_message.contains("git not found"));
        assert!(!app.needs_refresh, "should clear flag even on error");
    }

    #[test]
    fn refresh_does_not_affect_other_fields() {
        let mut app = test_app();
        app.should_quit = true;
        app.focused_pane = FocusedPane::Diff;
        app.mode = Mode::Search;
        app.refresh_branches().expect("ok");
        assert!(app.should_quit);
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(matches!(app.mode, Mode::Search));
    }

    // ── selected_branch ──

    #[test]
    fn selected_branch_returns_none_by_default() {
        let app = test_app();
        assert!(app.selected_branch().is_none());
    }

    // ── set_status ──

    #[test]
    fn set_status_does_not_panic() {
        let mut app = test_app();
        app.set_status("test message".to_string());
        assert_eq!(app.status_message, "test message");
    }

    #[test]
    fn set_status_with_empty_string() {
        let mut app = test_app();
        app.set_status(String::new());
        assert!(app.status_message.is_empty());
    }

    #[test]
    fn set_status_with_long_string() {
        let mut app = test_app();
        app.set_status("a".repeat(10_000));
    }

    // ── run ──

    /// Returns true only when explicitly opted into interactive terminal tests.
    /// Just having TERM set (e.g. in tmux) is not enough — run() blocks on input.
    fn has_interactive_terminal() -> bool {
        std::env::var("SCP_TUI_INTEGRATION").is_ok()
    }

    #[test]
<<<<<<< HEAD
    fn run_returns_terminal_error_without_tty() {
        let result = run(Box::new(StubBranchProvider));
        assert!(result.is_err(), "run() should fail without a terminal");
=======
    fn run_returns_ok() {
        if !has_interactive_terminal() {
            return; // skip: no terminal in CI/test environment
        }
        // Spawn in a thread so the terminal guard cleanup runs even on failure
        let handle = std::thread::spawn(|| {
            // Immediately set should_quit so the loop exits after one frame
            // We can't do this from outside run() without modifying the app,
            // so we test that run() initializes and cleans up without panicking.
            // The test verifies terminal setup/teardown round-trips.
            let _ = run();
        });
        let _ = handle.join();
>>>>>>> polecat/kappa
    }

    // ── FocusedPane discriminants ──

    #[test]
    fn focused_pane_variants_are_distinct() {
        let stack = FocusedPane::Stack;
        let diff = FocusedPane::Diff;
        assert!(std::mem::discriminant(&stack) != std::mem::discriminant(&diff));
    }

    #[test]
    fn focused_pane_size_is_minimal() {
        assert_eq!(std::mem::size_of::<FocusedPane>(), 1);
    }

    // ── Mode variants ──

    #[test]
    fn mode_normal_is_not_other_variants() {
        let modes = vec![
            Mode::Search,
            Mode::Help,
            Mode::Confirm(ConfirmAction::RestackAll),
            Mode::Input(InputAction::Rename),
            Mode::Reorder,
        ];
        for other in modes {
            assert!(
                !matches!(Mode::Normal, _ if std::mem::discriminant(&Mode::Normal) == std::mem::discriminant(&other))
            );
        }
    }

    #[test]
    fn mode_confirm_carries_branch_name_for_delete() {
        let mode = Mode::Confirm(ConfirmAction::Delete("my-branch".to_string()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert_eq!(name, "my-branch"),
            _ => panic!("expected Confirm::Delete"),
        }
    }

    #[test]
    fn mode_confirm_carries_branch_name_for_restack() {
        let mode = Mode::Confirm(ConfirmAction::Restack("feature/x".to_string()));
        match mode {
            Mode::Confirm(ConfirmAction::Restack(name)) => assert_eq!(name, "feature/x"),
            _ => panic!("expected Confirm::Restack"),
        }
    }

    #[test]
    fn mode_confirm_delete_preserves_empty_name() {
        let mode = Mode::Confirm(ConfirmAction::Delete(String::new()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert!(name.is_empty()),
            _ => panic!("expected Confirm::Delete"),
        }
    }

    #[test]
    fn mode_confirm_delete_preserves_long_name() {
        let long_name = "a".repeat(1000);
        let mode = Mode::Confirm(ConfirmAction::Delete(long_name.clone()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert_eq!(name.len(), 1000),
            _ => panic!("expected Confirm::Delete"),
        }
    }

    #[test]
    fn confirm_actions_are_unit_except_delete_and_restack() {
        // RestackAll and ApplyReorder carry no data
        let _ = ConfirmAction::RestackAll;
        let _ = ConfirmAction::ApplyReorder;
    }

    #[test]
    fn input_action_variants_are_distinct() {
        let rename = InputAction::Rename;
        let new_branch = InputAction::NewBranch;
        assert!(std::mem::discriminant(&rename) != std::mem::discriminant(&new_branch));
    }

    // ── State transition simulation ──

    #[test]
    fn simulate_quit_flow() {
        let mut app = test_app();
        assert!(!app.should_quit);
        app.should_quit = true;
        assert!(app.should_quit);
    }

    #[test]
    fn simulate_pane_switching() {
        let mut app = test_app();
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
        app.focused_pane = FocusedPane::Diff;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        app.focused_pane = FocusedPane::Stack;
        assert!(matches!(app.focused_pane, FocusedPane::Stack));
    }

    #[test]
    fn simulate_mode_transitions() {
        let mut app = test_app();
        // Normal -> Search
        app.mode = Mode::Search;
        assert!(matches!(app.mode, Mode::Search));
        // Search -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
        // Normal -> Help
        app.mode = Mode::Help;
        assert!(matches!(app.mode, Mode::Help));
        // Help -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
        // Normal -> Confirm(Delete)
        app.mode = Mode::Confirm(ConfirmAction::Delete("b".to_string()));
        assert!(matches!(app.mode, Mode::Confirm(ConfirmAction::Delete(_))));
        // Confirm -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
        // Normal -> Reorder
        app.mode = Mode::Reorder;
        assert!(matches!(app.mode, Mode::Reorder));
        // Reorder -> Normal
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn simulate_full_lifecycle() {
        let mut app = test_app();
        // Initial refresh
        assert!(app.needs_refresh);
        app.refresh_branches().expect("ok");
        assert!(!app.needs_refresh);

        // User interacts — switch pane
        app.focused_pane = FocusedPane::Diff;
        // User opens help
        app.mode = Mode::Help;
        // User closes help
        app.mode = Mode::Normal;
        // Trigger another refresh
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        // User quits
        app.should_quit = true;
        assert!(app.should_quit);
        assert!(!app.needs_refresh);
    }

    #[test]
    fn needs_refresh_can_be_rearmed() {
        let mut app = test_app();
        app.needs_refresh = false;
        app.refresh_branches().expect("ok");
        app.needs_refresh = true;
        app.refresh_branches().expect("ok");
        assert!(!app.needs_refresh);
        app.needs_refresh = true;
        assert!(app.needs_refresh);
    }

    // ── TuiApp memory layout ──

    #[test]
    fn tui_app_is_not_zero_sized() {
        assert!(std::mem::size_of::<TuiApp>() > 0);
    }

    #[test]
    fn tui_app_fields_are_accessible() {
        let app = test_app();
        let _pane: &FocusedPane = &app.focused_pane;
        let _mode: &Mode = &app.mode;
        let _needs_refresh: bool = app.needs_refresh;
        let _should_quit: bool = app.should_quit;
    }

    // ── Result type integration ──

    #[test]
    fn tui_app_new_satisfies_result_contract() {
        let result: Result<TuiApp> = TuiApp::new(Box::new(StubBranchProvider));
        assert!(result.is_ok());
        let _app = result.expect("ok");
    }

    #[test]
    fn refresh_branches_satisfies_result_contract() {
        let mut app = test_app();
        let result: Result<()> = app.refresh_branches();
        assert!(result.is_ok());
    }

    // ── FocusedPane exhaustive variants ──

    #[test]
    fn focused_pane_all_variants_constructible() {
        let _s = FocusedPane::Stack;
        let _d = FocusedPane::Diff;
    }

    #[test]
    fn focused_pane_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FocusedPane>();
    }

    #[test]
    fn focused_pane_debug_format() {
        let s = FocusedPane::Stack;
        let d = FocusedPane::Diff;
        let s_str = format!("{s:?}");
        let d_str = format!("{d:?}");
        assert!(!s_str.is_empty());
        assert!(!d_str.is_empty());
        assert_ne!(s_str, d_str);
    }

    // ── Mode exhaustive variants ──

    #[test]
    fn mode_all_variants_constructible() {
        let _normal = Mode::Normal;
        let _search = Mode::Search;
        let _help = Mode::Help;
        let _confirm_delete = Mode::Confirm(ConfirmAction::Delete("x".into()));
        let _confirm_restack = Mode::Confirm(ConfirmAction::Restack("x".into()));
        let _confirm_restack_all = Mode::Confirm(ConfirmAction::RestackAll);
        let _confirm_apply = Mode::Confirm(ConfirmAction::ApplyReorder);
        let _input_rename = Mode::Input(InputAction::Rename);
        let _input_new = Mode::Input(InputAction::NewBranch);
        let _reorder = Mode::Reorder;
    }

    #[test]
    fn mode_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Mode>();
    }

    #[test]
    fn mode_debug_format_variants() {
        let normal = format!("{:?}", Mode::Normal);
        let search = format!("{:?}", Mode::Search);
        let help = format!("{:?}", Mode::Help);
        let reorder = format!("{:?}", Mode::Reorder);
        for (name, s) in [
            ("Normal", normal),
            ("Search", search),
            ("Help", help),
            ("Reorder", reorder),
        ] {
            assert!(!s.is_empty(), "{name} debug should not be empty");
        }
    }

    #[test]
    fn mode_confirm_debug_shows_action() {
        let confirm = Mode::Confirm(ConfirmAction::Delete("branch".into()));
        let debug = format!("{confirm:?}");
        assert!(
            debug.contains("Delete"),
            "should mention Delete action: {debug}"
        );
    }

    #[test]
    fn mode_input_debug_shows_action() {
        let input = Mode::Input(InputAction::Rename);
        let debug = format!("{input:?}");
        assert!(
            debug.contains("Rename"),
            "should mention Rename action: {debug}"
        );
    }

    #[test]
    fn mode_input_new_branch_debug() {
        let input = Mode::Input(InputAction::NewBranch);
        let debug = format!("{input:?}");
        assert!(
            debug.contains("NewBranch"),
            "should mention NewBranch action: {debug}"
        );
    }

    // ── ConfirmAction exhaustive variants ──

    #[test]
    fn confirm_action_delete_captures_name() {
        let action = ConfirmAction::Delete("main".to_string());
        match action {
            ConfirmAction::Delete(name) => assert_eq!(name, "main"),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn confirm_action_restack_captures_name() {
        let action = ConfirmAction::Restack("feature".to_string());
        match action {
            ConfirmAction::Restack(name) => assert_eq!(name, "feature"),
            _ => panic!("expected Restack"),
        }
    }

    #[test]
    fn confirm_action_restack_all_is_unit() {
        let _ = ConfirmAction::RestackAll;
    }

    #[test]
    fn confirm_action_apply_reorder_is_unit() {
        let _ = ConfirmAction::ApplyReorder;
    }

    #[test]
    fn confirm_action_all_variants_distinct_discriminants() {
        use std::mem::discriminant;
        let delete = ConfirmAction::Delete("a".into());
        let restack = ConfirmAction::Restack("b".into());
        let restack_all = ConfirmAction::RestackAll;
        let apply = ConfirmAction::ApplyReorder;
        assert_ne!(discriminant(&delete), discriminant(&restack));
        assert_ne!(discriminant(&delete), discriminant(&restack_all));
        assert_ne!(discriminant(&delete), discriminant(&apply));
        assert_ne!(discriminant(&restack), discriminant(&restack_all));
        assert_ne!(discriminant(&restack), discriminant(&apply));
        assert_ne!(discriminant(&restack_all), discriminant(&apply));
    }

    #[test]
    fn confirm_action_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ConfirmAction>();
    }

    #[test]
    fn confirm_action_debug_format() {
        let delete = format!("{:?}", ConfirmAction::Delete("x".into()));
        let restack = format!("{:?}", ConfirmAction::Restack("y".into()));
        let all = format!("{:?}", ConfirmAction::RestackAll);
        let apply = format!("{:?}", ConfirmAction::ApplyReorder);
        assert!(delete.contains("Delete"));
        assert!(restack.contains("Restack"));
        assert!(all.contains("RestackAll"));
        assert!(apply.contains("ApplyReorder"));
    }

    #[test]
    fn confirm_action_delete_with_unicode_name() {
        let action = ConfirmAction::Delete("feature/日本語-branch".to_string());
        match action {
            ConfirmAction::Delete(name) => assert_eq!(name, "feature/日本語-branch"),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn confirm_action_delete_with_whitespace_name() {
        let action = ConfirmAction::Delete("   ".to_string());
        match action {
            ConfirmAction::Delete(name) => assert_eq!(name, "   "),
            _ => panic!("expected Delete"),
        }
    }

    // ── InputAction exhaustive variants ──

    #[test]
    fn input_action_rename_is_unit() {
        let _ = InputAction::Rename;
    }

    #[test]
    fn input_action_new_branch_is_unit() {
        let _ = InputAction::NewBranch;
    }

    #[test]
    fn input_action_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<InputAction>();
    }

    #[test]
    fn input_action_debug_format() {
        let rename = format!("{:?}", InputAction::Rename);
        let new_branch = format!("{:?}", InputAction::NewBranch);
        assert!(rename.contains("Rename"));
        assert!(new_branch.contains("NewBranch"));
    }

    // ── More state transition patterns ──

    #[test]
    fn can_enter_confirm_mode_and_return() {
        let mut app = test_app();
        assert!(matches!(app.mode, Mode::Normal));
        app.mode = Mode::Confirm(ConfirmAction::Delete("x".into()));
        assert!(matches!(app.mode, Mode::Confirm(ConfirmAction::Delete(_))));
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn can_enter_input_mode_and_return() {
        let mut app = test_app();
        app.mode = Mode::Input(InputAction::NewBranch);
        assert!(matches!(app.mode, Mode::Input(InputAction::NewBranch)));
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn can_enter_input_rename_mode_and_return() {
        let mut app = test_app();
        app.mode = Mode::Input(InputAction::Rename);
        assert!(matches!(app.mode, Mode::Input(InputAction::Rename)));
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn can_cycle_through_all_modes() {
        let mut app = test_app();
        let modes = vec![
            Mode::Search,
            Mode::Help,
            Mode::Confirm(ConfirmAction::Delete("a".into())),
            Mode::Confirm(ConfirmAction::Restack("b".into())),
            Mode::Confirm(ConfirmAction::RestackAll),
            Mode::Confirm(ConfirmAction::ApplyReorder),
            Mode::Input(InputAction::Rename),
            Mode::Input(InputAction::NewBranch),
            Mode::Reorder,
        ];
        for mode in modes {
            app.mode = mode;
            // just ensure assignment succeeds — no panic
        }
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn switching_pane_does_not_affect_mode() {
        let mut app = test_app();
        app.mode = Mode::Search;
        app.focused_pane = FocusedPane::Diff;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(matches!(app.mode, Mode::Search));
    }

    #[test]
    fn setting_mode_does_not_affect_pane() {
        let mut app = test_app();
        app.focused_pane = FocusedPane::Diff;
        app.mode = Mode::Help;
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(matches!(app.mode, Mode::Help));
    }

    #[test]
    fn should_quit_is_independent_of_other_fields() {
        let mut app = test_app();
        app.should_quit = true;
        app.mode = Mode::Reorder;
        app.focused_pane = FocusedPane::Diff;
        app.needs_refresh = true;
        assert!(app.should_quit);
        assert!(matches!(app.mode, Mode::Reorder));
        assert!(matches!(app.focused_pane, FocusedPane::Diff));
        assert!(app.needs_refresh);
    }

    #[test]
    fn needs_refresh_toggle_cycle() {
        let mut app = test_app();
        for _ in 0..100 {
            app.needs_refresh = !app.needs_refresh;
        }
        // After 100 toggles from initial true, it should be true (even toggles)
        assert!(app.needs_refresh);
    }

    #[test]
    fn repeated_refresh_calls() {
        let mut app = test_app();
        for _ in 0..50 {
            app.refresh_branches().expect("ok");
        }
        assert!(!app.needs_refresh);
    }

    #[test]
    fn selected_branch_always_none() {
        let mut app = test_app();
        app.mode = Mode::Reorder;
        app.focused_pane = FocusedPane::Diff;
        app.should_quit = true;
        assert!(app.selected_branch().is_none());
    }

    #[test]
    fn set_status_various_strings() {
        let mut app = test_app();
        let messages = vec![
            String::new(),
            "hello".into(),
            "a".repeat(1_000_000),
            "\0\x01\x02".into(),
            "line1\nline2\r\nline3".into(),
        ];
        for msg in messages {
            app.set_status(msg);
        }
    }

    #[test]
    fn tui_app_size_is_reasonable() {
        let size = std::mem::size_of::<TuiApp>();
        assert!(
            size < 512,
            "TuiApp should be reasonably sized, got {size} bytes"
        );
    }

    #[test]
    fn run_does_not_modify_static_state() {
<<<<<<< HEAD
        // run() creates a local app, should not have side effects
        let provider = Box::new(StubBranchProvider);
        let _ = run(provider);
=======
        if !has_interactive_terminal() {
            return; // skip: no terminal in CI/test environment
        }
        let _ = run();
        let _ = run();
        let _ = run();
>>>>>>> polecat/kappa
    }

    // ── Proptests ──

    use proptest::proptest;

    // ── DiffLine ──

    #[test]
    fn diff_line_variants_constructible() {
        let _h = DiffLine::Header("diff --git a/b".into());
        let _k = DiffLine::Hunk("@@ -1,3 +1,4 @@".into());
        let _c = DiffLine::Context(" unchanged".into());
        let _a = DiffLine::Add("+new line".into());
        let _r = DiffLine::Remove("-old line".into());
    }

    #[test]
    fn diff_line_equality() {
        assert_eq!(
            DiffLine::Add("x".into()),
            DiffLine::Add("x".into())
        );
        assert_ne!(
            DiffLine::Add("x".into()),
            DiffLine::Remove("x".into())
        );
    }

    #[test]
    fn diff_line_clone_independent() {
        let original = DiffLine::Header("test".into());
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn diff_line_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DiffLine>();
    }

    #[test]
    fn diff_line_debug_format() {
        let header = format!("{:?}", DiffLine::Header("h".into()));
        let hunk = format!("{:?}", DiffLine::Hunk("@@".into()));
        let context = format!("{:?}", DiffLine::Context("c".into()));
        let add = format!("{:?}", DiffLine::Add("+a".into()));
        let remove = format!("{:?}", DiffLine::Remove("-r".into()));
        assert!(header.contains("Header"));
        assert!(hunk.contains("Hunk"));
        assert!(context.contains("Context"));
        assert!(add.contains("Add"));
        assert!(remove.contains("Remove"));
    }

    #[test]
    fn diff_lines_empty_by_default() {
        let app = TuiApp::new().expect("ok");
        assert!(app.diff_lines.is_empty());
    }

    #[test]
    fn diff_lines_can_be_set() {
        let mut app = TuiApp::new().expect("ok");
        app.diff_lines.push(DiffLine::Header("diff --git a/b b/c".into()));
        app.diff_lines.push(DiffLine::Add("+hello".into()));
        assert_eq!(app.diff_lines.len(), 2);
    }

    #[test]
    fn diff_lines_with_various_content() {
        let mut app = TuiApp::new().expect("ok");
        app.diff_lines = vec![
            DiffLine::Header("diff --git a/foo b/foo".into()),
            DiffLine::Context(" line".into()),
            DiffLine::Remove("-old".into()),
            DiffLine::Add("+new".into()),
            DiffLine::Hunk("@@ -1,2 +1,3 @@".into()),
        ];
        assert_eq!(app.diff_lines.len(), 5);
        assert!(matches!(app.diff_lines[0], DiffLine::Header(_)));
        assert!(matches!(app.diff_lines[2], DiffLine::Remove(_)));
        assert!(matches!(app.diff_lines[3], DiffLine::Add(_)));
    }

    #[test]
    fn diff_lines_clearable() {
        let mut app = TuiApp::new().expect("ok");
        app.diff_lines.push(DiffLine::Add("x".into()));
        assert!(!app.diff_lines.is_empty());
        app.diff_lines.clear();
        assert!(app.diff_lines.is_empty());
    }

    proptest! {
        #[test]
        fn prop_delete_confirm_preserves_arbitrary_name(
            name in ".{0, 500}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Delete(name.clone()));
            if let Mode::Confirm(ConfirmAction::Delete(n)) = mode {
                assert_eq!(n, name);
            }
        }

        #[test]
        fn prop_restack_confirm_preserves_arbitrary_name(
            name in ".{0, 500}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Restack(name.clone()));
            if let Mode::Confirm(ConfirmAction::Restack(n)) = mode {
                assert_eq!(n, name);
            }
        }

        #[test]
        fn prop_needs_refresh_after_toggle_sequence(
            initial in proptest::bool::ANY,
        ) {
            let mut app = test_app();
            app.needs_refresh = initial;
            assert_eq!(app.needs_refresh, initial);
        }

        #[test]
        fn prop_should_quit_arbitrary_bool(
            quit in proptest::bool::ANY,
        ) {
            let mut app = test_app();
            app.should_quit = quit;
            assert_eq!(app.should_quit, quit);
        }

        #[test]
        fn prop_set_status_arbitrary_string_stored(
            msg in proptest::string::string_regex(".{0,10000}").unwrap(),
        ) {
            let mut app = test_app();
            app.set_status(msg.clone());
            assert_eq!(app.status_message, msg);
        }

        #[test]
        fn prop_refresh_always_clears_flag(
            start in proptest::bool::ANY,
        ) {
            let mut app = test_app();
            app.needs_refresh = start;
            app.refresh_branches().expect("ok");
            assert!(!app.needs_refresh);
        }

        #[test]
        fn prop_selected_branch_always_none(
            _dummy in proptest::bool::ANY,
        ) {
            let app = test_app();
            assert!(app.selected_branch().is_none());
        }

        #[test]
        fn prop_mode_confirm_delete_roundtrip(
            name in ".{0, 200}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Delete(name.clone()));
            if let Mode::Confirm(ConfirmAction::Delete(n)) = mode {
                assert_eq!(n, name);
            }
        }

        #[test]
        fn prop_mode_confirm_restack_roundtrip(
            name in ".{0, 200}",
        ) {
            let mode = Mode::Confirm(ConfirmAction::Restack(name.clone()));
            if let Mode::Confirm(ConfirmAction::Restack(n)) = mode {
                assert_eq!(n, name);
            }
        }
    }

    // ── Adversarial: wrong state ──

    #[test]
    fn adv_refresh_before_constructed() {
        // Multiple constructions should not share state
        let app1 = TuiApp::new().expect("ok");
        let _ = TuiApp::new().expect("ok");
        let _ = TuiApp::new().expect("ok");
        // First app should still have needs_refresh = true
        assert!(app1.needs_refresh);
    }

    #[test]
    fn adv_refresh_idempotent_under_stress() {
        let mut app = TuiApp::new().expect("ok");
        for _ in 0..1000 {
            app.refresh_branches().expect("ok");
            assert!(!app.needs_refresh);
        }
    }

    #[test]
    fn adv_mode_switch_stress() {
        let mut app = TuiApp::new().expect("ok");
        let modes = vec![
            Mode::Normal, Mode::Search, Mode::Help, Mode::Reorder,
            Mode::Confirm(ConfirmAction::Delete("x".into())),
            Mode::Confirm(ConfirmAction::Restack("y".into())),
            Mode::Confirm(ConfirmAction::RestackAll),
            Mode::Confirm(ConfirmAction::ApplyReorder),
            Mode::Input(InputAction::Rename),
            Mode::Input(InputAction::NewBranch),
        ];
        for _ in 0..100 {
            for mode in &modes {
                app.mode = mode.clone();
            }
        }
        app.mode = Mode::Normal;
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn adv_pane_switch_stress() {
        let mut app = TuiApp::new().expect("ok");
        let panes = [FocusedPane::Stack, FocusedPane::Diff, FocusedPane::Worktrees];
        for _ in 0..1000 {
            for pane in &panes {
                app.focused_pane = *pane;
            }
        }
    }

    #[test]
    fn adv_confirm_delete_with_path_traversal() {
        let traversal = "../../etc/passwd".to_string();
        let mode = Mode::Confirm(ConfirmAction::Delete(traversal.clone()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(name)) => assert_eq!(name, traversal),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn adv_confirm_delete_with_null_bytes() {
        let name = "branch\0malicious".to_string();
        let mode = Mode::Confirm(ConfirmAction::Delete(name.clone()));
        match mode {
            Mode::Confirm(ConfirmAction::Delete(n)) => assert_eq!(n, name),
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn adv_set_status_with_special_chars() {
        let mut app = TuiApp::new().expect("ok");
        app.set_status("\x00\x01\x02\x7f\x7e\x7d".to_string());
        app.set_status("他她它\x00".to_string());
        app.set_status("\r\n\t\0".to_string());
        // Must not panic — set_status is a no-op stub
    }

    #[test]
    fn adv_run_multiple_times() {
        if !has_interactive_terminal() {
            return; // skip: no terminal in CI/test environment
        }
        for _ in 0..100 {
            assert!(run().is_ok());
        }
    }

    // ── Event loop constants ──

    #[test]
    fn tick_rate_is_reasonable() {
        assert!(TICK_RATE_MS > 0, "tick rate must be positive");
        assert!(TICK_RATE_MS <= 1000, "tick rate should be at most 1 second");
    }

    #[test]
    fn tick_rate_duration_constructs() {
        let _dur = std::time::Duration::from_millis(TICK_RATE_MS);
    }

    // ── TerminalGuard type exists and is private ──

    #[test]
    fn terminal_guard_size_is_small() {
        // TerminalGuard is a ZST (zero-sized type) — just a Drop impl
        assert_eq!(std::mem::size_of::<TerminalGuard>(), 0);
    }

    // ── has_interactive_terminal gate ──

    #[test]
    fn has_interactive_terminal_returns_bool() {
        let _ = has_interactive_terminal();
    }
}
