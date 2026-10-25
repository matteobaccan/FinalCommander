//! The menu bar (`F9`) and what each entry does — see `docs/spec/10-menus-and-dialogs.md`.
//!
//! The five menus follow FAR's layout, because a user who knows FAR must find things where
//! they expect them. Entries whose feature is not here yet are kept, drawn dim, and say so
//! when chosen: a menu is also a map of the program, and a map with holes in it is better
//! than a map that lies.

use scmd_cmd::{Command, Side};
use scmd_core::ViewMode;

/// What choosing an entry does. Everything the menu can do is a command, so a menu entry and
/// a key press go through exactly the same code.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Run a command from the vocabulary.
    Run(Command),
    /// In the menu for completeness; reports "not implemented" with this name.
    Todo(&'static str),
}

/// The settings dialogs (checkboxes and choices).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKind {
    /// A stored connection (SPEC 10 §3.5).
    Site,
    /// Find file (SPEC 06 §1).
    Find,
    /// Replace across the selected files.
    Replace,
    /// Put the selection into a new archive (SPEC 04 §4).
    Pack,
    /// Take an archive apart.
    Unpack,
    /// What a file is, rather than what is in it (SPEC 10 §3.8).
    Attributes,
    /// Copy or move: where to, and under what rules (SPEC 03 §4).
    Copy,
    /// Which hash, and where the list of digests goes (SPEC 03 §9).
    Checksum,
    /// Which way to synchronise, and what counts as the same (SPEC 03 §10).
    Sync,
    /// A link to the file under the cursor: its name, and which kind of link it is.
    Link,
    /// Find text in what the viewer or the editor has open (SPEC 05 §1.4).
    Search,
    System,
    Panel,
    Interface,
    Confirmations,
    /// How the editor behaves (SPEC 10 §3.6).
    EditorSettings,
}

/// The dialogs where one row is chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickKind {
    Languages,
    Themes,
    /// Drives, mount points and open connections.
    Drives,
    /// Where this panel has been.
    FoldersHistory,
    /// The folders kept to hand (SPEC 02 §11).
    Bookmarks,
    /// What could open the file under the cursor (SPEC 06 §5).
    OpenWith,
    /// The command lines that have run.
    CommandHistory,
    /// The sort order for this panel.
    SortModes,
    /// What to do with the block of bytes that is marked (SPEC 05 §2.6).
    HexBlock,
    /// The viewers and editors left open, with the panels at the top (SPEC 02 §7).
    Screens,
    /// The recorded macros (SPEC 08 §5).
    Macros,
    /// The commands the loaded plugins provide (SPEC 08 §1).
    Plugins,
    /// How a just-recorded macro should play (SPEC 08 §5).
    MacroPlay,
    /// The tunnels that are open, and what has gone through them (SPEC 04 §6.2).
    Tunnels,
    /// What can be done with the archive at hand (`Shift+F3`).
    ArcCommands,
    /// The menu the user wrote (`F2`, SPEC 06 §4), one level of it at a time.
    UserMenu,
}

/// A mark (`√`) next to the entry that is currently in force.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    None,
    View(ViewMode),
    Hidden,
    /// This tab stays on its directory.
    TabLocked,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Message id of the label.
    pub label: &'static str,
    /// Shortcut, shown right-aligned, exactly as the keymap spells it.
    pub keys: &'static str,
    pub action: Action,
    pub mark: Mark,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Item(Item),
    Separator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub title: &'static str,
    /// `Some(0)` for the left panel's menu, `Some(1)` for the right one: choosing an entry
    /// acts on that panel, as FAR's Left and Right menus do.
    pub side: Option<usize>,
    pub items: Vec<Entry>,
}

fn item(label: &'static str, keys: &'static str, action: Action) -> Entry {
    Entry::Item(Item {
        label,
        keys,
        action,
        mark: Mark::None,
    })
}

fn marked(label: &'static str, keys: &'static str, action: Action, mark: Mark) -> Entry {
    Entry::Item(Item {
        label,
        keys,
        action,
        mark,
    })
}

