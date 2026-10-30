use std::fmt;

use fcmd_core::{SortKey, ViewMode};

/// Which panel a command acts on. FAR gives the two panels their own keys (`Alt+F1` and
/// `Alt+F2` for the drive list), so the side has to be part of the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Left,
    Right,
    /// Whichever panel has the focus.
    Active,
}

/// Which panel an insertion key reads: the one with the focus, the one without it, or the
/// one on a given side of the screen.
///
/// FAR has both pairs for the same reason a script does: *active* and *passive* follow the
/// focus, and *left* and *right* do not, so a line one writes once can mean either.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Which {
    Active,
    Passive,
    Left,
    Right,
}

impl Which {
    pub fn as_str(self) -> &'static str {
        match self {
            Which::Active => "active",
            Which::Passive => "passive",
            Which::Left => "left",
            Which::Right => "right",
        }
    }

    pub fn parse(text: &str) -> Option<Which> {
        match text.trim() {
            "" | "active" => Some(Which::Active),
            "passive" | "other" => Some(Which::Passive),
            "left" => Some(Which::Left),
            "right" => Some(Which::Right),
            _ => None,
        }
    }
}

/// What an insertion key puts on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Insert {
    /// The name under the cursor, on its own.
    Name,
    /// The panel's directory.
    Path,
    /// The directory and the name together.
    Full,
    /// The full name in the form another machine could use it: the URL of a remote path.
    /// A path on this disk has no such form, and the key says so rather than inserting the
    /// local path under a name that promises more.
    Unc,
}

