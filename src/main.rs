//! See the library documentation for more information.

mod app;

use app::{ModelCountingCommand, TranslationCommand};
use crusti_app_helper::{AppHelper, Command};

pub(crate) fn create_app_helper() -> AppHelper<'static> {
    let app_name = option_env!("CARGO_PKG_NAME").unwrap_or("unknown app name");
    let app_version = option_env!("CARGO_PKG_VERSION").unwrap_or("unknown version");
    let authors = option_env!("CARGO_PKG_AUTHORS").unwrap_or("unknown authors");
    let mut app = AppHelper::new(
        app_name,
        app_version,
        authors,
        "decdnnf-rs, a library for Decision-DNNFs.",
    );
    let commands: Vec<Box<dyn Command>> = vec![
        Box::<ModelCountingCommand>::default(),
        Box::<TranslationCommand>::default(),
    ];
    for c in commands {
        app.add_command(c);
    }
    app
}

fn main() {
    let app = create_app_helper();
    app.launch_app();
}
