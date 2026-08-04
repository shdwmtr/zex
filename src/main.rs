mod app;
mod cli;
mod config_docs;
mod explorer;
mod filesystem;
mod git;
mod keys;
mod settings;
mod theme;
mod ui;
mod workspace;

use std::path::PathBuf;

fn main() {
    let cli = cli::Cli::parse();

    if let Some(cli::Command::Config { key }) = &cli.command {
        match key {
            None => config_docs::print_list(),
            Some(key) => {
                if let Err(err) = config_docs::print_key(key) {
                    eprintln!("{err}");
                    std::process::exit(1);
                }
            }
        }
        return;
    }

    let default_start_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"));

    let startup = match cli.resolve(default_start_dir) {
        Ok(startup) => startup,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };

    let settings = settings::load(startup.config.as_deref());
    app::run(settings, startup);
}
