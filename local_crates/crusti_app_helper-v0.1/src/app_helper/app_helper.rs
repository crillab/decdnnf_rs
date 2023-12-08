// crusti_app_helper
// Copyright (C) 2020  Univ. Artois & CNRS
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::cli_manager::{cli_manager::CliManager, command::Command};
use anyhow::Result;
use log::{error, info};
use std::{ffi::OsString, sync::Once, time::SystemTime};

static LOGGER_INIT: Once = Once::new();

/// The main struct used to build an app.
///
/// This helper class should be used this way:
/// 1. create a new instance of this helper,
/// 2. add the commands for your app,
/// 3. call [`launch_app`].
///
/// For more information about commands see the documentation of the [`Command`] trait.
///
/// Calling [`launch_app`] is probably the last thing you do in your app.
/// It initializes the logger, reads the CLI arguments, and execute the right command.
/// If an error is returned by a command, the error stack is displayed and a status of 1 is returned to the system.
///
/// [`Command`]: trait.Command.html
/// [`launch_app`]: struct.AppHelper.html#method.launch_app
pub struct AppHelper<'a> {
    cli_manager: CliManager<'a>,
}

impl<'a> AppHelper<'a> {
    /// Creates a new instance of the helper.
    ///
    /// The author name and a description of the application must be provided.
    /// Hint: you can add the mail address of the author at the end of its name for it to be displayed.
    /// They will be displayed at the app startup.
    ///
    /// # Arguments
    /// * `author` - the author name
    /// * `about` - a textual description of the app
    pub fn new(app_name: &'a str, version: &'a str, author: &'a str, about: &'a str) -> Self {
        AppHelper {
            cli_manager: CliManager::new(app_name, version, author, about),
        }
    }

    /// Adds a new command to the app. See [`Command`] for more information.
    ///
    /// # Arguments
    /// * `command` - the command
    ///
    /// [`Command`]: trait.Command.html
    pub fn add_command(&mut self, command: Box<dyn Command<'a>>) {
        self.cli_manager.add_command(command);
    }

    /// Launch the application.
    ///
    /// The command line arguments are read through `std::env::args_os()`.
    ///
    /// Calling this function is probably the last thing you do in your app.
    /// It initializes the logger, reads the CLI arguments, and execute the right command.
    /// If an error is returned by a command, the error stack is displayed and a status of 1 is returned to the system.
    ///
    /// This function consumes the helper.
    pub fn launch_app(self) {
        self.launch_app_with_args(std::env::args_os())
    }

    /// Launch the application.
    ///
    /// The command line arguments are provided as a function parameter.
    ///
    /// Calling this function is probably the last thing you do in your app.
    /// It initializes the logger, reads the CLI arguments, and execute the right command.
    /// If an error is returned by a command, the error stack is displayed and a status of 1 is returned to the system.
    ///
    /// This function consumes the helper.
    pub fn launch_app_with_args<I, T>(self, args: I)
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        if let Err(e) = self.execute_app(args) {
            error!("an error occurred: {}", e);
            e.chain()
                .skip(1)
                .for_each(|err| error!("caused by: {}", err));
            std::process::exit(1);
        }
    }

    fn execute_app<I, T>(&self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let start_time = SystemTime::now();
        let result = self.cli_manager.parse_cli(args);
        if result.is_ok() {
            info!(
                "exiting successfully after {:?}",
                start_time.elapsed().unwrap()
            );
        }
        result
    }
}

pub fn init_logger() {
    init_logger_with_level(log::LevelFilter::Info)
}

pub fn init_logger_with_level(level: log::LevelFilter) {
    LOGGER_INIT.call_once(|| {
        let colors = fern::colors::ColoredLevelConfig::new().info(fern::colors::Color::Cyan);
        fern::Dispatch::new()
            .format(move |out, message, record| {
                out.finish(format_args!(
                    "![{:5}] {} {}",
                    colors.color(record.level()),
                    chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
                    message
                ))
            })
            .level(level)
            .chain(std::io::stdout())
            .apply()
            .unwrap_or(());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{App, Arg, SubCommand};

    struct LocalCommand;

    impl LocalCommand {
        // kcov-ignore-start
        fn new() -> Self {
            LocalCommand
        }
        // kcov-ignore-end
    }

    impl<'a> Command<'a> for LocalCommand {
        fn name(&self) -> &str {
            "local_command_name"
        }

        fn clap_subcommand(&self) -> App<'a, 'a> {
            SubCommand::with_name("local_command_name")
                .about("local_command_about")
                .arg(Arg::with_name("kill").short("k"))
        }

        fn execute(&self, arg_matches: &clap::ArgMatches<'_>) -> Result<()> {
            if arg_matches.is_present("kill") {
                Err(anyhow::anyhow!("foo"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn test_no_args() {
        init_logger();
        let mut h = AppHelper::new(
            option_env!("CARGO_PKG_NAME").unwrap_or("unknown app name"),
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown version"),
            "author",
            "about",
        );
        let c = LocalCommand::new();
        h.add_command(Box::new(c));
        h.execute_app(vec![] as Vec<&'static str>).unwrap_err();
    }

    #[test]
    fn test_no_subcommand() {
        init_logger();
        let mut h = AppHelper::new(
            option_env!("CARGO_PKG_NAME").unwrap_or("unknown app name"),
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown version"),
            "author",
            "about",
        );
        let c = LocalCommand::new();
        h.add_command(Box::new(c));
        h.execute_app(vec!["app"]).unwrap_err();
    }

    #[test]
    fn test_subcommand_ok() {
        init_logger();
        let mut h = AppHelper::new(
            option_env!("CARGO_PKG_NAME").unwrap_or("unknown app name"),
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown version"),
            "author",
            "about",
        );
        let c = LocalCommand::new();
        h.add_command(Box::new(c));
        h.execute_app(vec!["app", "local_command_name"]).unwrap();
    }

    #[test]
    fn test_subcommand_err() {
        init_logger();
        let mut h = AppHelper::new(
            option_env!("CARGO_PKG_NAME").unwrap_or("unknown app name"),
            option_env!("CARGO_PKG_VERSION").unwrap_or("unknown version"),
            "author",
            "about",
        );
        let c = LocalCommand::new();
        h.add_command(Box::new(c));
        h.execute_app(vec!["app", "local_command_name", "-k"])
            .unwrap_err();
    }
}
