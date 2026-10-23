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
    FileEditNew,
    FileCopy,
    FileMove,
    FileMkdir,
    FileDelete,
    FileDirSize,

    ToolUserMenu,
    ToolFind,
    ShellOutputShow,

    // netbox (SPEC 04 §6)
    /// The site list: connect, disconnect, add.
    NetSites,
    /// Connect to a URL typed by hand.
    NetConnect,
    /// Close the connection the active panel is on.
    NetDisconnect,

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
    /// Disassemble: machine code as instructions.
    ViewerDisasm,
    /// Decompile: WebAssembly to text, or an external decompiler's output.
    ViewerDecompile,

    // Editor (scope `editor`). Movement and typing are handled directly by the editor; the
    // keymap binds its commands.
    EditorQuit,
    EditorSave,
    EditorSaveAs,
    EditorReload,
    EditorUndo,
    EditorRedo,
    EditorSearch,
    EditorSearchNext,
    EditorSearchPrev,
    EditorReplace,
    EditorGoto,
    EditorSelectAll,
    EditorCopy,
    EditorCut,
    EditorPaste,
    EditorDeleteLine,
    EditorDuplicateLine,
    EditorMoveLineUp,
    EditorMoveLineDown,
    EditorIndent,
    EditorOutdent,
    EditorDeleteToEnd,
    EditorDeleteWordLeft,
    EditorToggleReadOnly,
}

impl Command {
    /// Every command, for `scmd --list-commands`, the command palette and the docs.
    pub fn all() -> Vec<Command> {
        let mut out: Vec<Command> = vec![
            Command::AppQuit,
            Command::AppHelp,
            Command::AppMenu,
            Command::AppPalette,
            Command::AppThemeNext,
            Command::PanelFocusToggle,
            Command::PanelSwap,
            Command::PanelReread,
            Command::PanelHiddenToggle,
            Command::PanelSortReverse,
            Command::PanelFilterQuick,
            Command::PanelOtherSync,
            Command::NavUp,
            Command::NavDown,
            Command::NavPageUp,
            Command::NavPageDown,
            Command::NavTop,
            Command::NavBottom,
            Command::NavEnter,
            Command::NavParent,
            Command::NavRoot,
            Command::NavBack,
            Command::NavForward,
            Command::NavGotoPath,
            Command::SelToggle,
            Command::SelAll,
            Command::SelNone,
            Command::SelInvert,
            Command::SelMaskAdd,
            Command::SelMaskRemove,
            Command::SelRestore,
            Command::FileView,
            Command::FileEdit,
            Command::FileEditNew,
            Command::FileCopy,
            Command::FileMove,
            Command::FileMkdir,
            Command::FileDelete,
            Command::FileDirSize,
            Command::ToolUserMenu,
            Command::ToolFind,
            Command::ShellOutputShow,
            Command::NetSites,
            Command::NetConnect,
            Command::NetDisconnect,
            Command::ViewerQuit,
            Command::ViewerWrap,
            Command::ViewerHex,
            Command::ViewerModeNext,
            Command::ViewerSearch,
            Command::ViewerSearchNext,
            Command::ViewerSearchPrev,
            Command::ViewerGoto,
            Command::ViewerCodepageNext,
            Command::ViewerFollow,
            Command::ViewerLeft,
            Command::ViewerRight,
            Command::ViewerUndoPosition,
            Command::ViewerBytesLess,
            Command::ViewerBytesMore,
            Command::ViewerDisasm,
            Command::ViewerDecompile,
            Command::EditorQuit,
            Command::EditorSave,
            Command::EditorSaveAs,
            Command::EditorReload,
            Command::EditorUndo,
            Command::EditorRedo,
            Command::EditorSearch,
            Command::EditorSearchNext,
            Command::EditorSearchPrev,
            Command::EditorReplace,
            Command::EditorGoto,
            Command::EditorSelectAll,
            Command::EditorCopy,
            Command::EditorCut,
            Command::EditorPaste,
            Command::EditorDeleteLine,
            Command::EditorDuplicateLine,
            Command::EditorMoveLineUp,
            Command::EditorMoveLineDown,
            Command::EditorIndent,
            Command::EditorOutdent,
            Command::EditorDeleteToEnd,
            Command::EditorDeleteWordLeft,
            Command::EditorToggleReadOnly,
        ];
        for v in [
            ViewMode::Brief,
            ViewMode::Full,
            ViewMode::Detail,
            ViewMode::Wide,
        ] {
            out.push(Command::PanelViewSet(v));
        }
        for k in [
            SortKey::Name,
            SortKey::Ext,
            SortKey::Size,
            SortKey::MTime,
            SortKey::Unsorted,
        ] {
            out.push(Command::PanelSortBy(k));
        }
        out.sort_by_key(|c| c.id());
        out
    }

