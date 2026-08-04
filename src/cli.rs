use std::path::{Path, PathBuf};

const HELP: &str = "\
A fast, keyboard-driven file explorer

Usage: zex [OPTIONS] [PATH]
       zex config [KEY]

Arguments:
  [PATH]  Directory to open, or a file whose parent directory should open
          with it selected. Relative paths resolve against the current
          working directory

Options:
      --disk-usage      Open straight into the disk usage view, rooted at
                         PATH if given
      --select <FILE>   Open FILE's parent directory with FILE selected
      --config <FILE>   Load settings from FILE instead of the default
                         config location
  -h, --help            Print help
  -V, --version         Print version

Commands:
  config [KEY]  List config keys, or show docs for one (e.g. `zex config
                inherit_from_zed`)
";

#[derive(Debug)]
pub struct Cli {
    path: Option<PathBuf>,
    disk_usage: bool,
    select: Option<PathBuf>,
    config: Option<PathBuf>,
    pub command: Option<Command>,
}

#[derive(Debug)]
pub enum Command {
    Config { key: Option<String> },
}

impl Cli {
    pub fn parse() -> Self {
        match Self::try_parse(std::env::args_os().skip(1)) {
            Ok(cli) => cli,
            Err(err) => {
                eprintln!("error: {err}");
                eprintln!("\n{HELP}");
                std::process::exit(2);
            }
        }
    }

    fn try_parse(args: impl Iterator<Item = std::ffi::OsString>) -> Result<Self, String> {
        let mut path: Option<PathBuf> = None;
        let mut disk_usage = false;
        let mut select: Option<PathBuf> = None;
        let mut config: Option<PathBuf> = None;
        let mut command: Option<Command> = None;

        let mut args = args.peekable();
        if let Some(first) = args.peek() {
            if first == "config" {
                args.next();
                let key = args.next().map(|s| s.to_string_lossy().into_owned());
                if let Some(extra) = args.next() {
                    return Err(format!("unexpected argument '{}'", extra.to_string_lossy()));
                }
                command = Some(Command::Config { key });
                return Ok(Cli { path, disk_usage, select, config, command });
            }
        }

        while let Some(arg) = args.next() {
            let arg_str = arg.to_string_lossy();
            match arg_str.as_ref() {
                "-h" | "--help" => {
                    print!("{HELP}");
                    std::process::exit(0);
                }
                "-V" | "--version" => {
                    println!("zex {}", env!("CARGO_PKG_VERSION"));
                    std::process::exit(0);
                }
                "--disk-usage" => disk_usage = true,
                "--select" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--select requires a value".to_string())?;
                    if path.is_some() {
                        return Err("the argument '--select <FILE>' cannot be used with a PATH argument".into());
                    }
                    select = Some(PathBuf::from(value));
                }
                "--config" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "--config requires a value".to_string())?;
                    config = Some(PathBuf::from(value));
                }
                _ if arg_str.starts_with("--") => {
                    return Err(format!("unknown option '{arg_str}'"));
                }
                _ => {
                    if select.is_some() {
                        return Err("the argument '--select <FILE>' cannot be used with a PATH argument".into());
                    }
                    if path.is_some() {
                        return Err(format!("unexpected argument '{arg_str}'"));
                    }
                    path = Some(PathBuf::from(arg));
                }
            }
        }

        Ok(Cli { path, disk_usage, select, config, command })
    }
}

#[derive(Debug)]
pub struct Startup {
    pub start_dir: PathBuf,
    pub select: Option<PathBuf>,
    pub disk_usage: bool,
    pub disk_usage_root: Option<PathBuf>,
    pub config: Option<PathBuf>,
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

fn expand_tilde(path: &Path) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path.to_path_buf();
    };
    if let Some(rest) = s.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if s == "~" {
        return home_dir();
    }
    path.to_path_buf()
}

fn resolve_arg_path(path: &Path) -> PathBuf {
    let expanded = expand_tilde(path);
    if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"))
            .join(expanded)
    }
}

