use crate::diff::FileChanges;
use ratatui::style::Color;
use std::collections::HashMap;

// ── Shared Color Palette ─────────────────────────────────────────────────
pub const ACCENT: Color = Color::Rgb(130, 170, 255);
pub const BORDER_FOCUSED: Color = Color::Rgb(130, 170, 255);
pub const BORDER_DIM: Color = Color::Rgb(55, 58, 65);
pub const FG_DIM: Color = Color::Rgb(100, 105, 115);
pub const FG_NORMAL: Color = Color::Rgb(190, 195, 205);
pub const FG_BRIGHT: Color = Color::Rgb(230, 233, 240);
pub const FG_ADDED: Color = Color::Rgb(80, 200, 100);
pub const FG_REMOVED: Color = Color::Rgb(225, 85, 85);
pub const FG_KEY: Color = Color::Rgb(220, 185, 100);
pub const BG_HEADER: Color = Color::Rgb(25, 28, 36);
pub const BG_SELECTION: Color = Color::Rgb(35, 48, 72);
pub const BG_ACCEPTED: Color = Color::Rgb(15, 40, 15);
pub const BG_REJECTED: Color = Color::Rgb(45, 15, 15);
pub const BG_MODAL_DIM: Color = Color::Rgb(10, 12, 18);
pub const BG_MODAL: Color = Color::Rgb(20, 22, 30);
pub const BORDER_MODAL: Color = Color::Rgb(80, 110, 180);
pub const BG_KEY_BADGE: Color = Color::Rgb(40, 45, 60);
pub const FG_SEPARATOR: Color = Color::Rgb(40, 44, 55);

pub enum AppMode {
    Diff,
    Rebase,
}

#[derive(Clone, PartialEq)]
pub enum ChangeState {
    Unselected,
    Accepted,
    Rejected,
}

#[derive(Clone, PartialEq)]
pub struct Change {
    pub line_num: usize,
    pub content: String,
    pub paired_content: Option<String>, // The paired line (if any)
    pub state: ChangeState,
    pub is_base: bool,
    pub context: Vec<String>,
    /// For unpaired additions: the computed base-file position to insert at.
    pub base_insert_pos: Option<usize>,
}

pub struct App<'a> {
    pub file_changes: &'a FileChanges,
    pub left_label: &'a str,
    pub right_label: &'a str,
    pub current_file_idx: usize,
    pub file_names: Vec<String>,
    pub scroll_positions: HashMap<String, u16>,
    pub focused_pane: Pane,
    pub view_mode: ViewMode,
    pub app_mode: AppMode,
    pub rebase_changes: HashMap<String, Vec<Change>>,
    pub current_change_idx: usize,
    pub rebase_notification: Option<String>,
    pub show_rebase_modal: bool,
    /// Transient status message shown in the help bar (cleared on next keypress)
    pub status_message: Option<String>,
    pub show_help_modal: bool,
}

pub enum Pane {
    FileList,
    DiffContent,
}

pub enum ViewMode {
    SideBySide,
    Unified,
}
