//! Widgets drawn into a cell buffer. Knows nothing about terminals: `sc-tui` takes the buffer
//! and puts it on screen. The interface is strictly TUI — this is the only renderer.

pub mod buffer;
pub mod color;
pub mod theme;
pub mod widgets;

pub use buffer::{elide_middle, truncate_cols, Buffer, Cell, Rect, Style};
pub use color::{Color, ColorDepth};
pub use theme::{Theme, ThemeError, ThemeSet, ThemeSource, Variant, BUILTIN_THEMES, DEFAULT_THEME};

pub use widgets::{
    brief_columns, draw_bar, draw_buttons, draw_choice, draw_cmdline, draw_dialog, draw_keybar,
    draw_panel, draw_progress, draw_prompt, draw_status, format_mtime, panel_rows, PanelRender,
    ProgressView,
};
