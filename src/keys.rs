use gpui::{Action, App, KeyBinding, KeyContext, KeybindingKeystroke, SharedString, Window, actions};

actions!(
    zex,
    [
        SelectAll,
        ToggleHidden,
        ToggleSidebar,
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
        CloseDiskUsage,
        CloseProperties,
        NewTab,
        CloseTab,
        NextTab,
        PrevTab,
        OpenSearch,
    ]
);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-a", SelectAll, Some("Explorer")),
        KeyBinding::new("ctrl-h", ToggleHidden, Some("Explorer")),
        KeyBinding::new("ctrl-b", ToggleSidebar, Some("Workspace")),
        KeyBinding::new("ctrl-t", NewTab, Some("Workspace")),
        KeyBinding::new("ctrl-w", CloseTab, Some("Workspace")),
        KeyBinding::new("ctrl-tab", NextTab, Some("Workspace")),
        KeyBinding::new("ctrl-shift-tab", PrevTab, Some("Workspace")),
        KeyBinding::new("ctrl-f", OpenSearch, Some("Explorer")),
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
        KeyBinding::new("escape", CloseDiskUsage, Some("DiskUsage")),
        KeyBinding::new("escape", CloseProperties, Some("Properties")),
    ]);
}

pub fn shortcut_label(
    window: &Window,
    action: &dyn Action,
    context: &str,
) -> Option<SharedString> {
    let context = KeyContext::parse(context).ok()?;
    let binding = window.highest_precedence_binding_for_action_in_context(action, context)?;
    let label = binding
        .keystrokes()
        .iter()
        .map(format_keystroke)
        .collect::<Vec<_>>()
        .join(" ");
    Some(label.into())
}

fn format_keystroke(keystroke: &KeybindingKeystroke) -> String {
    let modifiers = keystroke.modifiers();
    let mut parts = Vec::new();
    if modifiers.control {
        parts.push("Ctrl".to_string());
    }
    if modifiers.alt {
        parts.push("Alt".to_string());
    }
    if modifiers.platform {
        parts.push("Super".to_string());
    }
    if modifiers.shift {
        parts.push("Shift".to_string());
    }
    parts.push(capitalize(keystroke.key()));
    parts.join("-")
}

fn capitalize(key: &str) -> String {
    if key.chars().count() == 1 {
        return key.to_uppercase();
    }
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => key.to_string(),
    }
}
