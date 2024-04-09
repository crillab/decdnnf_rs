use super::common;
use anyhow::anyhow;
use crusti_app_helper::{info, App, AppSettings, SubCommand};
use decdnnf_rs::{BottomUpTraversal, CheckingVisitor, ModelIterator};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-enumeration";

impl<'a> crusti_app_helper::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("enumerates the models of the formula")
            .setting(AppSettings::DisableVersion)
            .arg(common::arg_input_var())
            .arg(common::arg_n_vars())
            .arg(crusti_app_helper::logging_level_cli_arg())
    }

    fn execute(&self, arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let traversal_visitor = Box::<CheckingVisitor>::default();
        let traversal_engine = BottomUpTraversal::new(traversal_visitor);
        let checking_data = traversal_engine.traverse(&ddnnf);
        if let Some(e) = checking_data.get_error() {
            return Err(anyhow!("{e}"));
        }
        let mut n_models = 0;
        let model_iterator = ModelIterator::new(&ddnnf);
        for model in model_iterator {
            n_models += 1;
            common::print_dimacs_model(&model);
        }
        info!("enumerated {n_models} models");
        Ok(())
    }
}
