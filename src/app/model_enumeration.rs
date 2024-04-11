use super::common;
use crusti_app_helper::{info, App, AppSettings, SubCommand};
use decdnnf_rs::{BottomUpTraversal, CheckingVisitor, ModelEnumerator};
use std::io::{BufWriter, Write};

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
        common::print_warnings_and_errors(&checking_data)?;
        let mut n_models = 0;
        let mut model_iterator = ModelEnumerator::new(&ddnnf);
        let stdout = std::io::stdout();
        let lock = stdout.lock();
        let mut buf = BufWriter::with_capacity(128 * 1024, lock);
        let mut pattern = Vec::new();
        let mut sign_location = Vec::with_capacity(ddnnf.n_vars());
        pattern.push(b'v');
        for i in 1..=ddnnf.n_vars() {
            pattern.push(b' ');
            sign_location.push(pattern.len());
            pattern.push(b' ');
            pattern.extend_from_slice(format!("{i}").as_bytes());
        }
        pattern.extend_from_slice(" 0 \n".as_bytes());
        while let Some(model) = model_iterator.compute_next_model() {
            n_models += 1;
            model.iter().zip(sign_location.iter()).for_each(|(l,o)| {
                if l.polarity() {
                    pattern[*o] = b' ';
                } else {
                    pattern[*o] = b'-';
                }
            });
            let _ = buf.write_all(&pattern);
        }
        info!("enumerated {n_models} models");
        Ok(())
    }
}
