use std::{io::Write, sync::Mutex};
use super::{common, model_writer::ModelWriter};
use anyhow::Context;
use crusti_app_helper::{info, App, AppSettings, Arg, SubCommand};
use decdnnf_rs::{DirectAccessEngine, Literal, ModelCounter, ModelEnumerator, ModelFinder};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rug::Integer;

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-enumeration";

const ARG_COMPACT_FREE_VARS: &str = "ARG_COMPACT_FREE_VARS";
const ARG_DECISION_TREE: &str = "ARG_DECISION_TREE";
const ARG_DO_NOT_PRINT: &str = "ARG_DO_NOT_PRINT";
const ARG_THREADS: &str = "ARG_THREADS";

const MT_BATCH_SIZE: usize = 1 << 10;

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
            .arg(
                Arg::with_name(ARG_THREADS)
                    .short("t")
                    .long("threads")
                    .empty_values(false)
                    .multiple(false)
                    .help("sets the maximal number of threads to use"),
            )
    }

    fn execute(&self, arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
        if arg_matches.is_present(ARG_DECISION_TREE) {
            enum_decision_tree(arg_matches)
        } else if arg_matches.is_present(ARG_THREADS) {
            enum_default_parallel(arg_matches)
        } else {
            enum_default(arg_matches)
        }
    }
}

fn enum_default(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
    let ddnnf = common::read_and_check_input_ddnnf(arg_matches)?;
    let mut model_writer = ModelWriter::new_locked(
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

fn enum_default_parallel(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
    let ddnnf = common::read_and_check_input_ddnnf(arg_matches)?;
    let model_counter = ModelCounter::new(&ddnnf);
    let n_models = model_counter.n_models();
    let next_min_bound = Mutex::new(Integer::ZERO);
    let writers_n_enumerated = Mutex::new(Integer::ZERO);
    let writers_n_models = Mutex::new(Integer::ZERO);
    let n_threads = str::parse::<usize>(arg_matches.value_of(ARG_THREADS).unwrap())
        .context("while parsing the number of threads provided on the command line")?;
    (0..n_threads).into_par_iter().for_each(|_| {
        let mut model_writer = ModelWriter::new_unlocked(
            ddnnf.n_vars(),
            arg_matches.is_present(ARG_COMPACT_FREE_VARS),
            arg_matches.is_present(ARG_DO_NOT_PRINT),
        );
        let mut model_iterator =
            ModelEnumerator::new(&ddnnf, arg_matches.is_present(ARG_COMPACT_FREE_VARS));
        let direct_access_engine = DirectAccessEngine::new(&model_counter);
        loop {
            let mut lock = next_min_bound.lock().unwrap();
            let mut min_bound = lock.clone();
            if &min_bound == n_models {
                std::mem::drop(lock);
                break;
            }
            let mut next_min_bound = Integer::from(&min_bound + MT_BATCH_SIZE);
            if &next_min_bound > n_models {
                next_min_bound.clone_from(n_models);
            }
            lock.clone_from(&next_min_bound);
            std::mem::drop(lock);
            let mut model = model_iterator
                .jump_to(&direct_access_engine, min_bound.clone())
                .unwrap();
            loop {
                model_writer.write_model_ordered(model);
                min_bound += 1;
                if min_bound == next_min_bound {
                    break;
                }
                model = model_iterator.compute_next_model().unwrap();
            }
        }
        model_writer.finalize();
        *writers_n_enumerated.lock().unwrap() += model_writer.n_enumerated();
        *writers_n_models.lock().unwrap() += model_writer.n_models();
    });
    write_summary_for(
        arg_matches.is_present(ARG_COMPACT_FREE_VARS),
        &writers_n_enumerated.lock().unwrap(),
        &writers_n_models.lock().unwrap(),
    );
    Ok(())
}

fn enum_decision_tree(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
    let ddnnf = common::read_and_check_input_ddnnf(arg_matches)?;
    let mut model_writer = ModelWriter::new_locked(
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

fn write_summary<W>(model_writer: &ModelWriter<W>)
where
    W: Write,
{
    write_summary_for(
        model_writer.compact_display(),
        model_writer.n_enumerated(),
        model_writer.n_models(),
    );
}

fn write_summary_for(compact_display: bool, n_enumerated: &Integer, n_models: &Integer) {
    if compact_display {
        info!(
            "enumerated {} compact models corresponding to {} models",
            n_enumerated, n_models
        );
    } else {
        info!("enumerated {} models", n_enumerated);
    }
}
