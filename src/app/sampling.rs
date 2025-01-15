use super::{cli_manager, common};
use crate::app::model_writer::ModelWriter;
use anyhow::Context;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use decdnnf_rs::{DirectAccessEngine, Literal, ModelCounter, OrderedDirectAccessEngine};
use log::info;
use rug::{rand::RandState, Integer};
use rustc_hash::FxHashMap;
use std::str::FromStr;

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "sampling";

const ARG_LIMIT: &str = "ARG_LIMIT";
const ARG_DO_NOT_PRINT: &str = "ARG_DO_NOT_PRINT";
const ARG_LEXICOGRAPHIC_ORDER: &str = "ARG_LEXICOGRAPHIC_ORDER";
const ARG_SEED: &str = "ARG_SEED";

impl<'a> super::command::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("performs a uniform sampling among the models of the formula")
            .setting(AppSettings::DisableVersion)
            .args(&common::args_input())
            .arg(cli_manager::logging_level_cli_arg())
            .arg(
                Arg::with_name(ARG_LIMIT)
                    .short("l")
                    .long("limit")
                    .empty_values(false)
                    .multiple(false)
                    .help("sets the maximal number of models to print"),
            )
            .arg(
                Arg::with_name(ARG_LEXICOGRAPHIC_ORDER)
                    .long("lexicographic-order")
                    .takes_value(false)
                    .help("applies a lexicographic order on the models"),
            )
            .arg(
                Arg::with_name(ARG_SEED)
                    .short("s")
                    .long("seed")
                    .empty_values(false)
                    .multiple(false)
                    .help("sets the random seed"),
            )
            .arg(
                Arg::with_name(ARG_DO_NOT_PRINT)
                    .long("do-not-print")
                    .takes_value(false)
                    .help("do not print the models (for testing purpose)"),
            )
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let mut rand = RandState::new_mersenne_twister();
        let seed = if let Some(str_n) = arg_matches.value_of(ARG_SEED) {
            Integer::from_str(str_n).context("while parsing the random seed")?
        } else {
            Integer::from(usize::MAX).random_below(&mut rand)
        };
        info!("random seed is {seed}");
        rand.seed(&seed);
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let model_counter = ModelCounter::new(&ddnnf, false);
        let n_models = model_counter.global_count();
        info!("formula has {n_models} models");
        let mut n_samples = model_counter.global_count().clone();
        if let Some(str_n) = arg_matches.value_of(ARG_LIMIT) {
            let n = Integer::from_str(str_n)
                .context("while parsing the maximal number of models to print")?;
            if n_samples > n {
                n_samples = n;
            }
        }
        info!("sampling {n_samples} samples");
        let engine = direct_access_engine(arg_matches, &model_counter);
        let mut counter = Integer::ZERO;
        let mut swapped: FxHashMap<Integer, Integer> = FxHashMap::default();
        let mut model_writer = ModelWriter::new_locked(
            ddnnf.n_vars(),
            false,
            arg_matches.is_present(ARG_DO_NOT_PRINT),
        );
        while counter < n_samples {
            let mut bound = Integer::from(n_models - &counter);
            let rand_index = Integer::from(bound.random_below_ref(&mut rand));
            bound -= 1;
            let last_value = swapped.get(&bound).unwrap_or(&bound).to_owned();
            let rand_value = swapped
                .insert(rand_index, last_value)
                .unwrap_or_else(|| Integer::from(bound.random_below_ref(&mut rand)));
            let model = engine(rand_value);
            model_writer.write_model_ordered(&model);
            counter += 1;
        }
        model_writer.finalize();
        Ok(())
    }
}

fn direct_access_engine<'a>(
    arg_matches: &ArgMatches<'_>,
    model_counter: &'a ModelCounter,
) -> Box<dyn Fn(Integer) -> Vec<Option<Literal>> + 'a> {
    if arg_matches.is_present(ARG_LEXICOGRAPHIC_ORDER) {
        let order = (1..=model_counter.ddnnf().n_vars())
            .map(|i| Literal::from(-isize::try_from(i).unwrap()))
            .collect::<Vec<_>>();
        let engine = OrderedDirectAccessEngine::new(model_counter.ddnnf(), order).unwrap();
        Box::new(move |i| {
            engine
                .model(i)
                .unwrap()
                .iter()
                .map(|l| Some(*l))
                .collect::<Vec<_>>()
        })
    } else {
        let engine = DirectAccessEngine::new(model_counter);
        Box::new(move |i| engine.model(i).unwrap())
    }
}
