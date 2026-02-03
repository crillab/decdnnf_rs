use super::{cli_manager, common};
use clap::{App, AppSettings, ArgMatches, SubCommand};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "translation";

impl<'a> super::command::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("translates a formula from an input format into an output format")
            .setting(AppSettings::DisableVersion)
            .args(&common::args_input())
            .arg(cli_manager::logging_level_cli_arg())
            .args(&common::args_output())
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let mut file_writer = common::create_output_file_writer(arg_matches)?;
        let formula_writer = common::create_output_formula_writer(arg_matches);
        formula_writer.write(&mut file_writer, &ddnnf)?;
        Ok(())
    }
}