impl Insert {
    pub fn as_str(self) -> &'static str {
        match self {
            Insert::Name => "name",
            Insert::Path => "path",
            Insert::Full => "full",
            Insert::Unc => "unc",
        }
    }

    pub fn parse(text: &str) -> Option<Insert> {
        match text.trim() {
            "name" => Some(Insert::Name),
            "path" => Some(Insert::Path),
            "full" => Some(Insert::Full),
            "unc" => Some(Insert::Unc),
            _ => None,
        }
    }
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Side::Left => "left",
            Side::Right => "right",
            Side::Active => "active",
        }
    }

    pub fn parse(s: &str) -> Option<Side> {
        match s.trim().to_ascii_lowercase().as_str() {
            "left" | "l" => Some(Side::Left),
            "right" | "r" => Some(Side::Right),
            "active" | "current" | "" => Some(Side::Active),
            _ => None,
        }
    }
}

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
    /// Ask the terminal for a font one step larger, one step smaller, or the one it started
    /// with. A program does not own its font; this is a request, and `docs/03-keymap.md` §9
    /// says which terminals answer it.
    AppFont(i8),
    /// The theme list (SPEC 10 §4).
    AppThemes,
    /// The language list (SPEC 10 §4).
    AppLanguages,
    /// Write the configuration file.
    AppSetupSave,

    // The settings dialogs (SPEC 10 §3).
    OptSystem,
    OptPanel,
    OptInterface,
    /// The `[[highlight]]` groups as a list, with a dialog per group (SPEC 10 §3.11).
    OptHighlight,
    /// The `[[sort_group]]` bands as a list, with a dialog per band (SPEC 02 §4).
    OptSortGroups,
    OptMaskGroups,
    /// The code pages as a list: which ones `F8` walks, and what to call each (SPEC 05 §1.12).
    /// The same list the viewer's `Shift+F8` opens — from here it is the favourites one edits
    /// rather than the page a file is being read in.
    OptCodepages,
    /// Every setting, as a list one types into: the key, the value, and `Enter` to change it
    /// (SPEC 07 §2e). What FAR reaches with `far:config`.
    OptSettings,
    OptConfirmations,
    /// The palette editor: every colour the interface uses, on the live theme.
    OptColors,
    /// How the editor behaves: tabs, indent, whitespace.
    OptEditor,
    /// How the viewer behaves: the chrome it opens with, the wrap and the tab width.
    OptViewer,

    PanelFocusToggle,
    PanelSwap,
    PanelReread,
    PanelHiddenToggle,
    /// A new tab on this side, on the directory under the cursor if the cursor is on one and
    /// otherwise on the one this tab is already showing (SPEC 02 §1.6).
    PanelTabNew,
    /// Close this tab. A side always has one, so the last one stays.
    PanelTabClose,
    PanelTabNext,
    PanelTabPrev,
    /// Hand this tab to the other side.
    PanelTabMove,
    /// Keep this tab on its directory, or let it move again.
    PanelTabLock,
    /// Give this tab a name, so the row says what it is for rather than where it is.
    PanelTabName,
    PanelViewSet(ViewMode),
    /// Turn the quick view on or off. The panel it names shows the viewer for whatever the
    /// *other* panel's cursor is on (SPEC 02 §7).
    PanelQuickView(Side),
    /// Turn the info panel on or off: the device, the directory and the selection.
    PanelInfo(Side),
    /// Turn the tree panel on or off. Moving its cursor changes the *other* panel's
    /// directory, which is what the tree is for.
    PanelTree(Side),
    PanelSortBy(SortKey),
    PanelSortReverse,
    /// Sort groups on or off (FAR's `Shift+F11`): the bands `[[sort_group]]` cuts the
    /// listing into, which pull some kinds of file ahead of the order and push others
    /// behind it whatever the sort key is.
    PanelSortGroups,
    /// Move the line between the two panels: a negative step gives the left one less.
    PanelSplit(i8),
    /// Make the panels a row shorter or a row taller: the user screen gets what they give up.
    ///
    /// The sign is the panels' *height*, which is what the keys read as — `Ctrl+Up` brings the
    /// bottom edge up, so the panels get shorter and the output below them gets longer.
    PanelHeight(i8),
    /// Directories before files, or mixed in with them (`Shift+F11`, FAR's `Group`).
    PanelDirsFirst,
    /// The selected files first in the order (`Shift+F12`, FAR's `SelUp`).
    PanelSelectedFirst,
    PanelFilterQuick,
    PanelOtherSync,
    /// Choose the sort order from a list.
    PanelSortMenu,
    /// Drives, mount points and open connections.
    PanelDrive(Side),
    /// Where this panel has been.
    PanelHistory,
    /// `Ctrl+F1`/`Ctrl+F2`: hide one panel and give the screen to the other.
    PanelToggle(Side),
    /// `Ctrl+P`: hide the panel that does not have the focus, which is FAR's own key for
    /// giving the screen to the one you are working in without first saying which it is.
    PanelToggleOther,
    /// Scroll a name column that is narrower than the names.
    PanelScrollNames(i8),

    NavUp,
    NavDown,
    NavPageUp,
    NavPageDown,
    NavTop,
    NavBottom,
    NavEnter,
    /// One column left in a multi-column view; in the tree panel, collapse or climb.
    NavLeft,
    /// One column right in a multi-column view; in the tree panel, expand.
    NavRight,
    NavParent,
    NavRoot,
    NavBack,
    NavForward,
    NavGotoPath,
    /// Put this panel's directory on folder shortcut `0`..`9` (SPEC 02 §11).
    NavBookmarkSet(u8),
    /// Go to the directory on folder shortcut `0`..`9`.
    NavBookmarkGo(u8),
    /// The whole hotlist, as a list to pick from.
    NavBookmarks,

    SelToggle,
    SelAll,
    SelNone,
    SelInvert,
    SelMaskAdd,
    SelMaskRemove,
    SelRestore,
    /// FAR's modifiers on the grey keys: the criterion changes, not the mechanics.
    /// Everything with the same extension as the file under the cursor.
    SelExtAdd,
    SelExtRemove,
    /// Everything with the same name, whatever the extension.
    SelNameAdd,
    SelNameRemove,
    /// Invert, directories included.
    SelInvertAll,

    FileView,
    FileEdit,
    FileEditNew,
    FileCopy,
    FileMove,
    FileMkdir,
    FileDelete,
    FileDirSize,
    /// `Shift+F5`, `Shift+F6`, `Shift+F8`: the item under the cursor, whatever is selected.
    FileCopyCurrent,
    FileMoveCurrent,
    FileDeleteCurrent,
    /// Delete outright, past the system's wastebasket, whatever the setting says.
    FileDeleteForce,
    /// Everything that could open the file under the cursor, as a list (SPEC 06 §5).
    FileAssoc,
    /// Read this file with what an `[[assoc]]` block names, or with `$PAGER`.
    FileViewAlt,
    /// Write it with what a block names, or with `$VISUAL` / `$EDITOR`.
    FileEditAlt,
    /// Write a list of digests for what is selected (SPEC 03 §9).
    /// Describe the file under the cursor: the line the directory's `descript.ion` keeps
    /// about it (FAR's `Ctrl+Z`).
    FileDescribe,
    /// Put what is selected into the temporary panel on the other side (SPEC 02 §7b).
    PanelTempAdd,
    /// Run a command and put the paths it printed into the panel (Midnight Commander's
    /// external panelize, SPEC 02 §7b).
    PanelPanelize,
    FileChecksum,
    /// Check the files beside a list of digests against it.
    FileChecksumVerify,
    /// `Ctrl+G`: run a command line once per selected file (SPEC 06 §5).
    FileApply,
    /// `Alt+F6`: a hard link or a symbolic link.
    FileLink,
    /// `Alt+Del`: overwrite, then unlink.
    FileWipe,
    /// What a file is, rather than what is in it: the permissions and the times.
    FileAttributes,
    /// `Ctrl+Ins`: the names of what is selected onto the system clipboard, one per line.
    FileClipNames,
    /// `Alt+Shift+Ins`: the same, with the whole path on each line.
    FileClipPaths,

    /// Run what is on the command line (SPEC 06 §7).
    CmdRun,
    /// Empty it.
    CmdClear,
    CmdHistoryPrev,
    CmdHistoryNext,
    /// The command history as a list.
    CmdHistory,
    /// The files that have been read or edited, as a list (`Alt+F11`).
    CmdFileHistory,
    /// `Ctrl+Ins` with something on the line: the line itself onto the system clipboard.
    CmdClip,
    /// Ask the command line what it could become (SPEC 06 §7.4).
    CmdComplete,
    /// Insert the name under the cursor, or the panel's path, into the line.
    /// Put something about a panel on the command line: FAR's `Ctrl+Enter` family, as one
    /// command with a table behind it rather than eleven of them.
    CmdInsert(Insert, Which),
    /// Put the selected files into a new archive (`Shift+F1`).
    FilePack,
    /// Take an archive apart, into a directory (`Shift+F2`).
    FileUnpack,
    /// What can be done with the archive under the cursor, as a menu (`Shift+F3`).
    ArcCommands,
    /// Read every member of the archive under the cursor and say whether it comes out whole.
    ArcTest,
    /// Open a tunnel over the connection the active panel is on: `ssh -L`, without the ssh.
    NetTunnel,
    /// The tunnels that are open, with what has gone through them.
    NetTunnels,
    ToolUserMenu,
    /// Compare the two panels by name, size and time, and select what differs.
    ToolCompareDirs,
    /// The same comparison, with the dialog of options in front of it: what makes a pair,
    /// what counts as the same, whether the bytes are read (SPEC 03 §11).
    ToolCompare,
    /// Find the files under this panel's directory that are copies of each other, and show
    /// them grouped (SPEC 03 §12).
    ToolDuplicates,
    /// The running processes as a panel — FAR's *Task list*, which is the operating system's
    /// processes and not this program's own screens (`F12` is those).
    ToolTaskList,
    /// The screen grabber: copy a piece of the screen as it stands (SPEC 06 §11b).
    ToolGrab,
    /// Make the two panels' directories hold the same thing: decide about every pair, show
    /// the table, and only then copy and delete (SPEC 03 §10).
    ToolSyncDirs,
    /// Compare the two panels by what is *in* the files: every pair with the same name is
    /// read on both sides and selected where the bytes differ. The slow, certain one.
    ToolCompareContents,
    /// The two files under the cursors, side by side, with what differs marked (SPEC 05 §3).
    ToolDiff,
    /// A sixty-four bit calculator, in every base at once.
    ToolCalculator,
    /// A real terminal, with a shell in it (SPEC 06 §4).
    ToolTerminal,
    ToolFind,
    /// `Alt+F10`: the folders the tree knows, as a list one types into (SPEC 02 §7).
    ToolFindFolder,
    /// Replace across the selected files, with a dry run first.
    ToolReplace,
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
    /// Markdown as it is meant to be read, or its source.
    ViewerMarkdown,
    /// A tabular file as a table, and back to its source (SPEC 05 §1.11).
    ViewerTable,
    /// A base64 file decoded, and back to the characters it is written in (SPEC 05 §1.13).
    ViewerBase64,
    /// Sort the table by the column the cursor is in: ascending, descending, then the order
    /// the records are in the file.
    ViewerTableSort,
    /// Keep only the table's rows that hold what is typed.
    ViewerTableFilter,
    ViewerModeNext,
    ViewerSearch,
    ViewerSearchNext,
    ViewerSearchPrev,
    ViewerGoto,
    ViewerCodepageNext,
    ViewerCodepageList,
    ViewerFollow,
    ViewerLeft,
    ViewerRight,
    ViewerUndoPosition,
    /// Move the whole dump one byte back or on, so a structure that does not begin on a
    /// multiple of the row width can be read in line. FAR's `Ctrl+Left` and `Ctrl+Right`.
    ViewerShiftBack,
    ViewerShiftForward,
    ViewerBytesLess,
    ViewerBytesMore,
    /// Disassemble: machine code as instructions.
    /// Follow the branch on the current row (SPEC 05 §1.9).
    ViewerFollowBranch,
    /// Back where the last branch was followed from.
    ViewerBack,
    /// The structure pane: what the file is and where anything inside it is (SPEC 05 §1.8).
    ViewerStructure,
    /// Take the key bar away, or put it back: FAR's `Ctrl+B`. The three that follow are the
    /// chrome a reader is allowed to remove one row at a time, to give a long line or a wide
    /// dump the whole window.
    ViewerChromeKeyBar,
    /// The top line — the name and the position: FAR's `Ctrl+Shift+B`.
    ViewerChromeStatus,
    /// The scrollbar down the right edge: FAR's `Ctrl+S`.
    ViewerChromeScrollbar,
    /// Move the keys between the content and the structure pane.
    ViewerPaneNext,
    ViewerDisasm,
    /// `F6`: the same file in the editor, at the same place.
    ViewerToEditor,
    /// `Gray +`/`Gray -`: the next and previous file of the panel, without going back.
    ViewerNextFile,
    ViewerPrevFile,
    /// `Ctrl+F10`: put the panel's cursor on the file being viewed.
    ViewerToPanel,
    /// Decompile: WebAssembly to text, or an external decompiler's output.
    ViewerDecompile,
    /// Run a command and read its output instead of the file: Midnight Commander's filtered
    /// view (SPEC 05 §1).
    ViewerFilter,

    // Editor (scope `editor`). Movement and typing are handled directly by the editor; the
    // keymap binds its commands.
    EditorQuit,
    EditorSave,
    EditorSaveAs,
    /// `Shift+F10`: save and leave, in one key — FAR's own.
    EditorSaveQuit,
    EditorReload,
    EditorUndo,
    EditorRedo,
    EditorSearch,
    EditorSearchNext,
    EditorSearchPrev,
    EditorReplace,
    EditorGoto,
    EditorSelectAll,
    /// `F6`: the same file in the viewer, at the same place.
    EditorToViewer,
    /// `Alt+H`: the same file as bytes instead of lines, or back again (SPEC 05 §2.5).
    EditorHexToggle,
    /// `F3` in hex mode: start or finish marking a block (SPEC 05 §2.6).
    EditorBlockMark,
    /// `F12` in hex mode: what to do with the block that is marked.
    EditorBlockOps,
    /// `F9` in hex mode: type one instruction and write its bytes (SPEC 05 §2.8).
    EditorAssemble,
    /// `F11`: the commands the loaded plugins provide (SPEC 08 §1).
    PluginCommands,
    /// `Ctrl+.`: start recording a macro, or stop and assign it to a key (SPEC 08 §5).
    MacroRecord,
    /// `Ctrl+Shift+.`: the recorded macros, to play or delete one.
    MacroList,
    /// `F12`: the viewers and editors left open, to step back into one (SPEC 02 §7).
    AppScreens,
    /// `Ctrl+Tab` / `Ctrl+Shift+Tab`: the next or previous screen, without the list.
    AppScreenNext,
    AppScreenPrev,
    EditorCopy,
    EditorCut,
    EditorPaste,
    EditorDeleteLine,
    EditorDuplicateLine,
    EditorMoveLineUp,
    EditorMoveLineDown,
    EditorIndent,
    /// Move the block, or the line the cursor is on, one indent to the right.
    EditorShiftRight,
    /// Add the block to what is on the clipboard instead of replacing it.
    EditorAppend,
    /// The next key typed goes in as itself, control characters included.
    EditorLiteral,
    /// The cursor to the first and the last line on the screen.
    EditorScreenTop,
    EditorScreenBottom,
    EditorOutdent,
    EditorDeleteToEnd,
    EditorDeleteWordLeft,
    /// `Ctrl+Del`, `Ctrl+T`: the word after the cursor.
    EditorDeleteWordRight,
    EditorToggleReadOnly,
}

