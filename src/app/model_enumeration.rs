use super::common;
use crusti_app_helper::{info, App, AppSettings, Arg, SubCommand};
use decdnnf_rs::{BottomUpTraversal, CheckingVisitor, ModelEnumerator};
use std::io::{BufWriter, Write};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-enumeration";

const ARG_COMPACT_FREE_VARS: &str = "ARG_COMPACT_FREE_VARS";

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
            .arg(
                Arg::with_name(ARG_COMPACT_FREE_VARS)
                    .short("c")
                    .long("compact-free-vars")
                    .takes_value(false)
                    .help("compact models with free variables"),
            )
    }

    fn execute(&self, arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let traversal_visitor = Box::<CheckingVisitor>::default();
        let traversal_engine = BottomUpTraversal::new(traversal_visitor);
        let checking_data = traversal_engine.traverse(&ddnnf);
        common::print_warnings_and_errors(&checking_data)?;
        let mut n_enumerated = 0;
        let mut n_models = 0;
        let mut model_iterator = ModelEnumerator::new(&ddnnf);
        if arg_matches.is_present(ARG_COMPACT_FREE_VARS) {
            model_iterator.elude_free_vars(true);
        }
        let mut sign_location = Vec::with_capacity(ddnnf.n_vars());
        let mut pattern = Vec::new();
        pattern.push(b'v');
        for i in 1..=ddnnf.n_vars() {
            pattern.push(b' ');
            sign_location.push(pattern.len());
            pattern.push(b' ');
            pattern.extend_from_slice(format!("{i}").as_bytes());
        }
        pattern.extend_from_slice(" 0 \n".as_bytes());
        let stdout = std::io::stdout();
        let lock = stdout.lock();
        let mut buf = BufWriter::with_capacity(128 * 1024, lock);
        while let Some(model) = model_iterator.compute_next_model() {
            n_enumerated += 1;
            let mut current_n_models = 1;
            model
                .iter()
                .zip(sign_location.iter())
                .for_each(|(opt_l, o)| {
                    if let Some(l) = opt_l {
                        if l.polarity() {
                            pattern[*o] = b' ';
                        } else {
                            pattern[*o] = b'-';
                        }
                    } else {
                        pattern[*o] = b'*';
                        current_n_models <<= 1;
                    }
                });
            let _ = buf.write_all(&pattern);
            n_models += current_n_models;
        }
        buf.flush().unwrap();
        if arg_matches.is_present(ARG_COMPACT_FREE_VARS) {
            info!("enumerated {n_enumerated} compact models corresponding to {n_models} models");
        } else {
            info!("enumerated {n_enumerated} models");
        }
        Ok(())
    }
}
