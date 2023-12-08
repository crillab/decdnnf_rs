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
use clap::App;
use clap::ArgMatches;

/// A trait for the app available commands.
///
/// Commands provide their own CLI argument requirements (using clap)
/// and execute themselves given the CLI arguments.
///
/// Each command must have a unique name.
pub trait Command<'a> {
    /// Returns the name of the command.
    fn name(&self) -> &str;

    /// Returns the clap subcommand describing the available CLI arguments for this command.
    fn clap_subcommand(&self) -> App<'a, 'a>;

    /// Executes the command given its arguments.
    /// The function returns `Ok(())` iff the main app should exit with a success status code.
    ///
    /// The arguments are the one returned by clap by calling one of its `get_matches` methods.
    ///
    /// # Arguments
    ///
    /// * `arg_matches` - the arguments for the command
    fn execute(&self, arg_matches: &ArgMatches<'_>) -> Result<()>;
}
