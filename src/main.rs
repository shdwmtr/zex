mod app;
mod cli;
mod explorer;
mod filesystem;
mod git;
mod keys;
mod settings;
mod theme;
mod ui;
mod workspace;

use std::path::PathBuf;

use clap::Parser;

fn main() {
    let cli = cli::Cli::parse();

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
