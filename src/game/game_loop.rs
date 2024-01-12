use crate::engine::{input::Input, system::System};

use super::app::App;

/// Runs the given systems in order.
macro_rules! run {
    ($app:ident, $system:expr) => {
        execute_system($app, $system)
    };
    ($app:ident, $system:expr, $($rest:expr),*) => {
        execute_system($app, $system);
        run!($($rest),*);
    };
}

pub fn game_loop(app: &mut App) {
    run!(app, Input::clear_inputs)
}

fn execute_system<Marker>(app: &mut App, mut system: impl System<Marker>) {
    system.run(app.resource_bank_mut());
}
