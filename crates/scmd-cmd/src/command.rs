use std::fmt;

use scmd_core::{SortKey, ViewMode};

/// Everything the application can be asked to do.
///
/// Keys, menu entries, macros and (later) plugins all resolve to one of these — nothing binds
/// behaviour directly. See `docs/03-keymap.md` §10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    AppQuit,
    AppHelp,
    AppMenu,
    AppPalette,
    AppThemeNext,

    PanelFocusToggle,
    PanelSwap,
    PanelReread,
    PanelHiddenToggle,
    PanelViewSet(ViewMode),
    PanelSortBy(SortKey),
    PanelSortReverse,
    PanelFilterQuick,
    PanelOtherSync,

    NavUp,
    NavDown,
    NavPageUp,
    NavPageDown,
    NavTop,
    NavBottom,
    NavEnter,
    NavParent,
    NavRoot,
    NavBack,
    NavForward,
    NavGotoPath,

    SelToggle,
    SelAll,
    SelNone,
    SelInvert,
    SelMaskAdd,
    SelMaskRemove,
    SelRestore,

    FileView,
    FileEdit,
    FileCopy,
    FileMove,
    FileMkdir,
    FileDelete,
    FileDirSize,

    ToolUserMenu,
    ToolFind,
    ShellOutputShow,

    // Viewer (scope `viewer`)
    ViewerQuit,
    ViewerWrap,
    ViewerHex,
    ViewerModeNext,
    ViewerSearch,
    ViewerSearchNext,
    ViewerSearchPrev,
    ViewerGoto,
    ViewerCodepageNext,
    ViewerFollow,
    ViewerLeft,
    ViewerRight,
    ViewerUndoPosition,
    ViewerBytesLess,
    ViewerBytesMore,
}

impl Command {
    /// The stable string id used in keymaps, menus, scripts and `sc --list-commands`.
    pub fn id(self) -> String {
        match self {
            Command::AppQuit => "app.quit".into(),
            Command::AppHelp => "app.help".into(),
            Command::AppMenu => "app.menu".into(),
            Command::AppPalette => "app.palette".into(),
            Command::AppThemeNext => "app.theme.next".into(),
            Command::PanelFocusToggle => "panel.focus.toggle".into(),
            Command::PanelSwap => "panel.swap".into(),
            Command::PanelReread => "panel.reread".into(),
            Command::PanelHiddenToggle => "panel.hidden.toggle".into(),
            Command::PanelViewSet(v) => format!("panel.view.set({})", v.as_str()),
            Command::PanelSortBy(k) => format!("panel.sort.by({})", k.as_str()),
            Command::PanelSortReverse => "panel.sort.reverse".into(),
            Command::PanelFilterQuick => "panel.filter.quick".into(),
            Command::PanelOtherSync => "panel.other.sync".into(),
            Command::NavUp => "nav.up".into(),
            Command::NavDown => "nav.down".into(),
            Command::NavPageUp => "nav.pageup".into(),
            Command::NavPageDown => "nav.pagedown".into(),
            Command::NavTop => "nav.top".into(),
            Command::NavBottom => "nav.bottom".into(),
            Command::NavEnter => "nav.enter".into(),
            Command::NavParent => "nav.parent".into(),
            Command::NavRoot => "nav.root".into(),
            Command::NavBack => "nav.history.back".into(),
            Command::NavForward => "nav.history.forward".into(),
            Command::NavGotoPath => "nav.goto.path".into(),
            Command::SelToggle => "sel.toggle".into(),
            Command::SelAll => "sel.all".into(),
            Command::SelNone => "sel.none".into(),
            Command::SelInvert => "sel.invert".into(),
            Command::SelMaskAdd => "sel.mask.add".into(),
            Command::SelMaskRemove => "sel.mask.remove".into(),
            Command::SelRestore => "sel.restore".into(),
            Command::FileView => "file.view".into(),
            Command::FileEdit => "file.edit".into(),
            Command::FileCopy => "file.copy".into(),
            Command::FileMove => "file.move".into(),
            Command::FileMkdir => "file.mkdir".into(),
            Command::FileDelete => "file.delete".into(),
            Command::FileDirSize => "file.dirsize".into(),
            Command::ToolUserMenu => "tool.usermenu".into(),
            Command::ToolFind => "tool.find".into(),
            Command::ShellOutputShow => "shell.output.show".into(),
            Command::ViewerQuit => "viewer.quit".into(),
            Command::ViewerWrap => "viewer.wrap".into(),
            Command::ViewerHex => "viewer.hex".into(),
            Command::ViewerModeNext => "viewer.mode.next".into(),
            Command::ViewerSearch => "viewer.search".into(),
            Command::ViewerSearchNext => "viewer.search.next".into(),
            Command::ViewerSearchPrev => "viewer.search.prev".into(),
            Command::ViewerGoto => "viewer.goto".into(),
            Command::ViewerCodepageNext => "viewer.codepage.next".into(),
            Command::ViewerFollow => "viewer.follow".into(),
            Command::ViewerLeft => "viewer.left".into(),
            Command::ViewerRight => "viewer.right".into(),
            Command::ViewerUndoPosition => "viewer.undo".into(),
            Command::ViewerBytesLess => "viewer.bytes.less".into(),
            Command::ViewerBytesMore => "viewer.bytes.more".into(),
        }
    }