impl Command {
    /// Every command, for `fcmd --list-commands`, the command palette and the docs.
    pub fn all() -> Vec<Command> {
        let mut out: Vec<Command> = vec![
            Command::AppQuit,
            Command::AppHelp,
            Command::AppMenu,
            Command::AppPalette,
            Command::AppThemeNext,
            Command::AppFont(-1),
            Command::AppFont(1),
            Command::AppFont(0),
            Command::AppThemes,
            Command::AppLanguages,
            Command::AppSetupSave,
            Command::OptSystem,
            Command::OptPanel,
            Command::OptInterface,
            Command::OptHighlight,
            Command::OptSortGroups,
            Command::OptMaskGroups,
            Command::OptCodepages,
            Command::OptSettings,
            Command::OptConfirmations,
            Command::OptColors,
            Command::OptEditor,
            Command::OptViewer,
            Command::PanelFocusToggle,
            Command::PanelSwap,
            Command::PanelReread,
            Command::PanelHiddenToggle,
            Command::PanelTabNew,
            Command::PanelTabClose,
            Command::PanelTabNext,
            Command::PanelTabPrev,
            Command::PanelTabMove,
            Command::PanelTabLock,
            Command::PanelTabName,
            Command::PanelSortReverse,
            Command::PanelSortGroups,
            Command::PanelSplit(-1),
            Command::PanelSplit(1),
            Command::PanelHeight(-1),
            Command::PanelHeight(1),
            Command::PanelDirsFirst,
            Command::PanelSelectedFirst,
            Command::PanelFilterQuick,
            Command::PanelOtherSync,
            Command::PanelSortMenu,
            Command::PanelDrive(Side::Active),
            Command::PanelDrive(Side::Left),
            Command::PanelDrive(Side::Right),
            Command::PanelHistory,
            Command::NavUp,
            Command::NavDown,
            Command::NavPageUp,
            Command::NavPageDown,
            Command::NavTop,
            Command::NavBottom,
            Command::NavEnter,
            Command::NavLeft,
            Command::NavRight,
            Command::NavParent,
            Command::NavRoot,
            Command::NavBack,
            Command::NavForward,
            Command::NavGotoPath,
            Command::NavBookmarks,
            Command::SelToggle,
            Command::SelAll,
            Command::SelNone,
            Command::SelInvert,
            Command::SelMaskAdd,
            Command::SelMaskRemove,
            Command::SelRestore,
            Command::SelExtAdd,
            Command::SelExtRemove,
            Command::SelNameAdd,
            Command::SelNameRemove,
            Command::SelInvertAll,
            Command::FileCopyCurrent,
            Command::FileMoveCurrent,
            Command::FileDeleteCurrent,
            Command::FileDeleteForce,
            Command::FileAssoc,
            Command::FileViewAlt,
            Command::FileEditAlt,
            Command::FileDescribe,
            Command::PanelTempAdd,
            Command::PanelPanelize,
            Command::FileChecksum,
            Command::FileChecksumVerify,
            Command::FileApply,
            Command::FileLink,
            Command::FileWipe,
            Command::FileClipNames,
            Command::FileClipPaths,
            Command::FileAttributes,
            Command::PanelToggle(Side::Active),
            Command::PanelToggleOther,
            Command::PanelToggle(Side::Left),
            Command::PanelToggle(Side::Right),
            Command::PanelScrollNames(-1),
            Command::PanelScrollNames(1),
            Command::PanelQuickView(Side::Active),
            Command::PanelQuickView(Side::Left),
            Command::PanelQuickView(Side::Right),
            Command::PanelInfo(Side::Active),
            Command::PanelInfo(Side::Left),
            Command::PanelInfo(Side::Right),
            Command::PanelTree(Side::Active),
            Command::PanelTree(Side::Left),
            Command::PanelTree(Side::Right),
            Command::FileView,
            Command::FileEdit,
            Command::FileEditNew,
            Command::FileCopy,
            Command::FileMove,
            Command::FileMkdir,
            Command::FileDelete,
            Command::FileDirSize,
            Command::CmdRun,
            Command::CmdClear,
            Command::CmdHistoryPrev,
            Command::CmdHistoryNext,
            Command::CmdHistory,
            Command::CmdFileHistory,
            Command::CmdClip,
            Command::CmdComplete,
            Command::CmdInsert(Insert::Name, Which::Active),
            Command::CmdInsert(Insert::Name, Which::Passive),
            Command::CmdInsert(Insert::Name, Which::Left),
            Command::CmdInsert(Insert::Name, Which::Right),
            Command::CmdInsert(Insert::Path, Which::Active),
            Command::CmdInsert(Insert::Path, Which::Passive),
            Command::CmdInsert(Insert::Path, Which::Left),
            Command::CmdInsert(Insert::Path, Which::Right),
            Command::CmdInsert(Insert::Full, Which::Active),
            Command::CmdInsert(Insert::Full, Which::Passive),
            Command::CmdInsert(Insert::Full, Which::Left),
            Command::CmdInsert(Insert::Full, Which::Right),
            Command::CmdInsert(Insert::Unc, Which::Active),
            Command::CmdInsert(Insert::Unc, Which::Passive),
            Command::CmdInsert(Insert::Unc, Which::Left),
            Command::CmdInsert(Insert::Unc, Which::Right),
            Command::FilePack,
            Command::FileUnpack,
            Command::ArcCommands,
            Command::ArcTest,
            Command::NetTunnel,
            Command::NetTunnels,
            Command::ToolUserMenu,
            Command::ToolCompareDirs,
            Command::ToolCompare,
            Command::ToolDuplicates,
            Command::ToolTaskList,
            Command::ToolGrab,
            Command::ToolSyncDirs,
            Command::ToolCompareContents,
            Command::ToolDiff,
            Command::ToolCalculator,
            Command::ToolTerminal,
            Command::ToolFind,
            Command::ToolFindFolder,
            Command::ToolReplace,
            Command::ShellOutputShow,
            Command::NetSites,
            Command::NetConnect,
            Command::NetDisconnect,
            Command::ViewerQuit,
            Command::ViewerWrap,
            Command::ViewerHex,
            Command::ViewerMarkdown,
            Command::ViewerTable,
            Command::ViewerBase64,
            Command::ViewerTableSort,
            Command::ViewerTableFilter,
            Command::ViewerModeNext,
            Command::ViewerSearch,
            Command::ViewerSearchNext,
            Command::ViewerSearchPrev,
            Command::ViewerGoto,
            Command::ViewerCodepageNext,
            Command::ViewerCodepageList,
            Command::ViewerFollow,
            Command::ViewerLeft,
            Command::ViewerRight,
            Command::ViewerUndoPosition,
            Command::ViewerShiftBack,
            Command::ViewerShiftForward,
            Command::ViewerBytesLess,
            Command::ViewerBytesMore,
            Command::ViewerFollowBranch,
            Command::ViewerBack,
            Command::ViewerStructure,
            Command::ViewerChromeKeyBar,
            Command::ViewerChromeStatus,
            Command::ViewerChromeScrollbar,
            Command::ViewerPaneNext,
            Command::ViewerDisasm,
            Command::ViewerDecompile,
            Command::ViewerFilter,
            Command::ViewerToEditor,
            Command::ViewerNextFile,
            Command::ViewerPrevFile,
            Command::ViewerToPanel,
            Command::EditorQuit,
            Command::EditorSave,
            Command::EditorSaveAs,
            Command::EditorSaveQuit,
            Command::EditorReload,
            Command::EditorUndo,
            Command::EditorRedo,
            Command::EditorSearch,
            Command::EditorSearchNext,
            Command::EditorSearchPrev,
            Command::EditorReplace,
            Command::EditorGoto,
            Command::EditorSelectAll,
            Command::EditorToViewer,
            Command::EditorHexToggle,
            Command::EditorBlockMark,
            Command::EditorBlockOps,
            Command::EditorAssemble,
            Command::PluginCommands,
            Command::MacroRecord,
            Command::MacroList,
            Command::AppScreens,
            Command::AppScreenNext,
            Command::AppScreenPrev,
            Command::EditorCopy,
            Command::EditorCut,
            Command::EditorPaste,
            Command::EditorDeleteLine,
            Command::EditorDuplicateLine,
            Command::EditorMoveLineUp,
            Command::EditorMoveLineDown,
            Command::EditorIndent,
            Command::EditorShiftRight,
            Command::EditorAppend,
            Command::EditorLiteral,
            Command::EditorScreenTop,
            Command::EditorScreenBottom,
            Command::EditorOutdent,
            Command::EditorDeleteToEnd,
            Command::EditorDeleteWordLeft,
            Command::EditorDeleteWordRight,
            Command::EditorToggleReadOnly,
        ];
        for v in ViewMode::ALL {
            out.push(Command::PanelViewSet(v));
        }
        // Ten folder shortcuts, which is FAR's own number: a digit is the whole point of
        // them, so they are written out rather than taking an argument from a dialog.
        for n in 0..=9u8 {
            out.push(Command::NavBookmarkSet(n));
            out.push(Command::NavBookmarkGo(n));
        }
        for k in [
            SortKey::Name,
            SortKey::Ext,
            SortKey::Size,
            SortKey::MTime,
            SortKey::BTime,
            SortKey::ATime,
            SortKey::Descr,
            SortKey::Owner,
            SortKey::Unsorted,
        ] {
            out.push(Command::PanelSortBy(k));
        }
        out.sort_by_key(|c| c.id());
        out
    }