/// The panel menus (Left and Right are the same, aimed at different panels).
fn panel_menu(title: &'static str, side: usize) -> Menu {
    let ctrl = if side == 0 { "Ctrl+F1" } else { "Ctrl+F2" };
    Menu {
        title,
        side: Some(side),
        items: vec![
            marked(
                "menu-brief",
                "Ctrl+1",
                Action::Run(Command::PanelViewSet(ViewMode::Brief)),
                Mark::View(ViewMode::Brief),
            ),
            marked(
                "menu-medium",
                "Ctrl+2",
                Action::Run(Command::PanelViewSet(ViewMode::Medium)),
                Mark::View(ViewMode::Medium),
            ),
            marked(
                "menu-full",
                "Ctrl+3",
                Action::Run(Command::PanelViewSet(ViewMode::Full)),
                Mark::View(ViewMode::Full),
            ),
            marked(
                "menu-wide",
                "Ctrl+4",
                Action::Run(Command::PanelViewSet(ViewMode::Wide)),
                Mark::View(ViewMode::Wide),
            ),
            marked(
                "menu-detailed",
                "Ctrl+5",
                Action::Run(Command::PanelViewSet(ViewMode::Detail)),
                Mark::View(ViewMode::Detail),
            ),
            Entry::Separator,
            item(
                "menu-info-panel",
                "Ctrl+L",
                Action::Run(Command::PanelInfo(Side::Active)),
            ),
            item(
                "menu-quick-view",
                "Ctrl+Q",
                Action::Run(Command::PanelQuickView(Side::Active)),
            ),
            item(
                "menu-tree-panel",
                "Ctrl+T",
                Action::Run(Command::PanelTree(Side::Active)),
            ),
            Entry::Separator,
            item(
                "menu-tab-new",
                "Ctrl+Alt+T",
                Action::Run(Command::PanelTabNew),
            ),
            item(
                "menu-tab-close",
                "Ctrl+Alt+W",
                Action::Run(Command::PanelTabClose),
            ),
            item(
                "menu-tab-next",
                "Ctrl+Alt+N",
                Action::Run(Command::PanelTabNext),
            ),
            item(
                "menu-tab-prev",
                "Ctrl+Alt+P",
                Action::Run(Command::PanelTabPrev),
            ),
            item(
                "menu-tab-move",
                "Ctrl+Alt+M",
                Action::Run(Command::PanelTabMove),
            ),
            marked(
                "menu-tab-lock",
                "Ctrl+Alt+K",
                Action::Run(Command::PanelTabLock),
                Mark::TabLocked,
            ),
            item("menu-tab-name", "", Action::Run(Command::PanelTabName)),
            Entry::Separator,
            item(
                "menu-sort-modes",
                "Ctrl+F12",
                Action::Run(Command::PanelSortMenu),
            ),
            marked(
                "menu-hidden",
                "Ctrl+H",
                Action::Run(Command::PanelHiddenToggle),
                Mark::Hidden,
            ),
            item(
                "menu-filter",
                "Alt+I",
                Action::Run(Command::PanelFilterQuick),
            ),
            Entry::Separator,
            item(
                "menu-panel-onoff",
                ctrl,
                Action::Run(Command::PanelToggle(if side == 0 {
                    Side::Left
                } else {
                    Side::Right
                })),
            ),
            item("menu-reread", "Ctrl+R", Action::Run(Command::PanelReread)),
            item(
                "menu-change-drive",
                if side == 0 { "Alt+F1" } else { "Alt+F2" },
                Action::Run(Command::PanelDrive(if side == 0 {
                    Side::Left
                } else {
                    Side::Right
                })),
            ),
        ],
    }
}

