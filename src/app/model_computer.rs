use super::cli_manager;
use super::common;
use clap::App;
use clap::ArgMatches;
use clap::{AppSettings, SubCommand};
use decdnnf_rs::ModelFinder;

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "compute-model";

impl<'a> super::command::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("returns a model of the formula")
            .setting(AppSettings::DisableVersion)
            .args(&common::args_input())
            .arg(common::arg_assumptions())
            .arg(cli_manager::logging_level_cli_arg())
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let assumptions = common::read_assumptions(&ddnnf, arg_matches)?;
        let model_finder = ModelFinder::new(&ddnnf);
        if let Some(model) = model_finder.find_model_under_assumptions(&assumptions) {
            println!("s SATISFIABLE");
            common::print_dimacs_model(&model);
        } else {
            println!("s UNSATISFIABLE");
        }
        Ok(())
    }
}