    /// The stable string id used in keymaps, menus, scripts and `fcmd --list-commands`.
    pub fn id(self) -> String {
        match self {
            Command::AppQuit => "app.quit".into(),
            Command::AppHelp => "app.help".into(),
            Command::AppMenu => "app.menu".into(),
            Command::AppPalette => "app.palette".into(),
            Command::AppThemeNext => "app.theme.next".into(),
            Command::AppFont(by) => format!("app.font({by})"),
            Command::AppThemes => "app.themes".into(),
            Command::AppLanguages => "app.languages".into(),
            Command::AppSetupSave => "app.setup.save".into(),
            Command::OptSystem => "opt.system".into(),
            Command::OptPanel => "opt.panel".into(),
            Command::OptInterface => "opt.interface".into(),
            Command::OptHighlight => "opt.highlight".into(),
            Command::OptSortGroups => "opt.sort.groups".into(),
            Command::OptMaskGroups => "opt.mask.groups".into(),
            Command::OptCodepages => "opt.codepages".into(),
            Command::OptSettings => "opt.settings".into(),
            Command::OptConfirmations => "opt.confirmations".into(),
            Command::OptColors => "opt.colors".into(),
            Command::OptEditor => "opt.editor".into(),
            Command::OptViewer => "opt.viewer".into(),
            Command::PanelFocusToggle => "panel.focus.toggle".into(),
            Command::PanelSwap => "panel.swap".into(),
            Command::PanelReread => "panel.reread".into(),
            Command::PanelHiddenToggle => "panel.hidden.toggle".into(),
            Command::PanelTabNew => "panel.tab.new".into(),
            Command::PanelTabClose => "panel.tab.close".into(),
            Command::PanelTabNext => "panel.tab.next".into(),
            Command::PanelTabPrev => "panel.tab.prev".into(),
            Command::PanelTabMove => "panel.tab.move".into(),
            Command::PanelTabLock => "panel.tab.lock".into(),
            Command::PanelTabName => "panel.tab.name".into(),
            Command::PanelViewSet(v) => format!("panel.view.set({})", v.as_str()),
            Command::PanelSortBy(k) => format!("panel.sort.by({})", k.as_str()),
            Command::PanelSortReverse => "panel.sort.reverse".into(),
            Command::PanelSortGroups => "panel.sort.groups".into(),
            Command::PanelSplit(by) => format!("panel.split({by})"),
            Command::PanelHeight(by) => format!("panel.height({by})"),
            Command::PanelDirsFirst => "panel.dirs.first".into(),
            Command::PanelSelectedFirst => "panel.selected.first".into(),
            Command::PanelFilterQuick => "panel.filter.quick".into(),
            Command::PanelOtherSync => "panel.other.sync".into(),
            Command::PanelSortMenu => "panel.sort.menu".into(),
            Command::PanelDrive(Side::Active) => "panel.drive".into(),
            Command::PanelDrive(side) => format!("panel.drive({})", side.as_str()),
            Command::PanelHistory => "panel.history".into(),
            Command::NavUp => "nav.up".into(),
            Command::NavDown => "nav.down".into(),
            Command::NavPageUp => "nav.pageup".into(),
            Command::NavPageDown => "nav.pagedown".into(),
            Command::NavTop => "nav.top".into(),
            Command::NavBottom => "nav.bottom".into(),
            Command::NavEnter => "nav.enter".into(),
            Command::NavLeft => "nav.left".into(),
            Command::NavRight => "nav.right".into(),
            Command::NavParent => "nav.parent".into(),
            Command::NavRoot => "nav.root".into(),
            Command::NavBack => "nav.history.back".into(),
            Command::NavForward => "nav.history.forward".into(),
            Command::NavGotoPath => "nav.goto.path".into(),
            Command::NavBookmarkSet(n) => format!("nav.bookmark.set({n})"),
            Command::NavBookmarkGo(n) => format!("nav.bookmark.go({n})"),
            Command::NavBookmarks => "nav.bookmarks".into(),
            Command::SelToggle => "sel.toggle".into(),
            Command::SelAll => "sel.all".into(),
            Command::SelNone => "sel.none".into(),
            Command::SelInvert => "sel.invert".into(),
            Command::SelMaskAdd => "sel.mask.add".into(),
            Command::SelMaskRemove => "sel.mask.remove".into(),
            Command::SelRestore => "sel.restore".into(),
            Command::SelExtAdd => "sel.ext.add".into(),
            Command::SelExtRemove => "sel.ext.remove".into(),
            Command::SelNameAdd => "sel.name.add".into(),
            Command::SelNameRemove => "sel.name.remove".into(),
            Command::SelInvertAll => "sel.invert.all".into(),
            Command::FileCopyCurrent => "file.copy.current".into(),
            Command::FileMoveCurrent => "file.move.current".into(),
            Command::FileDeleteCurrent => "file.delete.current".into(),
            Command::FileDeleteForce => "file.delete.permanent".into(),
            Command::FileAssoc => "file.assoc".into(),
            Command::FileViewAlt => "file.view.external".into(),
            Command::FileEditAlt => "file.edit.external".into(),
            Command::FileDescribe => "file.describe".into(),
            Command::PanelTempAdd => "panel.temp.add".into(),
            Command::PanelPanelize => "panel.panelize".into(),
            Command::FileChecksum => "file.checksum.create".into(),
            Command::FileChecksumVerify => "file.checksum.verify".into(),
            Command::FileApply => "file.apply".into(),
            Command::FileLink => "file.link".into(),
            Command::FileWipe => "file.wipe".into(),
            Command::FileClipNames => "file.clip.names".into(),
            Command::FileClipPaths => "file.clip.paths".into(),
            Command::FileAttributes => "file.attributes".into(),
            Command::PanelToggle(Side::Active) => "panel.toggle".into(),
            Command::PanelToggleOther => "panel.toggle.other".into(),
            Command::PanelToggle(side) => format!("panel.toggle({})", side.as_str()),
            Command::PanelQuickView(Side::Active) => "panel.quickview".into(),
            Command::PanelQuickView(side) => format!("panel.quickview({})", side.as_str()),
            Command::PanelInfo(Side::Active) => "panel.info".into(),
            Command::PanelInfo(side) => format!("panel.info({})", side.as_str()),
            Command::PanelTree(Side::Active) => "panel.tree".into(),
            Command::PanelTree(side) => format!("panel.tree({})", side.as_str()),
            Command::PanelScrollNames(n) if n < 0 => "panel.names.left".into(),
            Command::PanelScrollNames(_) => "panel.names.right".into(),
            Command::FileView => "file.view".into(),
            Command::FileEdit => "file.edit".into(),
            Command::FileEditNew => "file.edit.new".into(),
            Command::FileCopy => "file.copy".into(),
            Command::FileMove => "file.move".into(),
            Command::FileMkdir => "file.mkdir".into(),
            Command::FileDelete => "file.delete".into(),
            Command::FileDirSize => "file.dirsize".into(),
            Command::CmdRun => "cmd.run".into(),
            Command::CmdClear => "cmd.clear".into(),
            Command::CmdHistoryPrev => "cmd.history.prev".into(),
            Command::CmdHistoryNext => "cmd.history.next".into(),
            Command::CmdHistory => "cmd.history".into(),
            Command::CmdFileHistory => "cmd.history.files".into(),
            Command::CmdClip => "cmd.clip".into(),
            Command::CmdComplete => "cmd.complete".into(),
            // The two spellings the keymaps and the macros already hold keep working, and
            // everything else is the systematic form: `cmd.insert.full(left)`.
            Command::CmdInsert(what, Which::Active) => format!("cmd.insert.{}", what.as_str()),
            Command::CmdInsert(what, Which::Passive) => {
                format!("cmd.insert.{}.other", what.as_str())
            }
            Command::CmdInsert(what, which) => {
                format!("cmd.insert.{}({})", what.as_str(), which.as_str())
            }
            Command::FilePack => "file.pack".into(),
            Command::FileUnpack => "file.unpack".into(),
            Command::ArcCommands => "arc.commands".into(),
            Command::ArcTest => "arc.test".into(),
            Command::NetTunnel => "net.tunnel".into(),
            Command::NetTunnels => "net.tunnels".into(),
            Command::ToolUserMenu => "tool.usermenu".into(),
            Command::ToolCompareDirs => "tool.compare.dirs".into(),
            Command::ToolCompare => "tool.compare".into(),
            Command::ToolDuplicates => "tool.duplicates".into(),
            Command::ToolTaskList => "tool.task.list".into(),
            Command::ToolGrab => "tool.grab".into(),
            Command::ToolSyncDirs => "tool.sync.dirs".into(),
            Command::ToolCompareContents => "tool.compare.contents".into(),
            Command::ToolDiff => "tool.diff".into(),
            Command::ToolCalculator => "tool.calculator".into(),
            Command::ToolTerminal => "tool.terminal".into(),
            Command::ToolFind => "tool.find".into(),
            Command::ToolFindFolder => "tool.find.folder".into(),
            Command::ToolReplace => "tool.replace".into(),
            Command::ShellOutputShow => "shell.output.show".into(),
            Command::NetSites => "net.sites".into(),
            Command::NetConnect => "net.connect".into(),
            Command::NetDisconnect => "net.disconnect".into(),
            Command::ViewerQuit => "viewer.quit".into(),
            Command::ViewerWrap => "viewer.wrap".into(),
            Command::ViewerHex => "viewer.hex".into(),
            Command::ViewerMarkdown => "viewer.markdown".into(),
            Command::ViewerTable => "viewer.table".into(),
            Command::ViewerBase64 => "viewer.base64".into(),
            Command::ViewerTableSort => "viewer.table.sort".into(),
            Command::ViewerTableFilter => "viewer.table.filter".into(),
            Command::ViewerModeNext => "viewer.mode.next".into(),
            Command::ViewerSearch => "viewer.search".into(),
            Command::ViewerSearchNext => "viewer.search.next".into(),
            Command::ViewerSearchPrev => "viewer.search.prev".into(),
            Command::ViewerGoto => "viewer.goto".into(),
            Command::ViewerCodepageNext => "viewer.codepage.next".into(),
            Command::ViewerCodepageList => "viewer.codepage.list".into(),
            Command::ViewerFollow => "viewer.follow".into(),
            Command::ViewerLeft => "viewer.left".into(),
            Command::ViewerRight => "viewer.right".into(),
            Command::ViewerUndoPosition => "viewer.undo".into(),
            Command::ViewerShiftBack => "viewer.shift.back".into(),
            Command::ViewerShiftForward => "viewer.shift.forward".into(),
            Command::ViewerBytesLess => "viewer.bytes.less".into(),
            Command::ViewerBytesMore => "viewer.bytes.more".into(),
            Command::ViewerFollowBranch => "viewer.follow.branch".into(),
            Command::ViewerBack => "viewer.back".into(),
            Command::ViewerStructure => "viewer.structure".into(),
            Command::ViewerChromeKeyBar => "viewer.chrome.keybar".into(),
            Command::ViewerChromeStatus => "viewer.chrome.status".into(),
            Command::ViewerChromeScrollbar => "viewer.chrome.scrollbar".into(),
            Command::ViewerPaneNext => "viewer.pane.next".into(),
            Command::ViewerDisasm => "viewer.disasm".into(),
            Command::ViewerToEditor => "viewer.to.editor".into(),
            Command::ViewerNextFile => "viewer.file.next".into(),
            Command::ViewerPrevFile => "viewer.file.prev".into(),
            Command::ViewerToPanel => "viewer.to.panel".into(),
            Command::ViewerDecompile => "viewer.decompile".into(),
            Command::ViewerFilter => "viewer.filter".into(),
            Command::EditorQuit => "editor.quit".into(),
            Command::EditorSave => "editor.save".into(),
            Command::EditorSaveAs => "editor.save.as".into(),
            Command::EditorSaveQuit => "editor.save.quit".into(),
            Command::EditorReload => "editor.reload".into(),
            Command::EditorUndo => "editor.undo".into(),
            Command::EditorRedo => "editor.redo".into(),
            Command::EditorSearch => "editor.search".into(),
            Command::EditorSearchNext => "editor.search.next".into(),
            Command::EditorSearchPrev => "editor.search.prev".into(),
            Command::EditorReplace => "editor.replace".into(),
            Command::EditorGoto => "editor.goto".into(),
            Command::EditorSelectAll => "editor.select.all".into(),
            Command::EditorToViewer => "editor.to.viewer".into(),
            Command::EditorHexToggle => "editor.hex.toggle".into(),
            Command::EditorBlockMark => "editor.block.mark".into(),
            Command::EditorBlockOps => "editor.block.ops".into(),
            Command::EditorAssemble => "editor.assemble".into(),
            Command::PluginCommands => "plugin.commands".into(),
            Command::MacroRecord => "macro.record".into(),
            Command::MacroList => "macro.list".into(),
            Command::AppScreens => "app.screens".into(),
            Command::AppScreenNext => "app.screen.next".into(),
            Command::AppScreenPrev => "app.screen.prev".into(),
            Command::EditorCopy => "editor.copy".into(),
            Command::EditorCut => "editor.cut".into(),
            Command::EditorPaste => "editor.paste".into(),
            Command::EditorDeleteLine => "editor.line.delete".into(),
            Command::EditorDuplicateLine => "editor.line.duplicate".into(),
            Command::EditorMoveLineUp => "editor.line.up".into(),
            Command::EditorMoveLineDown => "editor.line.down".into(),
            Command::EditorIndent => "editor.indent".into(),
            Command::EditorShiftRight => "editor.shift.right".into(),
            Command::EditorAppend => "editor.block.append".into(),
            Command::EditorLiteral => "editor.literal".into(),
            Command::EditorScreenTop => "editor.screen.top".into(),
            Command::EditorScreenBottom => "editor.screen.bottom".into(),
            Command::EditorOutdent => "editor.outdent".into(),
            Command::EditorDeleteToEnd => "editor.delete.to.end".into(),
            Command::EditorDeleteWordLeft => "editor.delete.word.left".into(),
            Command::EditorDeleteWordRight => "editor.delete.word.right".into(),
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
            ("app.font", Some(a)) => Command::AppFont(a.trim().parse().ok()?),
            ("app.themes", _) => Command::AppThemes,
            ("app.languages", _) => Command::AppLanguages,
            ("app.setup.save", _) => Command::AppSetupSave,
            ("opt.system", _) => Command::OptSystem,
            ("opt.panel", _) => Command::OptPanel,
            ("opt.interface", _) => Command::OptInterface,
            ("opt.highlight", _) => Command::OptHighlight,
            ("opt.sort.groups", _) => Command::OptSortGroups,
            ("opt.mask.groups", _) => Command::OptMaskGroups,
            ("opt.codepages", _) => Command::OptCodepages,
            ("opt.settings", _) => Command::OptSettings,
            ("opt.confirmations", _) => Command::OptConfirmations,
            ("opt.colors", _) => Command::OptColors,
            ("opt.editor", _) => Command::OptEditor,
            ("opt.viewer", _) => Command::OptViewer,
            ("panel.sort.menu", _) => Command::PanelSortMenu,
            ("panel.drive", a) => Command::PanelDrive(Side::parse(a.unwrap_or(""))?),
            ("panel.history", _) => Command::PanelHistory,
            ("panel.focus.toggle", _) => Command::PanelFocusToggle,
            ("panel.swap", _) => Command::PanelSwap,
            ("panel.reread", _) => Command::PanelReread,
            ("panel.hidden.toggle", _) => Command::PanelHiddenToggle,
            ("panel.tab.new", _) => Command::PanelTabNew,
            ("panel.tab.close", _) => Command::PanelTabClose,
            ("panel.tab.next", _) => Command::PanelTabNext,
            ("panel.tab.prev", _) => Command::PanelTabPrev,
            ("panel.tab.move", _) => Command::PanelTabMove,
            ("panel.tab.lock", _) => Command::PanelTabLock,
            ("panel.tab.name", _) => Command::PanelTabName,
            ("panel.view.set", Some(a)) => Command::PanelViewSet(ViewMode::parse(a)?),
            ("panel.sort.by", Some(a)) => Command::PanelSortBy(parse_sort_key(a)?),
            ("panel.sort.reverse", _) => Command::PanelSortReverse,
            ("panel.sort.groups", _) => Command::PanelSortGroups,
            ("panel.split", Some(a)) => Command::PanelSplit(a.trim().parse().ok()?),
            ("panel.height", Some(a)) => Command::PanelHeight(a.trim().parse().ok()?),
            ("panel.dirs.first", _) => Command::PanelDirsFirst,
            ("panel.selected.first", _) => Command::PanelSelectedFirst,
            ("panel.filter.quick", _) => Command::PanelFilterQuick,
            ("panel.other.sync", _) => Command::PanelOtherSync,
            ("nav.up", _) => Command::NavUp,
            ("nav.down", _) => Command::NavDown,
            ("nav.pageup", _) => Command::NavPageUp,
            ("nav.pagedown", _) => Command::NavPageDown,
            ("nav.top", _) => Command::NavTop,
            ("nav.bottom", _) => Command::NavBottom,
            ("nav.enter", _) => Command::NavEnter,
            ("nav.left", _) => Command::NavLeft,
            ("nav.right", _) => Command::NavRight,
            ("nav.parent", _) => Command::NavParent,
            ("nav.root", _) => Command::NavRoot,
            ("nav.history.back", _) => Command::NavBack,
            ("nav.history.forward", _) => Command::NavForward,
            ("nav.goto.path", _) => Command::NavGotoPath,
            ("nav.bookmark.set", Some(a)) => Command::NavBookmarkSet(parse_digit(a)?),
            ("nav.bookmark.go", Some(a)) => Command::NavBookmarkGo(parse_digit(a)?),
            ("nav.bookmarks", _) => Command::NavBookmarks,
            ("sel.toggle", _) => Command::SelToggle,
            ("sel.all", _) => Command::SelAll,
            ("sel.none", _) => Command::SelNone,
            ("sel.invert", _) => Command::SelInvert,
            ("sel.mask.add", _) => Command::SelMaskAdd,
            ("sel.mask.remove", _) => Command::SelMaskRemove,
            ("sel.restore", _) => Command::SelRestore,
            ("sel.ext.add", _) => Command::SelExtAdd,
            ("sel.ext.remove", _) => Command::SelExtRemove,
            ("sel.name.add", _) => Command::SelNameAdd,
            ("sel.name.remove", _) => Command::SelNameRemove,
            ("sel.invert.all", _) => Command::SelInvertAll,
            ("file.copy.current", _) => Command::FileCopyCurrent,
            ("file.move.current", _) => Command::FileMoveCurrent,
            ("file.delete.current", _) => Command::FileDeleteCurrent,
            ("file.delete.permanent", _) => Command::FileDeleteForce,
            ("file.assoc", _) => Command::FileAssoc,
            ("file.view.external", _) => Command::FileViewAlt,
            ("file.edit.external", _) => Command::FileEditAlt,
            ("file.describe", _) => Command::FileDescribe,
            ("panel.temp.add", _) => Command::PanelTempAdd,
            ("panel.panelize", _) => Command::PanelPanelize,
            ("file.checksum.create", _) => Command::FileChecksum,
            ("file.checksum.verify", _) => Command::FileChecksumVerify,
            ("file.apply", _) => Command::FileApply,
            ("file.link", _) => Command::FileLink,
            ("file.wipe", _) => Command::FileWipe,
            ("file.clip.names", _) => Command::FileClipNames,
            ("file.clip.paths", _) => Command::FileClipPaths,
            ("file.attributes", _) => Command::FileAttributes,
            ("panel.toggle.other", _) => Command::PanelToggleOther,
            ("panel.toggle", a) => Command::PanelToggle(Side::parse(a.unwrap_or(""))?),
            ("panel.quickview", a) => Command::PanelQuickView(Side::parse(a.unwrap_or(""))?),
            ("panel.info", a) => Command::PanelInfo(Side::parse(a.unwrap_or(""))?),
            ("panel.tree", a) => Command::PanelTree(Side::parse(a.unwrap_or(""))?),
            ("panel.names.left", _) => Command::PanelScrollNames(-1),
            ("panel.names.right", _) => Command::PanelScrollNames(1),
            ("file.view", _) => Command::FileView,
            ("file.edit", _) => Command::FileEdit,
            ("file.edit.new", _) => Command::FileEditNew,
            ("file.copy", _) => Command::FileCopy,
            ("file.move", _) => Command::FileMove,
            ("file.mkdir", _) => Command::FileMkdir,
            ("file.delete", _) => Command::FileDelete,
            ("file.dirsize", _) => Command::FileDirSize,
            ("cmd.run", _) => Command::CmdRun,
            ("cmd.clear", _) => Command::CmdClear,
            ("cmd.history.prev", _) => Command::CmdHistoryPrev,
            ("cmd.history.next", _) => Command::CmdHistoryNext,
            ("cmd.history", _) => Command::CmdHistory,
            ("cmd.history.files", _) => Command::CmdFileHistory,
            ("cmd.clip", _) => Command::CmdClip,
            ("cmd.complete", _) => Command::CmdComplete,
            // `cmd.insert.<what>`, with the panel either in brackets or spelled `.other` —
            // the two the keymaps were written with before there was a table.
            (name, arg) if name.starts_with("cmd.insert.") => {
                let rest = &name["cmd.insert.".len()..];
                let (what, which) = match rest.strip_suffix(".other") {
                    Some(what) => (what, Which::Passive),
                    None => (rest, Which::parse(arg.unwrap_or(""))?),
                };
                Command::CmdInsert(Insert::parse(what)?, which)
            }
            ("tool.compare.dirs", _) => Command::ToolCompareDirs,
            ("tool.diff", _) => Command::ToolDiff,
            ("tool.compare", _) => Command::ToolCompare,
            ("tool.duplicates", _) => Command::ToolDuplicates,
            ("tool.task.list", _) => Command::ToolTaskList,
            ("tool.grab", _) => Command::ToolGrab,
            ("tool.sync.dirs", _) => Command::ToolSyncDirs,
            ("tool.compare.contents", _) => Command::ToolCompareContents,
            ("tool.calculator", _) => Command::ToolCalculator,
            ("tool.terminal", _) => Command::ToolTerminal,
            ("file.pack", _) => Command::FilePack,
            ("file.unpack", _) => Command::FileUnpack,
            ("arc.commands", _) => Command::ArcCommands,
            ("arc.test", _) => Command::ArcTest,
            ("net.tunnel", _) => Command::NetTunnel,
            ("net.tunnels", _) => Command::NetTunnels,
            ("tool.usermenu", _) => Command::ToolUserMenu,
            ("tool.find", _) => Command::ToolFind,
            ("tool.find.folder", _) => Command::ToolFindFolder,
            ("tool.replace", _) => Command::ToolReplace,
            ("shell.output.show", _) => Command::ShellOutputShow,
            ("net.sites", _) => Command::NetSites,
            ("net.connect", _) => Command::NetConnect,
            ("net.disconnect", _) => Command::NetDisconnect,
            ("viewer.quit", _) => Command::ViewerQuit,
            ("viewer.wrap", _) => Command::ViewerWrap,
            ("viewer.hex", _) => Command::ViewerHex,
            ("viewer.markdown", _) => Command::ViewerMarkdown,
            ("viewer.table", _) => Command::ViewerTable,
            ("viewer.base64", _) => Command::ViewerBase64,
            ("viewer.table.sort", _) => Command::ViewerTableSort,
            ("viewer.table.filter", _) => Command::ViewerTableFilter,
            ("viewer.mode.next", _) => Command::ViewerModeNext,
            ("viewer.search", _) => Command::ViewerSearch,
            ("viewer.search.next", _) => Command::ViewerSearchNext,
            ("viewer.search.prev", _) => Command::ViewerSearchPrev,
            ("viewer.goto", _) => Command::ViewerGoto,
            ("viewer.codepage.next", _) => Command::ViewerCodepageNext,
            ("viewer.codepage.list", _) => Command::ViewerCodepageList,
            ("viewer.follow", _) => Command::ViewerFollow,
            ("viewer.left", _) => Command::ViewerLeft,
            ("viewer.right", _) => Command::ViewerRight,
            ("viewer.undo", _) => Command::ViewerUndoPosition,
            ("viewer.shift.back", _) => Command::ViewerShiftBack,
            ("viewer.shift.forward", _) => Command::ViewerShiftForward,
            ("viewer.bytes.less", _) => Command::ViewerBytesLess,
            ("viewer.bytes.more", _) => Command::ViewerBytesMore,
            ("viewer.follow.branch", _) => Command::ViewerFollowBranch,
            ("viewer.back", _) => Command::ViewerBack,
            ("viewer.structure", _) => Command::ViewerStructure,
            ("viewer.chrome.keybar", _) => Command::ViewerChromeKeyBar,
            ("viewer.chrome.status", _) => Command::ViewerChromeStatus,
            ("viewer.chrome.scrollbar", _) => Command::ViewerChromeScrollbar,
            ("viewer.pane.next", _) => Command::ViewerPaneNext,
            ("viewer.disasm", _) => Command::ViewerDisasm,
            ("viewer.to.editor", _) => Command::ViewerToEditor,
            ("viewer.file.next", _) => Command::ViewerNextFile,
            ("viewer.file.prev", _) => Command::ViewerPrevFile,
            ("viewer.to.panel", _) => Command::ViewerToPanel,
            ("viewer.decompile", _) => Command::ViewerDecompile,
            ("viewer.filter", _) => Command::ViewerFilter,
            ("editor.quit", _) => Command::EditorQuit,
            ("editor.save", _) => Command::EditorSave,
            ("editor.save.as", _) => Command::EditorSaveAs,
            ("editor.save.quit", _) => Command::EditorSaveQuit,
            ("editor.reload", _) => Command::EditorReload,
            ("editor.undo", _) => Command::EditorUndo,
            ("editor.redo", _) => Command::EditorRedo,
            ("editor.search", _) => Command::EditorSearch,
            ("editor.search.next", _) => Command::EditorSearchNext,
            ("editor.search.prev", _) => Command::EditorSearchPrev,
            ("editor.replace", _) => Command::EditorReplace,
            ("editor.goto", _) => Command::EditorGoto,
            ("editor.select.all", _) => Command::EditorSelectAll,
            ("editor.to.viewer", _) => Command::EditorToViewer,
            ("editor.hex.toggle", _) => Command::EditorHexToggle,
            ("editor.block.mark", _) => Command::EditorBlockMark,
            ("editor.block.ops", _) => Command::EditorBlockOps,
            ("editor.assemble", _) => Command::EditorAssemble,
            ("plugin.commands", _) => Command::PluginCommands,
            ("macro.record", _) => Command::MacroRecord,
            ("macro.list", _) => Command::MacroList,
            ("app.screens", _) => Command::AppScreens,
            ("app.screen.next", _) => Command::AppScreenNext,
            ("app.screen.prev", _) => Command::AppScreenPrev,
            ("editor.copy", _) => Command::EditorCopy,
            ("editor.cut", _) => Command::EditorCut,
            ("editor.paste", _) => Command::EditorPaste,
            ("editor.line.delete", _) => Command::EditorDeleteLine,
            ("editor.line.duplicate", _) => Command::EditorDuplicateLine,
            ("editor.line.up", _) => Command::EditorMoveLineUp,
            ("editor.line.down", _) => Command::EditorMoveLineDown,
            ("editor.indent", _) => Command::EditorIndent,
            ("editor.shift.right", _) => Command::EditorShiftRight,
            ("editor.block.append", _) => Command::EditorAppend,
            ("editor.literal", _) => Command::EditorLiteral,
            ("editor.screen.top", _) => Command::EditorScreenTop,
            ("editor.screen.bottom", _) => Command::EditorScreenBottom,
            ("editor.outdent", _) => Command::EditorOutdent,
            ("editor.delete.to.end", _) => Command::EditorDeleteToEnd,
            ("editor.delete.word.left", _) => Command::EditorDeleteWordLeft,
            ("editor.delete.word.right", _) => Command::EditorDeleteWordRight,
            ("editor.readonly.toggle", _) => Command::EditorToggleReadOnly,
            _ => return None,
        };
        Some(cmd)
    }