/// Every menu, left to right.
pub fn menus() -> Vec<Menu> {
    vec![
        panel_menu("menu-left", 0),
        Menu {
            title: "menu-files",
            side: None,
            items: vec![
                item("menu-view", "F3", Action::Run(Command::FileView)),
                item("menu-edit", "F4", Action::Run(Command::FileEdit)),
                item(
                    "menu-edit-new",
                    "Shift+F4",
                    Action::Run(Command::FileEditNew),
                ),
                item("menu-copy", "F5", Action::Run(Command::FileCopy)),
                item("menu-rename", "F6", Action::Run(Command::FileMove)),
                item("menu-link", "Alt+F6", Action::Run(Command::FileLink)),
                item("menu-mkdir", "F7", Action::Run(Command::FileMkdir)),
                item("menu-delete", "F8", Action::Run(Command::FileDelete)),
                item(
                    "menu-delete-permanent",
                    "Shift+Del",
                    Action::Run(Command::FileDeleteForce),
                ),
                item("menu-wipe", "Alt+Del", Action::Run(Command::FileWipe)),
                Entry::Separator,
                item(
                    "menu-archive-add",
                    "Shift+F1",
                    Action::Run(Command::FilePack),
                ),
                item(
                    "menu-archive-extract",
                    "Shift+F2",
                    Action::Run(Command::FileUnpack),
                ),
                item(
                    "menu-archive-commands",
                    "Shift+F3",
                    Action::Run(Command::ArcCommands),
                ),
                item("menu-archive-test", "", Action::Run(Command::ArcTest)),
                Entry::Separator,
                item("menu-checksum", "", Action::Run(Command::FileChecksum)),
                item(
                    "menu-checksum-verify",
                    "",
                    Action::Run(Command::FileChecksumVerify),
                ),
                Entry::Separator,
                item(
                    "menu-attributes",
                    "Ctrl+A",
                    Action::Run(Command::FileAttributes),
                ),
                item(
                    "menu-apply-command",
                    "Ctrl+G",
                    Action::Run(Command::FileApply),
                ),
                // Ours rather than FAR's: FAR has no replace across files.
                item(
                    "menu-replace-in-files",
                    "Ctrl+Shift+R",
                    Action::Run(Command::ToolReplace),
                ),
                Entry::Separator,
                item("menu-select-group", "+", Action::Run(Command::SelMaskAdd)),
                item(
                    "menu-unselect-group",
                    "-",
                    Action::Run(Command::SelMaskRemove),
                ),
                item(
                    "menu-invert-selection",
                    "*",
                    Action::Run(Command::SelInvert),
                ),
                // FAR's selection matrix by the hardest route: `Ctrl+Shift` with a grey key
                // is the kind of binding a terminal often eats, and the keymap file says the
                // menu reaches the same work — so it has to.
                item(
                    "menu-select-ext",
                    "Ctrl+Shift++",
                    Action::Run(Command::SelExtAdd),
                ),
                item(
                    "menu-unselect-ext",
                    "Ctrl+Shift+-",
                    Action::Run(Command::SelExtRemove),
                ),
                item("menu-select-all", "Ctrl+A", Action::Run(Command::SelAll)),
                item(
                    "menu-restore-selection",
                    "Ctrl+Shift+M",
                    Action::Run(Command::SelRestore),
                ),
                Entry::Separator,
                item("menu-dirsize", "Space", Action::Run(Command::FileDirSize)),
            ],
        },
        Menu {
            title: "menu-commands",
            side: None,
            items: vec![
                item("menu-find-file", "Alt+F7", Action::Run(Command::ToolFind)),
                item("menu-history", "Alt+F8", Action::Run(Command::CmdHistory)),
                item(
                    "menu-view-history",
                    "Alt+F11",
                    Action::Todo("history.files"),
                ),
                item(
                    "menu-folders-history",
                    "Alt+F12",
                    Action::Run(Command::PanelHistory),
                ),
                item(
                    "menu-bookmarks",
                    "Ctrl+D",
                    Action::Run(Command::NavBookmarks),
                ),
                Entry::Separator,
                item(
                    "menu-swap-panels",
                    "Ctrl+U",
                    Action::Run(Command::PanelSwap),
                ),
                item(
                    "menu-sync-panel",
                    "Ctrl+=",
                    Action::Run(Command::PanelOtherSync),
                ),
                item(
                    "menu-panels-onoff",
                    "Ctrl+O",
                    Action::Run(Command::ShellOutputShow),
                ),
                item(
                    "menu-compare-folders",
                    "",
                    Action::Run(Command::ToolCompareDirs),
                ),
                item(
                    "menu-sync-folders",
                    "Ctrl+Shift+Y",
                    Action::Run(Command::ToolSyncDirs),
                ),
                item("menu-tunnel", "", Action::Run(Command::NetTunnel)),
                item("menu-tunnels", "", Action::Run(Command::NetTunnels)),
                item(
                    "menu-compare-contents",
                    "",
                    Action::Run(Command::ToolCompareContents),
                ),
                item(
                    "menu-calculator",
                    "Alt+=",
                    Action::Run(Command::ToolCalculator),
                ),
                item(
                    "menu-terminal",
                    "Ctrl+Alt+E",
                    Action::Run(Command::ToolTerminal),
                ),
                Entry::Separator,
                item("menu-connections", "Alt+N", Action::Run(Command::NetSites)),
                item(
                    "menu-disconnect",
                    "Alt+Shift+N",
                    Action::Run(Command::NetDisconnect),
                ),
                Entry::Separator,
                item("menu-user-menu", "F2", Action::Run(Command::ToolUserMenu)),
                item("menu-associations", "", Action::Run(Command::FileAssoc)),
                item(
                    "menu-folder-shortcuts",
                    "Ctrl+D",
                    Action::Run(Command::NavBookmarks),
                ),
                Entry::Separator,
                item(
                    "menu-plugin-commands",
                    "F11",
                    Action::Run(Command::PluginCommands),
                ),
                item("menu-screens-list", "F12", Action::Run(Command::AppScreens)),
                item(
                    "menu-macro-record",
                    "Ctrl+.",
                    Action::Run(Command::MacroRecord),
                ),
                item(
                    "menu-macro-list",
                    "Ctrl+Shift+.",
                    Action::Run(Command::MacroList),
                ),
                item("menu-task-list", "Ctrl+W", Action::Todo("task.list")),
            ],
        },
        Menu {
            title: "menu-options",
            side: None,
            items: vec![
                item("menu-system-settings", "", Action::Run(Command::OptSystem)),
                item("menu-panel-settings", "", Action::Run(Command::OptPanel)),
                item(
                    "menu-interface-settings",
                    "",
                    Action::Run(Command::OptInterface),
                ),
                item("menu-languages", "", Action::Run(Command::AppLanguages)),
                item("menu-themes", "", Action::Run(Command::AppThemes)),
                item("menu-colors", "", Action::Run(Command::OptColors)),
                Entry::Separator,
                item(
                    "menu-confirmations",
                    "",
                    Action::Run(Command::OptConfirmations),
                ),
                item("menu-viewer-settings", "", Action::Todo("settings.viewer")),
                item("menu-editor-settings", "", Action::Run(Command::OptEditor)),
                item("menu-codepages", "", Action::Todo("settings.codepages")),
                item(
                    "menu-highlighting",
                    "",
                    Action::Todo("settings.highlighting"),
                ),
                item("menu-plugins-config", "", Action::Todo("settings.plugins")),
                Entry::Separator,
                item("menu-help", "F1", Action::Run(Command::AppHelp)),
                item(
                    "menu-save-setup",
                    "Shift+F9",
                    Action::Run(Command::AppSetupSave),
                ),
                item("menu-quit", "F10", Action::Run(Command::AppQuit)),
            ],
        },
        panel_menu("menu-right", 1),
    ]
}

