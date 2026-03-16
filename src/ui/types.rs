use crate::diff::FileChanges;
use std::collections::HashMap;

use super::theme::Theme;

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
    pub scroll_positions: HashMap<String, usize>,
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
    pub theme: Theme,
}

pub enum Pane {
    FileList,
    DiffContent,
}

pub enum ViewMode {
    SideBySide,
    Unified,
}
