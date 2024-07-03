use super::common;
use crusti_app_helper::{App, AppSettings, SubCommand};
use decdnnf_rs::ModelCounter;

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-counting";

impl<'a> crusti_app_helper::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("counts the models of the formula")
            .setting(AppSettings::DisableVersion)
            .arg(common::arg_input_var())
            .arg(common::arg_n_vars())
            .arg(crusti_app_helper::logging_level_cli_arg())
    }

    fn execute(&self, arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_and_check_input_ddnnf(arg_matches)?;
        let model_counter = ModelCounter::new(&ddnnf);
        println!("{}", model_counter.n_models());
        Ok(())
    }
}
