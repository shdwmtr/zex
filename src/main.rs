mod app;
mod explorer;
mod filesystem;
mod keys;
mod settings;
mod theme;
mod ui;

fn main() {
    let settings = settings::load();
    app::run(settings);
}
