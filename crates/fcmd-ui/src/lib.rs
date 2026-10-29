//! Widgets drawn into a cell buffer. Knows nothing about terminals: `fcmd-tui` takes the buffer
//! and puts it on screen. The interface is strictly TUI — this is the only renderer.

pub mod buffer;
pub mod color;
pub mod highlight;
pub mod svg;
pub mod theme;
pub mod widgets;

pub use buffer::{char_cols, elide_middle, truncate_cols, Border, Buffer, Cell, Rect, Style};
pub use color::{Color, ColorDepth};
pub use highlight::Highlight;
pub use svg::to_svg;
pub use theme::{Theme, ThemeError, ThemeSet, ThemeSource, Variant, BUILTIN_THEMES, DEFAULT_THEME};

pub use widgets::{
    brief_columns, clock_now, draw_bar, draw_buttons, draw_choice, draw_cmdline, draw_dialog,
    draw_keybar, draw_panel, draw_progress, draw_prompt, draw_prompt_at, draw_status, format_mtime,
    format_stamp, format_stamp_in, format_stamp_utc, mnemonic, panel_rows, parse_stamp,
    parse_stamp_in, parse_stamp_utc, PanelRender, ProgressView, KEYBAR_SLOTS,
};
