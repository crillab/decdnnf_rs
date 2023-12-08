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

//! A crate used to build apps with subcommands and logging.
//!
//! Logging is initialized using the `fern` crate with colors.
//! Subcommands are handled using the builtin [`Command`] trait, which takes advantage of the `clap` trait.
//!
//! [`Command`]: trait.Command.html

mod app_helper;
mod cli_manager;

pub use clap::{App, ArgGroup, AppSettings, Arg, ArgMatches, SubCommand};
pub use log::{debug, error, info, trace, warn};

pub use app_helper::app_helper::{init_logger, AppHelper};
pub use app_helper::license_command::LicenseCommand;
pub use cli_manager::cli_manager::logging_level_cli_arg;
pub use cli_manager::command::Command;
