use super::cli_manager;
use super::common;
use clap::App;
use clap::ArgMatches;
use clap::{AppSettings, Arg, SubCommand};
use decdnnf_rs::DecisionDNNFChecker;
use decdnnf_rs::{Literal, ModelFinder};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "compute-model";

const ARG_ASSUMPTIONS: &str = "ARG_ASSUMPTIONS";

impl<'a> super::command::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("returns a model of the formula")
            .setting(AppSettings::DisableVersion)
            .arg(common::arg_input_var())
            .arg(common::arg_n_vars())
            .arg(
                Arg::with_name(ARG_ASSUMPTIONS)
                    .short("a")
                    .long("assumptions")
                    .empty_values(false)
                    .multiple(false)
                    .allow_hyphen_values(true)
                    .help("sets some assumptions as a string of blank separated DIMACS literals"),
            )
            .arg(cli_manager::logging_level_cli_arg())
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let checking_data = DecisionDNNFChecker::check(&ddnnf);
        common::print_warnings_and_errors(&checking_data)?;
        let assumptions = if let Some(str_assumptions) = arg_matches.value_of(ARG_ASSUMPTIONS) {
            str_assumptions
                .split_whitespace()
                .map(|s| str::parse::<isize>(s).map(Literal::from))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
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
