# Z-Explorer (zex)

A blazingly-fast file explorer/disk usage analyzer companion for [Zed Editor](https://github.com/zed-industries/zed), built using the same [GPUI renderer framework](https://www.gpui.rs/).

## Features

- **Built
- **Keyboard-driven** navigate, select, and act without reaching for the mouse (see [Keybindings](#keybindings))
- **Cut / copy / paste** with a real clipboard, not a re-implemented one
- **Undo / redo** for file operations, not just text
- **Trash, not delete** files go to the system trash and can come back
- **Bulk operations** with progress and cancellation for large copies/moves
- **Drag and drop**, in and out of the app
- **Custom icon themes**, JSON-defined, Zed-schema compatible
- **Custom color themes**, same Zed-schema compatibility, including themes installed via Zed extensions
- **Configurable sidebar** with sections and pinned locations
- **Client-side window decorations** on Linux, native chrome elsewhere
- **Git status** (opt-in) per-file status coloring/badges, folder aggregation, and a branch indicator, backed by the real `git` CLI

## Installing

zex is currently source-only. You'll need a recent Rust toolchain (edition 2024).

```sh
cargo run --release
```

The `gpui` dependency is vendored under `vendor/gpui` and patched in via `Cargo.toml`, so no separate checkout is needed.

## Usage

```
zex [PATH]
zex [PATH] --disk-usage
zex --select <FILE>
zex --config <FILE>
```

| Argument | Effect |
| --- | --- |
| `PATH` | Directory to open. If `PATH` is a file, its parent directory opens with that file selected. Relative paths resolve against the current working directory; `~` is expanded. Defaults to your home directory if omitted. |
| `--select <FILE>` | Open `FILE`'s parent directory with `FILE` selected. Can't be combined with `PATH`. |
| `--disk-usage` | Open straight into the disk usage view, rooted at `PATH` if given (or the whole disk otherwise). |
| `--config <FILE>` | Load settings from `FILE` instead of the default config location. |

A nonexistent `PATH` or `--select` target is a hard error (nonzero exit), not a silent fall back to the home directory.

## Configuration

zex reads a JSONC config from `$XDG_CONFIG_HOME/zex/config.json` (falling back to `~/.config/zex/config.json`) — icon/color themes, fonts, the sidebar, and git status integration all live there. Every field is optional, and comments/trailing commas are allowed.

The full field reference lives in the CLI, not here:

```sh
zex config              # list every key, grouped by section
zex config theme.mode   # docs, type, and default for one key
```

## Keybindings

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move selection |
| `Enter` | Open |
| `Backspace` | Go up a directory |
| `←` / `→` | Back / forward |
| `Ctrl-A` | Select all |
| `Ctrl-H` | Toggle hidden files |
| `F2` | Rename |
| `Delete` | Move to trash |
| `Ctrl-C` / `Ctrl-X` / `Ctrl-V` | Copy / cut / paste |
| `Ctrl-Z` / `Ctrl-Shift-Z` | Undo / redo |

## License

Apache 2.0. See [LICENSE](LICENSE).