/// Every message id the menu needs, for the test that keeps the catalogues honest.
#[cfg(test)]
pub fn label_ids() -> Vec<&'static str> {
    let mut ids = Vec::new();
    for menu in menus() {
        ids.push(menu.title);
        for entry in &menu.items {
            if let Entry::Item(item) = entry {
                ids.push(item.label);
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bar_has_fars_five_menus_in_fars_order() {
        let menus = menus();
        let titles: Vec<&str> = menus.iter().map(|m| m.title).collect();
        assert_eq!(
            titles,
            [
                "menu-left",
                "menu-files",
                "menu-commands",
                "menu-options",
                "menu-right"
            ]
        );
        assert_eq!(menus[0].side, Some(0));
        assert_eq!(menus[4].side, Some(1));
        assert!(menus[1].side.is_none());
    }

    #[test]
    fn every_command_in_the_menu_exists_in_the_vocabulary() {
        let all = Command::all();
        for menu in menus() {
            for entry in &menu.items {
                if let Entry::Item(Item {
                    action: Action::Run(cmd),
                    label,
                    ..
                }) = entry
                {
                    assert!(
                        all.contains(cmd),
                        "{label} runs {cmd}, which does not exist"
                    );
                }
            }
        }
    }

    #[test]
    fn every_label_is_translated_in_every_language() {
        for (tag, _) in scmd_i18n::LANGUAGES {
            let i18n = scmd_i18n::I18n::new(tag);
            for id in label_ids() {
                assert_ne!(
                    i18n.t(id),
                    id,
                    "{tag}: `{id}` is missing from the catalogue"
                );
            }
        }
    }

    #[test]
    fn no_menu_is_empty_and_none_starts_or_ends_with_a_separator() {
        for menu in menus() {
            assert!(!menu.items.is_empty(), "{} is empty", menu.title);
            assert!(
                !matches!(menu.items.first(), Some(Entry::Separator)),
                "{} starts with a separator",
                menu.title
            );
            assert!(
                !matches!(menu.items.last(), Some(Entry::Separator)),
                "{} ends with a separator",
                menu.title
            );
        }
    }
}
