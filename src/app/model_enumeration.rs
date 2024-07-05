use super::{common, model_writer::ModelWriter};
use crusti_app_helper::{info, App, AppSettings, Arg, SubCommand};
use decdnnf_rs::{Literal, ModelEnumerator, ModelFinder};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-enumeration";

const ARG_COMPACT_FREE_VARS: &str = "ARG_COMPACT_FREE_VARS";
const ARG_DECISION_TREE: &str = "ARG_DECISION_TREE";
const ARG_DO_NOT_PRINT: &str = "ARG_DO_NOT_PRINT";

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
            .arg(
                Arg::with_name(ARG_DECISION_TREE)
                    .long("decision-tree")
                    .takes_value(false)
                    .conflicts_with(ARG_COMPACT_FREE_VARS)
                    .help("enumerate by building a decision tree (should be less efficient)"),
            )
            .arg(
                Arg::with_name(ARG_DO_NOT_PRINT)
                    .long("do-not-print")
                    .takes_value(false)
                    .help("do not print the models (for testing purpose)"),
            )
    }

    fn execute(&self, arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
        if arg_matches.is_present(ARG_DECISION_TREE) {
            enum_decision_tree(arg_matches)
        } else {
            enum_default(arg_matches)
        }
    }
}

fn enum_default(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
    let ddnnf = common::read_and_check_input_ddnnf(arg_matches)?;
    let mut model_writer = ModelWriter::new(
        ddnnf.n_vars(),
        arg_matches.is_present(ARG_COMPACT_FREE_VARS),
        arg_matches.is_present(ARG_DO_NOT_PRINT),
    );
    let mut model_iterator =
        ModelEnumerator::new(&ddnnf, arg_matches.is_present(ARG_COMPACT_FREE_VARS));
    while let Some(model) = model_iterator.compute_next_model() {
        model_writer.write_model_ordered(model);
    }
    model_writer.finalize();
    write_summary(&model_writer);
    Ok(())
}

fn enum_decision_tree(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
    let ddnnf = common::read_and_check_input_ddnnf(arg_matches)?;
    let mut model_writer = ModelWriter::new(
        ddnnf.n_vars(),
        arg_matches.is_present(ARG_COMPACT_FREE_VARS),
        arg_matches.is_present(ARG_DO_NOT_PRINT),
    );
    let model_finder = ModelFinder::new(&ddnnf);
    let mut assumptions = Vec::with_capacity(ddnnf.n_vars());
    let mut stack = Vec::with_capacity(ddnnf.n_vars() << 1);
    let mut last_model = vec![];
    let update_stack = |m: &[Literal], i, stack: &mut Vec<(bool, Literal)>| {
        let shortcut_lit = *m.iter().find(|l| l.var_index() == i).unwrap();
        stack.push((false, shortcut_lit.flip()));
        stack.push((true, shortcut_lit));
    };
    if let Some(ref mut model) = model_finder.find_model() {
        std::mem::swap(&mut last_model, model);
        if ddnnf.n_vars() == 0 {
            model_writer.write_model_no_opt(&[]);
        } else {
            update_stack(&last_model, 0, &mut stack);
        }
    }
    while let Some((shortcut, lit)) = stack.pop() {
        assumptions.truncate(lit.var_index());
        assumptions.push(lit);
        if shortcut {
            if assumptions.len() == ddnnf.n_vars() {
                model_writer.write_model_no_opt(&last_model);
            } else {
                update_stack(&last_model, assumptions.len(), &mut stack);
            }
        } else {
            let opt_model = model_finder.find_model_under_assumptions(&assumptions);
            if opt_model.is_some() {
                let mut new_model = opt_model.unwrap();
                std::mem::swap(&mut last_model, &mut new_model);
                if assumptions.len() == ddnnf.n_vars() {
                    model_writer.write_model_no_opt(&last_model);
                } else {
                    update_stack(&last_model, assumptions.len(), &mut stack);
                }
            }
        }
    }
    model_writer.finalize();
    write_summary(&model_writer);
    Ok(())
}

fn write_summary(model_writer: &ModelWriter) {
    if model_writer.compact_display() {
        info!(
            "enumerated {} compact models corresponding to {} models",
            model_writer.n_enumerated(),
            model_writer.n_models()
        );
    } else {
        info!("enumerated {} models", model_writer.n_enumerated());
    }
}
