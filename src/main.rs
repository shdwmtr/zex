mod app;
mod explorer;
mod filesystem;
mod git;
mod keys;
mod settings;
mod theme;
mod ui;

fn main() {
    let settings = settings::load();
    app::run(settings);
}