    /// Message id for the localised label (`fcmd-i18n`): `cmd.app.quit`.
    pub fn label_key(self) -> String {
        let id = self.id();
        let base = id.split('(').next().unwrap_or(&id);
        format!("cmd.{base}")
    }
}

/// One digit, `0` to `9`: a folder shortcut's number. Anything else is not a shortcut, and a
/// keymap that names one is wrong rather than clamped.
fn parse_digit(s: &str) -> Option<u8> {
    match s.trim().parse::<u8>() {
        Ok(n) if n <= 9 => Some(n),
        _ => None,
    }
}

fn parse_sort_key(s: &str) -> Option<SortKey> {
    match s.trim().to_ascii_lowercase().as_str() {
        "name" => Some(SortKey::Name),
        "ext" | "extension" => Some(SortKey::Ext),
        "size" => Some(SortKey::Size),
        "mtime" | "time" | "date" => Some(SortKey::MTime),
        "btime" | "created" | "creation" => Some(SortKey::BTime),
        "atime" | "accessed" | "access" => Some(SortKey::ATime),
        "descr" | "description" | "diz" => Some(SortKey::Descr),
        "owner" => Some(SortKey::Owner),
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
            Command::ViewerFollowBranch,
            Command::ViewerBack,
            Command::ViewerStructure,
            Command::ViewerPaneNext,
            Command::ViewerDisasm,
            Command::ViewerDecompile,
            Command::ViewerFilter,
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
