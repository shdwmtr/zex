# Z-Explorer (zex)

An extremely fast file explorer and disk usage analyzer companion for Linux, built for the [Zed ecosystem](https://github.com/zed-industries/zed) using the same [GPUI renderer framework](https://www.gpui.rs/). 

Naturally, Zex can inherit config, themes, and icon themes from Zed OOTB; its a complete visual match.

## Preview (Zed on left, Zex on right)
![](https://github.com/user-attachments/assets/5fb253a2-c27e-4d4c-8b1e-4b364da86564)

## Building from source
Zex requires the recent 2024 rust compiler toolchain.

```bash
# Run zexplorer
$ cargo run

# Build optimized release
$ cargo build --profile optimized-release
```

## Installation
`make install` builds the optimized release and installs the binary, `.desktop` entry, and icon:

```bash
$ make build
$ sudo make install               # installs to /usr/local by default
$ sudo make PREFIX=/usr install   # or a custom prefix
```

Arch Linux users can instead build the [PKGBUILD](packaging/PKGBUILD) in `packaging/`:

```bash
$ cd packaging && makepkg -si
```

To make Zex the default handler for folders (so it opens from your file manager, "Open With", etc.):

```bash
$ xdg-mime default zex.desktop inode/directory
```

Uninstall with `sudo make uninstall` (or `pacman -R zex-git` if installed via the PKGBUILD).

## Configuration
Zex reads a JSONC config from `$XDG_CONFIG_HOME/zex/config.json`, falling back to `~/.config/zex/config.json`. Every field is optional, and comments/trailing commas are allowed.

The full field reference lives in the CLI:

```sh
# list every key, grouped by section
$ zex config
# docs, type, and default for one key   
$ zex config theme.mode
```

### Boilerplate configuration
`$XDG_CONFIG_HOME/zex/config.json`

```json
{
    "inherit_from_zed": true,
    "show_hidden_files": true,
    "sidebar": [
        { "name": "Home", "path": "~" },
        "separator",
        {
            "section": "Places",
            "entries": [
                { "name": "Downloads", "path": "~/Downloads" },
                { "name": "Documents", "path": "~/Documents" },
                { "name": "Development", "path": "~/Development" }
            ]
        },
        "separator",
        { "name": "Local Disk", "path": "/" },
        { "name": "Trash", "path": ":trash" }
    ],
    "git": {
        "enabled": true
    }
}
```

## Contributions
Contributions are welcome; there's plenty of room to help: new features, bug fixes, performance improvements, or just cleaning up rough edges. Open a PR, we'll take a look.

## License

Apache 2.0. See [LICENSE](LICENSE).
