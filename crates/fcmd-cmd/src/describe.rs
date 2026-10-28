//! One sentence per command, for the help.
//!
//! `F1` shows the keys of wherever you are, and a list of keys with command identifiers next
//! to them tells a reader who already knows the program what they already knew. This is the
//! other column: what the command actually does, in words.
//!
//! The match is exhaustive on purpose. A command added without a sentence does not compile,
//! which is the only way a table like this stays complete.
//!
//! It is English, like the rest of the help (`docs/help/`), and unlike the interface, which
//! is translated: a manual is a different thing from a label, and a half-translated manual
//! reads worse than an English one.

use fcmd_core::{SortKey, ViewMode};

use crate::command::{Command, Side};

impl Command {
    /// What this command does, in a few words.
    pub fn describe(self) -> &'static str {
        match self {
            Command::AppQuit => "leave the program",
            Command::AppHelp => "the help for where you are",
            Command::AppMenu => "the menu bar",
            Command::AppPalette => "every command, searchable",
            Command::AppThemeNext => "the next colour theme",
            Command::AppFont(by) => match by {
                0 => "ask the terminal for the font it started with",
                by if by < 0 => "ask the terminal for a smaller font",
                _ => "ask the terminal for a larger font",
            },
            Command::AppThemes => "choose a colour theme",
            Command::AppLanguages => "choose the language",
            Command::AppSetupSave => "write the settings to fcmd.toml",
            Command::OptSystem => "the system settings",
            Command::OptPanel => "the panel settings",
            Command::OptInterface => "the interface settings",
            Command::OptHighlight => "the colour groups, as a list",
            Command::OptConfirmations => "which questions are asked",
            Command::OptColors => "edit the colours of the theme in use",
            Command::OptEditor => "how the editor behaves",
            Command::OptViewer => "how the viewer behaves",

            Command::PanelFocusToggle => "move between the two panels",
            Command::PanelSwap => "swap the panels over",
            Command::PanelReread => "read this directory again",
            Command::PanelHiddenToggle => "show or hide the hidden files",
            Command::NavBookmarkSet(_) => "put this directory on a folder shortcut",
            Command::NavBookmarkGo(_) => "go to a folder shortcut",
            Command::NavBookmarks => "the folders kept to hand",
            Command::PanelTabNew => "another tab on this side",
            Command::PanelTabClose => "close this tab",
            Command::PanelTabNext => "the next tab",
            Command::PanelTabPrev => "the previous tab",
            Command::PanelTabMove => "hand this tab to the other side",
            Command::PanelTabLock => "keep this tab on its directory, or let it move again",
            Command::PanelTabName => "give this tab a name",
            Command::PanelViewSet(mode) => match mode {
                ViewMode::Brief => "names only, three columns",
                ViewMode::Medium => "names only, two columns",
                ViewMode::Full => "name, size, date and time",
                ViewMode::Wide => "name and size, two columns",
                ViewMode::Detail => "name, size, time and attributes",
                ViewMode::Owners => "name, owner, group and size",
            },
            Command::PanelQuickView(_) => "turn the other panel into a preview of the file",
            Command::PanelInfo(_) => "turn the other panel into an information panel",
            Command::PanelTree(_) => "turn the other panel into a directory tree",
            Command::PanelSortBy(key) => match key {
                SortKey::Name => "sort by name",
                SortKey::Ext => "sort by extension",
                SortKey::Size => "sort by size",
                SortKey::MTime => "sort by when it was last changed",
                SortKey::BTime => "sort by when it was created",
                SortKey::ATime => "sort by when it was last read",
                SortKey::Unsorted => "leave the order as the filesystem gives it",
            },
            Command::PanelSortReverse => "turn the order round",
            Command::PanelSplit(by) => {
                if by < 0 {
                    "give the left panel less of the width"
                } else {
                    "give the left panel more of the width"
                }
            }
            Command::PanelHeight(by) => {
                if by < 0 {
                    "take a row off the panels for the command output"
                } else {
                    "give the panels a row back from the command output"
                }
            }
            Command::PanelDirsFirst => "directories first, or mixed in with the files",
            Command::PanelSelectedFirst => "the selected files first, or in their own place",
            Command::PanelFilterQuick => "show only the names that match a mask",
            Command::PanelOtherSync => "send the other panel to this directory",
            Command::PanelSortMenu => "choose the order from a list",
            Command::PanelDrive(_) => "the drives, mount points and open connections",
            Command::PanelHistory => "where this panel has been",
            Command::PanelToggle(side) => match side {
                Side::Left => "hide the left panel, or bring it back",
                Side::Right => "hide the right panel, or bring it back",
                Side::Active => "hide this panel, or bring it back",
            },
            Command::PanelToggleOther => "hide the other panel, or bring it back",
            Command::PanelScrollNames(_) => "scroll the name column sideways",

            Command::NavUp => "up one row",
            Command::NavDown => "down one row",
            Command::NavPageUp => "up a screenful",
            Command::NavPageDown => "down a screenful",
            Command::NavTop => "to the first row",
            Command::NavBottom => "to the last row",
            Command::NavEnter => "enter the directory, or run the file",
            Command::NavLeft => "left one column",
            Command::NavRight => "right one column",
            Command::NavParent => "up to the directory above",
            Command::NavRoot => "to the root of this filesystem",
            Command::NavBack => "back to where you were",
            Command::NavForward => "forward again",
            Command::NavGotoPath => "type a path to go to",

            Command::SelToggle => "select this row, and move on",
            Command::SelAll => "select everything",
            Command::SelNone => "select nothing",
            Command::SelInvert => "select what is not selected, and vice versa",
            Command::SelMaskAdd => "select by a file mask",
            Command::SelMaskRemove => "unselect by a file mask",
            Command::SelRestore => "put back the selection that was just cleared",
            Command::SelExtAdd => "select everything with this extension",
            Command::SelExtRemove => "unselect everything with this extension",
            Command::SelNameAdd => "select everything with this name",
            Command::SelNameRemove => "unselect everything with this name",
            Command::SelInvertAll => "invert the selection, directories included",

            Command::FileView => "look at the file",
            Command::FileEdit => "edit the file",
            Command::FileEditNew => "edit a new file",
            Command::FileCopy => "copy what is selected to the other panel",
            Command::FileMove => "move or rename what is selected",
            Command::FileMkdir => "make a directory",
            Command::FileDelete => "delete what is selected",
            Command::FileDirSize => "measure the directories",
            Command::FileCopyCurrent => "copy this one file, under a name you give",
            Command::FileMoveCurrent => "rename this one file",
            Command::FileDeleteCurrent => "delete this one file",
            Command::FileDeleteForce => "delete outright, past the wastebasket",
            Command::FileAssoc => "what could open this file",
            Command::FileViewAlt => "read this file with something that is not us",
            Command::FileEditAlt => "write this file with something that is not us",
            Command::FileChecksum => "write a list of digests for what is selected",
            Command::FileChecksumVerify => "check the files beside a list of digests",
            Command::FileApply => "run a command over each selected file",
            Command::FileLink => "make a link",
            Command::FileWipe => "delete, overwriting the bytes first",
            Command::FileClipNames => "copy the names to the clipboard",
            Command::FileClipPaths => "copy the full paths to the clipboard",
            Command::FileAttributes => "the permissions and the times of what is selected",

            Command::CmdRun => "run what is typed on the command line",
            Command::CmdClear => "empty the command line",
            Command::CmdHistoryPrev => "the command before this one",
            Command::CmdHistoryNext => "the command after this one",
            Command::CmdHistory => "the commands that have run",
            Command::CmdFileHistory => "the files that have been read or edited, as a list",
            Command::CmdClip => "copy the command line to the clipboard",
            Command::CmdInsertName => "put the name under the cursor on the command line",
            Command::CmdInsertNameOther => "the same, from the other panel",
            Command::CmdInsertPath => "put this panel's path on the command line",
            Command::CmdInsertPathOther => "the same, from the other panel",

            Command::FilePack => "put what is selected into a new archive",
            Command::FileUnpack => "take an archive apart",
            Command::ArcCommands => "what can be done with this archive",
            Command::ArcTest => "read every entry and say what does not come out whole",

            Command::NetTunnel => "open a tunnel over the connection this panel is on",
            Command::NetTunnels => "the tunnels that are open",
            Command::ToolUserMenu => "your own menu of commands",
            Command::ToolCompareDirs => "compare the two panels by name, size and time",
            Command::ToolSyncDirs => "make the two directories hold the same thing",
            Command::ToolCompareContents => "compare them by what is in the files",
            Command::ToolCalculator => "a calculator, in every base at once",
            Command::ToolTerminal => "a real terminal, with a shell in it",
            Command::ToolFind => "find files, by name or by what is in them",
            Command::ToolReplace => "replace across the selected files",
            Command::ShellOutputShow => "the panels off, and the screen the commands printed on",
            Command::NetSites => "the stored connections",
            Command::NetConnect => "connect to a server",
            Command::NetDisconnect => "close this connection",

            Command::ViewerQuit => "close the viewer",
            Command::ViewerWrap => "wrap the long lines, or leave them long",
            Command::ViewerHex => "between the text and the bytes",
            Command::ViewerMarkdown => "between Markdown laid out to be read and its source",
            Command::ViewerTable => "between a tabular file as a table and its source",
            Command::ViewerModeNext => "the next way of reading this file",
            Command::ViewerSearch => "search: what, as what, and which way",
            Command::ViewerSearchNext => "the next hit",
            Command::ViewerSearchPrev => "the hit before",
            Command::ViewerGoto => "go to an offset or a percentage",
            Command::ViewerCodepageNext => "read the bytes as another encoding",
            Command::ViewerFollow => "follow the file as it grows",
            Command::ViewerLeft => "left, or eight bytes narrower in hex",
            Command::ViewerRight => "right, or eight bytes wider in hex",
            Command::ViewerUndoPosition => "back to where you were reading",
            Command::ViewerShiftBack => "shift the bytes one back",
            Command::ViewerShiftForward => "shift the bytes one on",
            Command::ViewerBytesLess => "fewer bytes to the row",
            Command::ViewerBytesMore => "more bytes to the row",
            Command::ViewerFollowBranch => "follow the jump under the cursor",
            Command::ViewerBack => "back from the jump you followed",
            Command::ViewerStructure => "the file's own structure, beside it",
            Command::ViewerChromeKeyBar => "take the key bar away, or put it back",
            Command::ViewerChromeStatus => "take the top line away, or put it back",
            Command::ViewerChromeScrollbar => "the scrollbar, on or off",
            Command::ViewerPaneNext => "move between the file and its structure",
            Command::ViewerDisasm => "read the bytes as instructions",
            Command::ViewerToEditor => "open the same place in the editor",
            Command::ViewerNextFile => "the next file of the panel",
            Command::ViewerPrevFile => "the file before",
            Command::ViewerToPanel => "make the panel catch up with what you are reading",
            Command::ViewerDecompile => "decompile, where a tool for it exists",

            Command::EditorQuit => "close the editor",
            Command::EditorSave => "save",
            Command::EditorSaveAs => "save under another name",
            Command::EditorSaveQuit => "save and leave",
            Command::EditorReload => "read the file again, losing the changes",
            Command::EditorUndo => "undo",
            Command::EditorRedo => "redo",
            Command::EditorSearch => "search: what, as what, and which way",
            Command::EditorSearchNext => "the next hit",
            Command::EditorSearchPrev => "the hit before",
            Command::EditorReplace => "replace",
            Command::EditorGoto => "go to a line",
            Command::EditorSelectAll => "mark the whole file",
            Command::EditorToViewer => "open the same place in the viewer",
            Command::EditorHexToggle => "between the text and the bytes",
            Command::EditorBlockMark => "start or end a marked block",
            Command::EditorBlockOps => "what can be done with the marked bytes",
            Command::EditorAssemble => "assemble an instruction over the bytes",
            Command::EditorCopy => "copy the block",
            Command::EditorCut => "cut the block",
            Command::EditorPaste => "paste",
            Command::EditorDeleteLine => "delete this line",
            Command::EditorDuplicateLine => "duplicate this line",
            Command::EditorMoveLineUp => "move this line up",
            Command::EditorMoveLineDown => "move this line down",
            Command::EditorIndent => "indent the block",
            Command::EditorShiftRight => "move the block or the line one indent right",
            Command::EditorAppend => "add the block to the clipboard",
            Command::EditorLiteral => "insert the next key as itself",
            Command::EditorScreenTop => "the cursor to the first line on the screen",
            Command::EditorScreenBottom => "the cursor to the last line on the screen",
            Command::EditorOutdent => "take one level of indent off the block",
            Command::EditorDeleteToEnd => "delete to the end of the line",
            Command::EditorDeleteWordLeft => "delete the word before the cursor",
            Command::EditorDeleteWordRight => "delete the word after the cursor",
            Command::EditorToggleReadOnly => "allow changes, or refuse them",

            Command::PluginCommands => "the commands the plugins provide",
            Command::MacroRecord => "start or stop recording a macro",
            Command::MacroList => "the macros that are recorded",
            Command::AppScreens => "everything that is open",
            Command::AppScreenNext => "the next open screen",
            Command::AppScreenPrev => "the screen before",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every command has to say something, and nothing may say it twice in a way that reads
    /// like a copy-and-paste left unfinished.
    #[test]
    fn every_command_says_what_it_does() {
        for cmd in Command::all() {
            let text = cmd.describe();
            assert!(!text.is_empty(), "{} says nothing", cmd.id());
            assert!(
                text.chars().next().is_some_and(|c| c.is_lowercase()),
                "{}: `{text}` should read as the end of a sentence, in lower case",
                cmd.id()
            );
            assert!(
                !text.ends_with('.'),
                "{}: `{text}` is a clause, not a sentence",
                cmd.id()
            );
        }
    }

    #[test]
    fn a_command_with_an_argument_describes_that_argument() {
        assert_eq!(
            Command::PanelSortBy(SortKey::Size).describe(),
            "sort by size"
        );
        assert_ne!(
            Command::PanelToggle(Side::Left).describe(),
            Command::PanelToggle(Side::Right).describe()
        );
        assert_ne!(
            Command::PanelSplit(-1).describe(),
            Command::PanelSplit(1).describe()
        );
    }
}