    /// The stable string id used in keymaps, menus, scripts and `scmd --list-commands`.
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
            Command::FileEditNew => "file.edit.new".into(),
            Command::FileCopy => "file.copy".into(),
            Command::FileMove => "file.move".into(),
            Command::FileMkdir => "file.mkdir".into(),
            Command::FileDelete => "file.delete".into(),
            Command::FileDirSize => "file.dirsize".into(),
            Command::ToolUserMenu => "tool.usermenu".into(),
            Command::ToolFind => "tool.find".into(),
            Command::ShellOutputShow => "shell.output.show".into(),
            Command::NetSites => "net.sites".into(),
            Command::NetConnect => "net.connect".into(),
            Command::NetDisconnect => "net.disconnect".into(),
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
            Command::ViewerDisasm => "viewer.disasm".into(),
            Command::ViewerDecompile => "viewer.decompile".into(),
            Command::EditorQuit => "editor.quit".into(),
            Command::EditorSave => "editor.save".into(),
            Command::EditorSaveAs => "editor.save.as".into(),
            Command::EditorReload => "editor.reload".into(),
            Command::EditorUndo => "editor.undo".into(),
            Command::EditorRedo => "editor.redo".into(),
            Command::EditorSearch => "editor.search".into(),
            Command::EditorSearchNext => "editor.search.next".into(),
            Command::EditorSearchPrev => "editor.search.prev".into(),
            Command::EditorReplace => "editor.replace".into(),
            Command::EditorGoto => "editor.goto".into(),
            Command::EditorSelectAll => "editor.select.all".into(),
            Command::EditorCopy => "editor.copy".into(),
            Command::EditorCut => "editor.cut".into(),
            Command::EditorPaste => "editor.paste".into(),
            Command::EditorDeleteLine => "editor.line.delete".into(),
            Command::EditorDuplicateLine => "editor.line.duplicate".into(),
            Command::EditorMoveLineUp => "editor.line.up".into(),
            Command::EditorMoveLineDown => "editor.line.down".into(),
            Command::EditorIndent => "editor.indent".into(),
            Command::EditorOutdent => "editor.outdent".into(),
            Command::EditorDeleteToEnd => "editor.delete.to.end".into(),
            Command::EditorDeleteWordLeft => "editor.delete.word.left".into(),
            Command::EditorToggleReadOnly => "editor.readonly.toggle".into(),
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
            ("file.edit.new", _) => Command::FileEditNew,
            ("file.copy", _) => Command::FileCopy,
            ("file.move", _) => Command::FileMove,
            ("file.mkdir", _) => Command::FileMkdir,
            ("file.delete", _) => Command::FileDelete,
            ("file.dirsize", _) => Command::FileDirSize,
            ("tool.usermenu", _) => Command::ToolUserMenu,
            ("tool.find", _) => Command::ToolFind,
            ("shell.output.show", _) => Command::ShellOutputShow,
            ("net.sites", _) => Command::NetSites,
            ("net.connect", _) => Command::NetConnect,
            ("net.disconnect", _) => Command::NetDisconnect,
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
            ("viewer.disasm", _) => Command::ViewerDisasm,
            ("viewer.decompile", _) => Command::ViewerDecompile,
            ("editor.quit", _) => Command::EditorQuit,
            ("editor.save", _) => Command::EditorSave,
            ("editor.save.as", _) => Command::EditorSaveAs,
            ("editor.reload", _) => Command::EditorReload,
            ("editor.undo", _) => Command::EditorUndo,
            ("editor.redo", _) => Command::EditorRedo,
            ("editor.search", _) => Command::EditorSearch,
            ("editor.search.next", _) => Command::EditorSearchNext,
            ("editor.search.prev", _) => Command::EditorSearchPrev,
            ("editor.replace", _) => Command::EditorReplace,
            ("editor.goto", _) => Command::EditorGoto,
            ("editor.select.all", _) => Command::EditorSelectAll,
            ("editor.copy", _) => Command::EditorCopy,
            ("editor.cut", _) => Command::EditorCut,
            ("editor.paste", _) => Command::EditorPaste,
            ("editor.line.delete", _) => Command::EditorDeleteLine,
            ("editor.line.duplicate", _) => Command::EditorDuplicateLine,
            ("editor.line.up", _) => Command::EditorMoveLineUp,
            ("editor.line.down", _) => Command::EditorMoveLineDown,
            ("editor.indent", _) => Command::EditorIndent,
            ("editor.outdent", _) => Command::EditorOutdent,
            ("editor.delete.to.end", _) => Command::EditorDeleteToEnd,
            ("editor.delete.word.left", _) => Command::EditorDeleteWordLeft,
            ("editor.readonly.toggle", _) => Command::EditorToggleReadOnly,
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
    use crate::Keymap;

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
            Command::ViewerDisasm,
            Command::ViewerDecompile,
            Command::EditorSave,
            Command::EditorPaste,
            Command::EditorToggleReadOnly,
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

    #[test]
    fn every_command_is_listed_and_round_trips() {
        let all = Command::all();
        let mut ids: Vec<String> = all.iter().map(|c| c.id()).collect();
        let before = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(before, ids.len(), "two commands share an id");
        for cmd in &all {
            let id = cmd.id();
            assert_eq!(
                Command::parse(&id),
                Some(*cmd),
                "`{id}` does not parse back to itself"
            );
        }
        // Everything the keymaps bind must be in the list.
        for profile in ["far", "mc"] {
            for (_, _, cmd) in Keymap::builtin(profile).unwrap().bindings() {
                assert!(
                    all.contains(&cmd),
                    "{profile} binds {cmd}, which `all()` omits"
                );
            }
        }
    }
}
