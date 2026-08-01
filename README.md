# Z-Explorer (zex)

An extremelt fast file explorer/disk usage analyzer companion for [Zed Editor](https://github.com/zed-industries/zed), built using the same [GPUI renderer framework](https://www.gpui.rs/).

Naturally, Zex picks up user-installed Zed themes/icons themes automatically; its a complete visual match.  

### Preview (Zed on left, Zex on right)

![](https://github.com/user-attachments/assets/5fb253a2-c27e-4d4c-8b1e-4b364da86564)

## Usage

```
$ zex --help

$ zex [PATH]
$ zex [PATH] --disk-usage
$ zex --select <FILE>
$ zex --config <FILE>
```

## Configuration

Zex reads a JSONC config from `$XDG_CONFIG_HOME/zex/config.json`, falling back to `~/.config/zex/config.json`. Every field is optional, and comments/trailing commas are allowed.

The full field reference lives in the CLI:

```sh
# list every key, grouped by section
$ zex config
# docs, type, and default for one key   
$ zex config theme.mode
```

## License

Apache 2.0. See [LICENSE](LICENSE).
