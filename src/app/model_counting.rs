use super::{cli_manager, common};
use clap::{App, AppSettings, ArgMatches, SubCommand};
use decdnnf_rs::{BiBottomUpVisitor, BottomUpTraversal, CheckingVisitor, ModelCountingVisitor};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-counting";

impl<'a> super::command::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("counts the models of the formula")
            .setting(AppSettings::DisableVersion)
            .arg(common::arg_input_var())
            .arg(common::arg_n_vars())
            .arg(cli_manager::logging_level_cli_arg())
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let traversal_visitor = BiBottomUpVisitor::new(
            Box::<CheckingVisitor>::default(),
            Box::<ModelCountingVisitor>::default(),
        );
        let traversal_engine = BottomUpTraversal::new(Box::new(traversal_visitor));
        let (checking_data, model_counting_data) = traversal_engine.traverse(&ddnnf);
        common::print_warnings_and_errors(&checking_data)?;
        println!("{}", model_counting_data.n_models());
        Ok(())
    }
}
