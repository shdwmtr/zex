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

zex reads a JSON config from `$XDG_CONFIG_HOME/zex/config.json` (falling back to `~/.config/zex/config.json`). Every field is optional — an empty or missing file just gets you the defaults. Comments and trailing commas are allowed.

```jsonc
{
  "inherit_from_zed": true,
  "icon_theme": "Zex Default",
  "ui_font_family": "JetBrainsMono Nerd Font Mono",
  "ui_font_size": 15.0,
  "ui_font_weight": 500.0,
  "show_hidden_files": false,
  "sidebar": [
    { "name": "Home", "path": "/home/you" },
    {
      "section": "Work",
      "entries": [
        { "name": "Projects", "path": "/home/you/Projects" },
        { "name": "Downloads", "path": "/home/you/Downloads" }
      ]
    }
  ],
  "theme": {
    "mode": "dark",
    "light": "VSCode Light Modern",
    "dark": "VSCode Dark Modern"
  },
  "git": {
    "enabled": false,
    "status": {
      "enabled": true,
      "show_untracked": true,
      "show_ignored": false,
      "dim_ignored": true,
      "aggregate_folders": true,
      "badge_style": "text_color"
    },
    "branch": {
      "enabled": true,
      "show_in_status_bar": true,
      "show_dirty_indicator": true,
      "show_ahead_behind": true
    },
    "refresh": {
      "poll_interval_ms": 2000
    },
    "cli": {
      "binary_path": null,
      "timeout_ms": 3000,
      "max_repo_entries": null
    }
  }
}
```

### Inheriting Zed's settings

Set `"inherit_from_zed"` to `true` to load `$XDG_CONFIG_HOME/zed/settings.json` as a base, or to a string to point at a specific settings file (e.g. `"~/.config/zed/settings.json"` or an absolute path). Only fields zex understands are used from it — anything Zed-specific with no zex equivalent is ignored.

Any field you also declare in zex's own `config.json` wins, merged in field by field rather than replacing whole sections — so setting `"theme": { "light": "..." }` in zex overrides just that one field while still picking up everything else (icon theme, fonts, the rest of the theme block, etc.) from Zed's config.

Sidebar entries can be listed flat or grouped into named sections — mix and match as needed.

### Color themes

`theme` is optional — omit it (or leave `dark`/`light` unset) and zex uses its own bundled dark palette, itself expressed in the same Zed theme schema (`assets/themes/zex-default.json`).

| Field | Default | Effect |
| --- | --- | --- |
| `theme.mode` | `"dark"` | `"dark"` / `"light"` force a palette; `"system"` follows the OS light/dark setting at launch. |
| `theme.light` | `null` | Theme name to use when the resolved mode is light. |
| `theme.dark` | `null` | Theme name to use when the resolved mode is dark. |

A theme name is resolved the same way icon themes already are: zex scans installed Zed extensions under `$XDG_DATA_HOME/zed/extensions/installed/*/extension.toml` for a `themes = [...]` entry, parses the referenced theme JSON (Zed's real theme schema — the same files Zed itself loads), and matches on the theme's `name`. Zex also vendors Zed's own built-in theme families (Ayu, Gruvbox, One — the ones compiled directly into the `zed` binary rather than shipped as extensions) under `assets/themes/zed_builtin/`, so `"dark": "Gruvbox Dark"` resolves without that theme being installed as an extension. If the name isn't found in either place, or `theme.light`/`theme.dark` is left unset for the active mode, zex falls back to its own bundled default. A matched theme only needs to define the handful of keys zex actually uses (background, borders, text, selection/hover states, and the generic `modified`/`created`/`deleted`/`renamed`/`conflict`/`ignored` status roles for git status colors — the same roles Zed's own project panel uses) — anything it omits falls back to the bundled default rather than erroring.

Color themes are resolved once at startup, same as icon themes and fonts — changing `theme` requires restarting zex.

### Git status

Git awareness is off by default (`git.enabled: false`) — flip it on and every sub-toggle below defaults to a sane "on" state unless you override it:

| Field | Default | Effect |
| --- | --- | --- |
| `git.enabled` | `false` | Master switch. Everything else in `git` is inert until this is `true`. |
| `status.enabled` | `true` | Per-file/folder status decoration in the file list. |
| `status.show_untracked` | `true` | Include untracked files in status. |
| `status.show_ignored` | `false` | Include `.gitignore`d files/folders in status at all. |
| `status.dim_ignored` | `true` | Fade ignored rows (only matters if `show_ignored` is on). |
| `status.aggregate_folders` | `true` | A folder shows the worst status of anything changed inside it. |
| `status.badge_style` | `"text_color"` | `"text_color"` recolors the filename, `"icon"` adds a small colored letter chip (`M`/`A`/`D`/`R`/`U`/`I`/`!`), `"both"` does both. Colors always come from the active [color theme](#color-themes), not a separate override. |
| `branch.enabled` | `true` | Master switch for the branch indicator. |
| `branch.show_in_status_bar` | `true` | Show the current branch name in the status bar. |
| `branch.show_dirty_indicator` | `true` | Small dot next to the branch name when the working tree isn't clean. |
| `branch.show_ahead_behind` | `true` | `↑N`/`↓N` counts relative to the upstream branch. |
| `refresh.poll_interval_ms` | `2000` | How often to re-run `git status` in the background, in addition to the refresh that already happens on navigation and filesystem changes. `0` disables polling (git-only changes, like a commit from another terminal, won't be picked up until you navigate away and back). |
| `cli.binary_path` | `null` | Override the `git` binary used (defaults to whatever `git` resolves to on `PATH`). |
| `cli.timeout_ms` | `3000` | Kill and ignore the `git status` call if it takes longer than this. |
| `cli.max_repo_entries` | `null` | If the status listing has more entries than this, skip decorating rather than render a huge repo's worth of badges. |

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