    pub fn parse(s: &str) -> Option<Command> {
        let s = s.trim();
        let (name, arg) = match s.split_once('(') {
            Some((n, rest)) => (n.trim(), Some(rest.trim_end().trim_end_matches(')').trim())),
            None => (s, None),
        };
        let cmd = match (name, arg) {
            ("app.quit", _) => Command::AppQuit,
            ("app.help", _) => Command::AppHelp,
            ("app.menu", _) => Command::AppMenu,
            ("app.palette", _) => Command::AppPalette,
            ("app.theme.next", _) => Command::AppThemeNext,
            ("panel.focus.toggle", _) => Command::PanelFocusToggle,
            ("panel.swap", _) => Command::PanelSwap,
            ("panel.reread", _) => Command::PanelReread,
            ("panel.hidden.toggle", _) => Command::PanelHiddenToggle,
            ("panel.view.set", Some(a)) => Command::PanelViewSet(ViewMode::parse(a)?),
            ("panel.sort.by", Some(a)) => Command::PanelSortBy(parse_sort_key(a)?),
            ("panel.sort.reverse", _) => Command::PanelSortReverse,
            ("panel.filter.quick", _) => Command::PanelFilterQuick,
            ("panel.other.sync", _) => Command::PanelOtherSync,
            ("nav.up", _) => Command::NavUp,
            ("nav.down", _) => Command::NavDown,
            ("nav.pageup", _) => Command::NavPageUp,
            ("nav.pagedown", _) => Command::NavPageDown,
            ("nav.top", _) => Command::NavTop,
            ("nav.bottom", _) => Command::NavBottom,
            ("nav.enter", _) => Command::NavEnter,
            ("nav.parent", _) => Command::NavParent,
            ("nav.root", _) => Command::NavRoot,
            ("nav.history.back", _) => Command::NavBack,
            ("nav.history.forward", _) => Command::NavForward,
            ("nav.goto.path", _) => Command::NavGotoPath,
            ("sel.toggle", _) => Command::SelToggle,
            ("sel.all", _) => Command::SelAll,
            ("sel.none", _) => Command::SelNone,
            ("sel.invert", _) => Command::SelInvert,
            ("sel.mask.add", _) => Command::SelMaskAdd,
            ("sel.mask.remove", _) => Command::SelMaskRemove,
            ("sel.restore", _) => Command::SelRestore,
            ("file.view", _) => Command::FileView,
            ("file.edit", _) => Command::FileEdit,
            ("file.copy", _) => Command::FileCopy,
            ("file.move", _) => Command::FileMove,
            ("file.mkdir", _) => Command::FileMkdir,
            ("file.delete", _) => Command::FileDelete,
            ("file.dirsize", _) => Command::FileDirSize,
            ("tool.usermenu", _) => Command::ToolUserMenu,
            ("tool.find", _) => Command::ToolFind,
            ("shell.output.show", _) => Command::ShellOutputShow,
            ("viewer.quit", _) => Command::ViewerQuit,
            ("viewer.wrap", _) => Command::ViewerWrap,
            ("viewer.hex", _) => Command::ViewerHex,
            ("viewer.mode.next", _) => Command::ViewerModeNext,
            ("viewer.search", _) => Command::ViewerSearch,
            ("viewer.search.next", _) => Command::ViewerSearchNext,
            ("viewer.search.prev", _) => Command::ViewerSearchPrev,
            ("viewer.goto", _) => Command::ViewerGoto,
            ("viewer.codepage.next", _) => Command::ViewerCodepageNext,
            ("viewer.follow", _) => Command::ViewerFollow,
            ("viewer.left", _) => Command::ViewerLeft,
            ("viewer.right", _) => Command::ViewerRight,
            ("viewer.undo", _) => Command::ViewerUndoPosition,
            ("viewer.bytes.less", _) => Command::ViewerBytesLess,
            ("viewer.bytes.more", _) => Command::ViewerBytesMore,
            _ => return None,
        };
        Some(cmd)
    }

    /// Message id for the localised label (`scmd-i18n`): `cmd.app.quit`.
    pub fn label_key(self) -> String {
        let id = self.id();
        let base = id.split('(').next().unwrap_or(&id);
        format!("cmd.{base}")
    }
}

fn parse_sort_key(s: &str) -> Option<SortKey> {
    match s.trim().to_ascii_lowercase().as_str() {
        "name" => Some(SortKey::Name),
        "ext" | "extension" => Some(SortKey::Ext),
        "size" => Some(SortKey::Size),
        "mtime" | "time" | "date" => Some(SortKey::MTime),
        "none" | "unsorted" => Some(SortKey::Unsorted),
        _ => None,
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_id_round_trips() {
        let all = [
            Command::AppQuit,
            Command::PanelViewSet(ViewMode::Brief),
            Command::PanelViewSet(ViewMode::Detail),
            Command::PanelSortBy(SortKey::MTime),
            Command::PanelSortBy(SortKey::Unsorted),
            Command::SelMaskAdd,
            Command::FileDirSize,
            Command::ShellOutputShow,
            Command::ViewerQuit,
            Command::ViewerSearchNext,
            Command::ViewerBytesMore,
        ];
        for c in all {
            assert_eq!(Command::parse(&c.id()), Some(c), "{}", c.id());
        }
    }

    #[test]
    fn unknown_ids_are_rejected() {
        assert_eq!(Command::parse("panel.sort.by(colour)"), None);
        assert_eq!(Command::parse("nope"), None);
        assert_eq!(Command::parse("panel.view.set"), None);
    }

    #[test]
    fn label_keys_drop_arguments() {
        assert_eq!(
            Command::PanelSortBy(SortKey::Size).label_key(),
            "cmd.panel.sort.by"
        );
    }
}
