use crate::error::Result;

pub enum FocusedPane {
    Stack,
    Diff,
}

pub enum Mode {
    Normal,
    Search,
    Help,
    Confirm(ConfirmAction),
    Input(InputAction),
    Reorder,
}

pub enum ConfirmAction {
    Delete(String),
    Restack(String),
    RestackAll,
    ApplyReorder,
}

pub enum InputAction {
    Rename,
    NewBranch,
}

pub struct TuiApp {
    pub focused_pane: FocusedPane,
    pub mode: Mode,
    pub needs_refresh: bool,
    pub should_quit: bool,
}

impl TuiApp {
    pub fn new() -> Result<Self> {
        Ok(Self {
            focused_pane: FocusedPane::Stack,
            mode: Mode::Normal,
            needs_refresh: true,
            should_quit: false,
        })
    }

    pub fn refresh_branches(&mut self) -> Result<()> {
        self.needs_refresh = false;
        Ok(())
    }

    pub fn selected_branch(&self) -> Option<String> {
        None
    }

    pub fn set_status(&mut self, _message: String) {}
}

pub fn run() -> Result<()> {
    let mut app = TuiApp::new()?;
    app.needs_refresh = true;
    Ok(())
}
