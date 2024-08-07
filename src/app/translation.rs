use super::{cli_manager, common};
use clap::{App, AppSettings, ArgMatches, SubCommand};
use decdnnf_rs::{BottomUpTraversal, C2dWriter, CheckingVisitor};

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
            .arg(common::arg_input_var())
            .arg(common::arg_n_vars())
            .arg(cli_manager::logging_level_cli_arg())
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let traversal_engine = BottomUpTraversal::new(Box::<CheckingVisitor>::default());
        let checking_data = traversal_engine.traverse(&ddnnf);
        common::print_warnings_and_errors(&checking_data)?;
        C2dWriter::write(&mut std::io::stdout(), &ddnnf)?;
        Ok(())
    }
}
