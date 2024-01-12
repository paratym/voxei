use game::setup::setup_resources;

pub mod constants;
pub mod engine;
pub mod game;

fn main() {
    let mut app = game::app::App::new();

    setup_resources(&mut app);

    app.run();
}
