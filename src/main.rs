//! See the library documentation for more information.

mod app;

use app::{
    app_helper::AppHelper, command::Command, DirectAccessCommand, ModelComputerCommand,
    ModelCountingCommand, ModelEnumerationCommand, SamplingCommand, TranslationCommand,
};

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
    #[allow(unused_mut)]
    let mut commands: Vec<Box<dyn Command>> = vec![
        Box::<DirectAccessCommand>::default(),
        Box::<ModelComputerCommand>::default(),
        Box::<ModelCountingCommand>::default(),
        Box::<ModelEnumerationCommand>::default(),
        Box::<SamplingCommand>::default(),
        Box::<TranslationCommand>::default(),
    ];
    #[cfg(feature = "mpi")]
    commands.push(Box::<app::ModelEnumerationMPICommand>::default());
    for c in commands {
        app.add_command(c);
    }
    app
}

fn main() {
    let app = create_app_helper();
    app.launch_app();
}
