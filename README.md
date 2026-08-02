<img height="220" alt="image" align="left" src="https://github.com/user-attachments/assets/34fab8fb-c8c4-4b8d-a0d8-43bd0ca5b1bd" />

### Z-Explorer (zex)
An extremely fast file explorer and disk usage analyzer companion for Linux, built for the [Zed ecosystem](https://github.com/zed-industries/zed) using the same [GPUI renderer framework](https://www.gpui.rs/). 

Naturally, Zex can inherit config, themes, and icon themes from Zed OOTB; its a complete visual match.

&nbsp;
### Preview (Zed on left, Zex on right)
![](https://github.com/user-attachments/assets/5fb253a2-c27e-4d4c-8b1e-4b364da86564)

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
