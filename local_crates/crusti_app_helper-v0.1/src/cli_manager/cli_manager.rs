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

use super::{command::Command, writable_string::WritableString};
use crate::{init_logger, app_helper::app_helper::init_logger_with_level};
use anyhow::{anyhow, Result};
use clap::{App, AppSettings, Arg};
use log::info;
use std::{ffi::OsString, str::FromStr};
use sysinfo::{ProcessorExt, System, SystemExt};

/// A structure used to handle the set of commands and to process the CLI arguments against them.
pub(crate) struct CliManager<'a> {
    app_name: &'a str,
    version: &'a str,
    author: &'a str,
    about: &'a str,
    commands: Vec<Box<dyn Command<'a>>>,
}

const APP_HELPER_LOGGING_LEVEL_ARG: &str = "APP_HELPER_LOGGING_LEVEL_ARG";

pub fn logging_level_cli_arg<'a>() -> Arg<'a, 'a> {
    Arg::with_name(APP_HELPER_LOGGING_LEVEL_ARG)
        .long("logging-level")
        .multiple(false)
        .default_value("info")
        .possible_values(&["trace", "debug", "info", "warn", "error", "off"])
        .help("set the minimal logging level")
}

impl<'a> CliManager<'a> {
    pub fn new(app_name: &'a str, version: &'a str, author: &'a str, about: &'a str) -> Self {
        CliManager {
            app_name,
            version,
            author,
            about,
            commands: vec![],
        }
    }

    pub fn add_command(&mut self, command: Box<dyn Command<'a>>) {
        self.commands.push(command);
    }

    pub fn parse_cli<I, T>(&self, args: I) -> Result<()>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let args: Vec<T> = args.into_iter().collect();
        let mut app = App::new(self.app_name)
            .global_setting(AppSettings::DisableVersion)
            .global_setting(AppSettings::VersionlessSubcommands)
            .setting(AppSettings::NeedsSubcommandHelp)
            .setting(AppSettings::SubcommandRequired)
            .version(self.version)
            .author(self.author)
            .about(self.about);
        for c in self.commands.iter() {
            app = app.subcommand(c.clap_subcommand());
        }
        let matches_result = app
            .clone()
            .get_matches_from_safe(&mut args.clone().into_iter());
        match matches_result {
            Ok(matches) => {
                for c in self.commands.iter() {
                    if let Some(matches) = matches.subcommand_matches(c.name()) {
                        let log_level = if let Some(str_log_level) =
                            matches.value_of(APP_HELPER_LOGGING_LEVEL_ARG)
                        {
                            log::LevelFilter::from_str(str_log_level).unwrap()
                        } else {
                            log::LevelFilter::Info
                        };
                        init_logger_with_level(log_level);
                        info!("{} {}", self.app_name, self.version);
                        sys_info();
                        return c.execute(matches);
                    }
                }
                panic!("unreachable"); // kcov-ignore
            }
            Err(clap::Error {
                kind: clap::ErrorKind::HelpDisplayed,
                ..
            }) => {
                init_logger();
                self.print_help(&mut app, args.as_slice());
                Ok(())
            }
            Err(e) => {
                init_logger();
                info!("{} {}", self.app_name, self.version);
                Err(anyhow!("{}", e))
            }
        }
    }

    fn print_help<T>(&self, app: &mut App, args: &[T])
    where
        T: Into<OsString> + Clone,
    {
        const HELP_STRINGS: [&str; 3] = ["help", "-h", "--help"];
        fn print_message(message: WritableString) {
            message.to_string().split('\n').for_each(|s| info!("{}", s));
            info!("");
        }
        fn search_subcommand(commands: &[Box<dyn Command>], subcommand_arg: &str) -> bool {
            for c in commands.iter() {
                if c.name() == subcommand_arg {
                    let mut message = WritableString::default();
                    c.clap_subcommand().write_long_help(&mut message).unwrap();
                    print_message(message);
                    return true;
                }
            }
            panic!("unreachable") // kcov-ignore
        }
        if args.len() >= 2 {
            let arg1 = args[1].clone().into().into_string().unwrap();
            if !HELP_STRINGS.contains(&arg1.as_ref() as &&str)
                && search_subcommand(&self.commands, &arg1)
            {
                return;
            }
            if args.len() >= 3
                && HELP_STRINGS.contains(&arg1.as_ref() as &&str)
                && search_subcommand(
                    &self.commands,
                    args[2].clone().into().into_string().as_ref().unwrap(),
                )
            {
                return;
            }
        }
        let mut message = WritableString::default();
        app.write_long_help(&mut message).unwrap();
        print_message(message);
    }
}

