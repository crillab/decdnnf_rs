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

use anyhow::Result;
use clap::{App, AppSettings, ArgMatches, SubCommand};
use log::info;

use crate::Command;

/// A command used to display the license of an app.
///
/// The license text must be given, for example by reading the `LICENSE` file using `include_str!`.
///
/// It does not have any options.
#[derive(Clone)]
pub struct LicenseCommand {
    license_text: String,
}

/// The name of the command, as it must be typed in the command line.
pub const NAME: &str = "license";

impl LicenseCommand {
    /// Builds a new [`LicenseCommand`] given the license text.
    ///
    /// # Arguments
    /// * `license_text` - the license text
    ///
    /// [`LicenseCommand`]: struct.LicenseCommand.html
    pub fn new(license_text: String) -> Self {
        LicenseCommand { license_text }
    }
}

impl<'a> Command<'a> for LicenseCommand {
    fn name(&self) -> &'static str {
        NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(NAME)
            .about("Displays the software\'s license")
            .setting(AppSettings::DisableVersion)
    }

    fn execute(&self, _arg_matches: &ArgMatches<'_>) -> Result<()> {
        info!("");
        self.license_text.split('\n').for_each(|s| info!("{}", s));
        info!("");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name() {
        let l = LicenseCommand::new("foo".to_string());
        assert_eq!(NAME, l.name());
    }

    fn execute(args: &[String]) -> Result<()> {
        let cmd = LicenseCommand::new("foo".to_string());
        let app = App::new("bar").subcommand(cmd.clap_subcommand());
        let arg_matches = app.get_matches_from_safe(args).unwrap();
        cmd.execute(arg_matches.subcommand_matches("license").unwrap())
    }

    #[test]
    fn test_manage_arguments_default() {
        let cli_args = vec!["bar".to_string(), "license".to_string()];
        execute(cli_args.as_slice()).unwrap();
    }

    #[test]
    fn test_manage_arguments_help() {
        let cli_args = vec!["bar".to_string(), "license".to_string(), "-h".to_string()];
        let cmd = LicenseCommand::new("foo".to_string());
        let app = App::new("bar").subcommand(cmd.clap_subcommand());
        match app.get_matches_from_safe(cli_args) {
            Err(clap::Error {
                kind: clap::ErrorKind::HelpDisplayed,
                ..
            }) => {}
            _ => panic!(), // kcov-ignore (unreachable)
        }
    }
}
