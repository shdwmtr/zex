use gpui::{App, KeyBinding, actions};

actions!(
    zex,
    [
        SelectAll,
        ToggleHidden,
        Rename,
        Delete,
        Copy,
        Cut,
        Paste,
        Undo,
        Redo,
        NewFolder,
        NewFile,
        MoveUp,
        MoveDown,
        Open,
        GoUp,
        GoBack,
        GoForward,
    ]
);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-a", SelectAll, Some("Explorer")),
        KeyBinding::new("ctrl-h", ToggleHidden, Some("Explorer")),
        KeyBinding::new("f2", Rename, Some("Explorer")),
        KeyBinding::new("delete", Delete, Some("Explorer")),
        KeyBinding::new("ctrl-c", Copy, Some("Explorer")),
        KeyBinding::new("ctrl-x", Cut, Some("Explorer")),
        KeyBinding::new("ctrl-v", Paste, Some("Explorer")),
        KeyBinding::new("ctrl-z", Undo, Some("Explorer")),
        KeyBinding::new("ctrl-shift-z", Redo, Some("Explorer")),
        KeyBinding::new("up", MoveUp, Some("Explorer")),
        KeyBinding::new("down", MoveDown, Some("Explorer")),
        KeyBinding::new("enter", Open, Some("Explorer")),
        KeyBinding::new("backspace", GoUp, Some("Explorer")),
        KeyBinding::new("left", GoBack, Some("Explorer")),
        KeyBinding::new("right", GoForward, Some("Explorer")),
    ]);
}
