use std::path::PathBuf;

use gpui::{Context, Entity, Window};

use crate::ui::popup_menu::{PopupMenu, PopupMenuItem};

use crate::explorer::Explorer;
use crate::explorer::columns::Column;
use crate::theme::icon_theme;

pub fn build(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let target = explorer.update(cx, |explorer, _cx| explorer.context_menu_target.take());

    match target {
        Some(path) => file_row_menu(explorer, path, menu, window, cx),
        None => empty_space_menu(explorer, menu, window, cx),
    }
}

pub fn build_trash(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let target = explorer.update(cx, |explorer, _cx| explorer.context_menu_target.take());

    match target {
        Some(id_path) => trash_row_menu(explorer, id_path, menu, window, cx),
        None => trash_empty_space_menu(explorer, menu, window, cx),
    }
}

fn trash_row_menu(
    explorer: Entity<Explorer>,
    _id_path: PathBuf,
    menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let restore_explorer = explorer.clone();
    let purge_explorer = explorer.clone();

    menu.item(
        PopupMenuItem::new("Restore").on_click(move |_, _window, cx| {
            restore_explorer.update(cx, |explorer, cx| explorer.restore_selection(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Delete Permanently").on_click(move |_, _window, cx| {
            purge_explorer.update(cx, |explorer, cx| explorer.purge_selection(cx));
        }),
    )
}

fn trash_empty_space_menu(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let show_hidden = explorer.read(cx).show_hidden;
    let undo_label = explorer.read(cx).undo_label();
    let redo_label = explorer.read(cx).redo_label();

    let toggle_explorer = explorer.clone();
    let undo_explorer = explorer.clone();
    let redo_explorer = explorer.clone();
    let empty_trash_explorer = explorer.clone();

    menu.item(
        PopupMenuItem::new(match undo_label {
            Some(label) => format!("Undo {label}"),
            None => "Undo".to_string(),
        })
        .disabled(undo_label.is_none())
        .on_click(move |_, _window, cx| {
            undo_explorer.update(cx, |explorer, cx| explorer.undo(cx));
        }),
    )
    .item(
        PopupMenuItem::new(match redo_label {
            Some(label) => format!("Redo {label}"),
            None => "Redo".to_string(),
        })
        .disabled(redo_label.is_none())
        .on_click(move |_, _window, cx| {
            redo_explorer.update(cx, |explorer, cx| explorer.redo(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Empty Trash").on_click(move |_, _window, cx| {
            empty_trash_explorer.update(cx, |explorer, cx| explorer.empty_trash(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Show Hidden Files")
            .checked(show_hidden)
            .on_click(move |_, _window, cx| {
                toggle_explorer.update(cx, |explorer, cx| explorer.toggle_hidden(cx));
            }),
    )
}

fn file_row_menu(
    explorer: Entity<Explorer>,
    path: PathBuf,
    mut menu: PopupMenu,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let is_multi = explorer.read(cx).selected.len() > 1;

    if !is_multi {
        let rename_explorer = explorer.clone();
        let rename_path = path.clone();
        menu = menu.item(PopupMenuItem::new("Rename").on_click(move |_, window, cx| {
            rename_explorer.update(cx, |explorer, cx| {
                explorer.begin_rename(rename_path.clone(), window, cx);
            });
        }));
    }

    let cut_explorer = explorer.clone();
    let copy_explorer = explorer.clone();
    let copy_path_explorer = explorer.clone();
    let copy_path_target = path.clone();
    let delete_explorer = explorer.clone();
    let properties_explorer = explorer.clone();

    menu.item(PopupMenuItem::new("Cut").on_click(move |_, _window, cx| {
        cut_explorer.update(cx, |explorer, cx| explorer.cut_selection(cx));
    }))
    .item(PopupMenuItem::new("Copy").on_click(move |_, _window, cx| {
        copy_explorer.update(cx, |explorer, cx| explorer.copy_selection(cx));
    }))
    .item(
        PopupMenuItem::new("Copy Path(s)").on_click(move |_, _window, cx| {
            copy_path_explorer.update(cx, |explorer, cx| {
                let paths: Vec<PathBuf> = if explorer.selected.len() > 1 {
                    explorer.selected.iter().cloned().collect()
                } else {
                    vec![copy_path_target.clone()]
                };
                explorer.copy_paths_to_clipboard(&paths, cx);
            });
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Delete").on_click(move |_, _window, cx| {
            delete_explorer.update(cx, |explorer, cx| explorer.delete_selection(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Properties").on_click(move |_, window, cx| {
            properties_explorer.update(cx, |explorer, cx| {
                let paths: Vec<PathBuf> = explorer.selected.iter().cloned().collect();
                explorer.open_properties(paths, window, cx);
            });
        }),
    )
}

pub fn disk_usage_row_menu(
    explorer: Entity<Explorer>,
    path: PathBuf,
    menu: PopupMenu,
    _window: &mut Window,
    _cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let open_explorer = explorer.clone();
    let open_path = path.clone();
    let copy_explorer = explorer.clone();
    let copy_path = path.clone();
    let trash_explorer = explorer;

    menu.item(PopupMenuItem::new("Open").on_click(move |_, _window, cx| {
        open_explorer.update(cx, |explorer, cx| {
            explorer.open_disk_usage_entry(open_path.clone(), cx);
        });
    }))
    .item(
        PopupMenuItem::new("Copy Path").on_click(move |_, _window, cx| {
            copy_explorer.update(cx, |explorer, cx| {
                explorer.copy_paths_to_clipboard(std::slice::from_ref(&copy_path), cx);
            });
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Move to Trash").on_click(move |_, _window, cx| {
            trash_explorer.update(cx, |explorer, cx| {
                explorer.trash_disk_usage_entry(path.clone(), cx);
            });
        }),
    )
}

pub fn column_menu(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let visibility = explorer.read(cx).column_visibility;

    let type_explorer = explorer.clone();
    let size_explorer = explorer.clone();
    let modified_explorer = explorer.clone();

    menu.item(
        PopupMenuItem::new("Type")
            .checked(visibility.get(Column::Type))
            .on_click(move |_, _window, cx| {
                type_explorer.update(cx, |explorer, cx| {
                    explorer.toggle_column(Column::Type, cx);
                });
            }),
    )
    .item(
        PopupMenuItem::new("Size")
            .checked(visibility.get(Column::Size))
            .on_click(move |_, _window, cx| {
                size_explorer.update(cx, |explorer, cx| {
                    explorer.toggle_column(Column::Size, cx);
                });
            }),
    )
    .item(
        PopupMenuItem::new("Date Modified")
            .checked(visibility.get(Column::Modified))
            .on_click(move |_, _window, cx| {
                modified_explorer.update(cx, |explorer, cx| {
                    explorer.toggle_column(Column::Modified, cx);
                });
            }),
    )
}

fn empty_space_menu(
    explorer: Entity<Explorer>,
    menu: PopupMenu,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let show_hidden = explorer.read(cx).show_hidden;
    let has_clipboard = explorer.read(cx).clipboard.is_some()
        || cx
            .read_from_clipboard()
            .is_some_and(|item| item.paths().is_some());
    let undo_label = explorer.read(cx).undo_label();
    let redo_label = explorer.read(cx).redo_label();

    let new_folder_explorer = explorer.clone();
    let new_file_explorer = explorer.clone();
    let paste_explorer = explorer.clone();
    let toggle_explorer = explorer.clone();
    let properties_explorer = explorer.clone();
    let undo_explorer = explorer.clone();
    let redo_explorer = explorer.clone();

    let menu = menu
        .item(
            PopupMenuItem::new(match undo_label {
                Some(label) => format!("Undo {label}"),
                None => "Undo".to_string(),
            })
            .disabled(undo_label.is_none())
            .on_click(move |_, _window, cx| {
                undo_explorer.update(cx, |explorer, cx| explorer.undo(cx));
            }),
        )
        .item(
            PopupMenuItem::new(match redo_label {
                Some(label) => format!("Redo {label}"),
                None => "Redo".to_string(),
            })
            .disabled(redo_label.is_none())
            .on_click(move |_, _window, cx| {
                redo_explorer.update(cx, |explorer, cx| explorer.redo(cx));
            }),
        )
        .separator();

    menu.item(
        PopupMenuItem::new("New Folder")
            .icon(icon_theme::directory_menu_icon(cx))
            .on_click(move |_, window, cx| {
                new_folder_explorer
                    .update(cx, |explorer, cx| explorer.begin_new_folder(window, cx));
            }),
    )
    .item(
        PopupMenuItem::new("New File")
            .icon(icon_theme::generic_file_menu_icon(cx))
            .on_click(move |_, window, cx| {
                new_file_explorer.update(cx, |explorer, cx| explorer.begin_new_file(window, cx));
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Paste")
            .disabled(!has_clipboard)
            .on_click(move |_, _window, cx| {
                paste_explorer.update(cx, |explorer, cx| explorer.paste(cx));
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Show Hidden Files")
            .checked(show_hidden)
            .on_click(move |_, _window, cx| {
                toggle_explorer.update(cx, |explorer, cx| explorer.toggle_hidden(cx));
            }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Properties").on_click(move |_, window, cx| {
            properties_explorer.update(cx, |explorer, cx| {
                let dir = explorer.current_dir().to_path_buf();
                explorer.open_properties(vec![dir], window, cx);
            });
        }),
    )
}
