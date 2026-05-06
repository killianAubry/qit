// UI-only presentational state. Nothing here affects the simulation.

#[derive(Clone, Debug)]
pub struct UiState {
    /// Whether the command palette overlay is visible.
    pub cmd_palette_open: bool,
    pub cmd_palette_input: String,

    /// Whether the tile-picker overlay is visible (Cmd+T).
    pub tile_picker_open: bool,
    pub tile_picker_input: String,

    /// Whether the config popup is visible (Cmd+,).
    pub config_popup_open: bool,

    /// One-line transient status message (drawn at the bottom).
    pub status_message: Option<(String, StatusKind)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    Info,
    Ok,
    Err,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            cmd_palette_open: false,
            cmd_palette_input: String::new(),
            tile_picker_open: false,
            tile_picker_input: String::new(),
            config_popup_open: false,
            status_message: None,
        }
    }
}

impl UiState {
    pub fn flash(&mut self, msg: impl Into<String>, kind: StatusKind) {
        self.status_message = Some((msg.into(), kind));
    }
}