fn sys_info() {
    info!("----------------------------------------");
    let sys = System::new_all();
    let unknown = || "[unknown]".to_string();
    info!("running on {}", sys.host_name().unwrap_or_else(unknown));
    info!(
        "OS is {} {} with kernel {}",
        sys.name().unwrap_or_else(unknown),
        sys.os_version().unwrap_or_else(unknown),
        sys.kernel_version().unwrap_or_else(unknown)
    );
    let mut processor_kinds: Vec<&str> = sys.processors().iter().map(|p| p.brand()).collect();
    processor_kinds.sort();
    processor_kinds.dedup();
    info!(
        "physical core count: {} {:?}",
        sys.physical_core_count().unwrap(),
        processor_kinds
    );
    info!("total memory: {} KB", sys.total_memory());
    info!("----------------------------------------");
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, SubCommand};
    use std::{cell::RefCell, rc::Rc};

    struct LocalCommand {
        command_involved: Rc<RefCell<bool>>,
        argument_set: Rc<RefCell<bool>>,
    }

    impl LocalCommand {
        fn new(command_involved: Rc<RefCell<bool>>, argument_set: Rc<RefCell<bool>>) -> Self {
            LocalCommand {
                command_involved,
                argument_set,
            }
        }
    }

    impl<'a> Command<'a> for LocalCommand {
        fn name(&self) -> &str {
            "local_command_name"
        }

        fn clap_subcommand(&self) -> App<'a, 'a> {
            SubCommand::with_name("local_command_name")
                .about("local_command_about")
                .arg(Arg::with_name("arg_name").short("a"))
                .setting(AppSettings::DisableVersion)
        }

        fn execute(&self, arg_matches: &clap::ArgMatches<'_>) -> Result<()> {
            (*self.command_involved.borrow_mut()) = true;
            if arg_matches.is_present("arg_name") {
                (*self.argument_set.borrow_mut()) = true;
            }
            Ok(())
        }
    }

    fn test_local_command_result(
        args: Vec<&'static str>,
    ) -> Result<(Rc<RefCell<bool>>, Rc<RefCell<bool>>)> {
        let mut manager = CliManager::new("app_name", "app_version", "author", "about");
        let command_involved = Rc::new(RefCell::new(false));
        let argument_set = Rc::new(RefCell::new(false));
        let command = LocalCommand::new(Rc::clone(&command_involved), Rc::clone(&argument_set));
        manager.add_command(Box::new(command));
        match manager.parse_cli(args) {
            Ok(()) => Ok((command_involved, argument_set)),
            Err(e) => Err(e),
        }
    }

    #[test]
    fn test_command_involved() {
        let command = test_local_command_result(vec!["app_name", "local_command_name"]).unwrap();
        assert!(*command.0.borrow());
        assert!(!*command.1.borrow());
    }

    #[test]
    fn test_command_and_arg_involved() {
        let command =
            test_local_command_result(vec!["app_name", "local_command_name", "-a"]).unwrap();
        assert!(*command.0.borrow());
        assert!(*command.1.borrow());
    }

    #[test]
    fn test_no_subcommand() {
        assert!(test_local_command_result(vec!["app_name"]).is_err());
    }

    #[test]
    fn test_wrong_subcommand() {
        assert!(test_local_command_result(vec!["app_name", "foo"]).is_err());
    }

    #[test]
    fn test_wrong_arg() {
        assert!(test_local_command_result(vec!["app_name", "local_command_name", "-b"]).is_err());
    }

    #[test]
    fn test_help() {
        test_local_command_result(vec!["app_name", "-h"]).unwrap();
    }

    #[test]
    fn test_help_subcommand() {
        test_local_command_result(vec!["app_name", "help"]).unwrap();
    }

    #[test]
    fn test_help_for_subcommand() {
        test_local_command_result(vec!["app_name", "help", "local_command_name"]).unwrap();
    }

    #[test]
    fn test_subcommand_help() {
        test_local_command_result(vec!["app_name", "local_command_name", "-h"]).unwrap();
    }
}
