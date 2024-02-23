pub mod app;
pub mod constants;
pub mod engine;
pub mod game;
pub mod game_loop;
pub mod settings;
pub mod setup;

fn main() {
    let mut app = app::App::new();

    setup::setup_resources(&mut app);

    app.run();
}
