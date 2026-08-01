# Z-Explorer (zex)

<img height="200" alt="image" align="left" src="https://github.com/user-attachments/assets/34fab8fb-c8c4-4b8d-a0d8-43bd0ca5b1bd" />

### What is zex?

An extremely fast file explorer/disk usage analyzer companion for [Zed Editor](https://github.com/zed-industries/zed), built using the same [GPUI renderer framework](https://www.gpui.rs/).

Naturally, Zex picks up user-installed Zed themes/icons themes automatically; its a complete visual match.
&nbsp;

&nbsp; &nbsp;
&nbsp;
&nbsp;
### Preview (Zed on left, Zex on right)

![](https://github.com/user-attachments/assets/5fb253a2-c27e-4d4c-8b1e-4b364da86564)

### Getting Started

```
$ zex --help

$ zex [PATH]
$ zex [PATH] --disk-usage
$ zex --select <FILE>
$ zex --config <FILE>
```

## Installation

Zex is currently source-only until the 1.0.0 release.

```bash
# Run zexplorer
$ cargo run

# Build optimized release
$ cargo build --profile optimized-release
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

## Contributions

Contributions are welcome; there's plenty of room to help: new features, bug fixes, performance improvements, or just cleaning up rough edges. Open a PR, we'll take a look.

## License

Apache 2.0. See [LICENSE](LICENSE).