impl Cli {
    pub fn resolve(self, default_start_dir: PathBuf) -> Result<Startup, String> {
        let (start_dir, select, explicit_dir) = if let Some(select_path) = self.select {
            let resolved = resolve_arg_path(&select_path);
            std::fs::symlink_metadata(&resolved)
                .map_err(|_| format!("zex: path not found: {}", resolved.display()))?;
            let parent = resolved
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| resolved.clone());
            (parent, Some(resolved), None)
        } else if let Some(path) = self.path {
            let resolved = resolve_arg_path(&path);
            let metadata = std::fs::metadata(&resolved)
                .map_err(|_| format!("zex: path not found: {}", resolved.display()))?;
            if metadata.is_dir() {
                (resolved.clone(), None, Some(resolved))
            } else {
                let parent = resolved
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| resolved.clone());
                (parent, Some(resolved), None)
            }
        } else {
            (default_start_dir, None, None)
        };

        Ok(Startup {
            start_dir,
            select,
            disk_usage: self.disk_usage,
            disk_usage_root: explicit_dir,
            config: self.config,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli(path: Option<PathBuf>, select: Option<PathBuf>) -> Cli {
        Cli { path, select, disk_usage: false, config: None, command: None }
    }

    #[test]
    fn no_path_falls_back_to_default_start_dir() {
        let default_dir = PathBuf::from("/some/default");

        let startup = cli(None, None).resolve(default_dir.clone()).unwrap();

        assert_eq!(startup.start_dir, default_dir);
        assert_eq!(startup.select, None);
        assert_eq!(startup.disk_usage_root, None);
    }

    #[test]
    fn missing_path_is_an_error() {
        let err = cli(Some(PathBuf::from("/definitely/does/not/exist/zex")), None)
            .resolve(PathBuf::from("/"))
            .unwrap_err();

        assert!(err.contains("/definitely/does/not/exist/zex"));
    }

    #[test]
    fn directory_path_becomes_start_dir_with_no_selection() {
        let dir = std::env::temp_dir();

        let startup = cli(Some(dir.clone()), None).resolve(PathBuf::from("/")).unwrap();

        assert_eq!(startup.start_dir, dir);
        assert_eq!(startup.select, None);
        assert_eq!(startup.disk_usage_root, Some(dir));
    }

    #[test]
    fn file_path_opens_parent_with_file_selected() {
        let dir = std::env::temp_dir();
        let file = dir.join("zex_cli_test_file_path.txt");
        std::fs::write(&file, b"").unwrap();

        let startup = cli(Some(file.clone()), None).resolve(PathBuf::from("/")).unwrap();

        std::fs::remove_file(&file).unwrap();

        assert_eq!(startup.start_dir, dir);
        assert_eq!(startup.select, Some(file));
        assert_eq!(startup.disk_usage_root, None);
    }

    #[test]
    fn select_flag_opens_parent_with_target_selected_even_for_a_directory() {
        let dir = std::env::temp_dir();
        let target = dir.join("zex_cli_test_select_dir");
        std::fs::create_dir_all(&target).unwrap();

        let startup = cli(None, Some(target.clone())).resolve(PathBuf::from("/")).unwrap();

        std::fs::remove_dir(&target).unwrap();

        assert_eq!(startup.start_dir, dir);
        assert_eq!(startup.select, Some(target));
        assert_eq!(startup.disk_usage_root, None);
    }

    #[test]
    fn select_flag_errors_when_target_missing() {
        let err = cli(None, Some(PathBuf::from("/definitely/does/not/exist/zex")))
            .resolve(PathBuf::from("/"))
            .unwrap_err();

        assert!(err.contains("/definitely/does/not/exist/zex"));
    }

    #[test]
    fn relative_path_resolves_against_current_dir() {
        let cwd = std::env::current_dir().unwrap();
        let dir_name = format!("zex_cli_test_relative_{}", std::process::id());
        std::fs::create_dir_all(cwd.join(&dir_name)).unwrap();

        let startup = cli(Some(PathBuf::from(&dir_name)), None)
            .resolve(PathBuf::from("/"))
            .unwrap();

        std::fs::remove_dir(cwd.join(&dir_name)).unwrap();

        assert_eq!(startup.start_dir, cwd.join(&dir_name));
    }

    #[test]
    fn bare_tilde_expands_to_home_dir() {
        let expected = home_dir();

        let startup = cli(Some(PathBuf::from("~")), None)
            .resolve(PathBuf::from("/"))
            .unwrap();

        assert_eq!(startup.start_dir, expected);
    }
}
