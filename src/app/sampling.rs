use super::{cli_manager, common};
use crate::app::model_writer::ModelWriter;
use anyhow::Context;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use decdnnf_rs::{ModelCounter, ModelSampler};
use log::info;
use rug::{rand::RandState, Integer};
use std::{
    rc::Rc,
    str::FromStr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "sampling";

const ARG_LIMIT: &str = "ARG_LIMIT";
const ARG_DO_NOT_PRINT: &str = "ARG_DO_NOT_PRINT";
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
            .arg(common::arg_assumptions())
            .arg(cli_manager::logging_level_cli_arg())
            .arg(super::direct_access::arg_lexicographic_order())
            .arg(
                Arg::with_name(ARG_LIMIT)
                    .short("l")
                    .long("limit")
                    .empty_values(false)
                    .multiple(false)
                    .help("sets the maximal number of models to print"),
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
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
            rand.seed(&Integer::from(since_the_epoch.as_millis()));
            Integer::from(usize::MAX).random_below(&mut rand)
        };
        info!("random seed is {seed}");
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let mut model_counter = ModelCounter::new(&ddnnf, false);
        if let Some(a) = common::read_assumptions(&ddnnf, arg_matches)? {
            info!("user set {} assumptions", a.as_slice().len());
            model_counter.set_assumptions(Rc::new(a));
        } else {
            info!("user set 0 assumptions");
        }
        let n_models = model_counter.global_count();
        info!("formula has {n_models} models");
        let n_samples = if let Some(str_n) = arg_matches.value_of(ARG_LIMIT) {
            Integer::from_str(str_n)
                .context("while parsing the maximal number of models to print")?
        } else {
            Integer::from(n_models)
        };
        let mut sampler = ModelSampler::new(&model_counter, n_samples);
        info!("sampling {} samples", sampler.n_samples_remaining());
        sampler.set_seed(&seed);
        if arg_matches.is_present(super::direct_access::ARG_LEXICOGRAPHIC_ORDER) {
            sampler.set_lexicographic_order();
        }
        let mut model_writer = ModelWriter::new_locked(
            ddnnf.n_vars(),
            false,
            arg_matches.is_present(ARG_DO_NOT_PRINT),
        );
        while let Some(model) = sampler.compute_next_model() {
            model_writer.write_model_ordered(&model);
        }
        model_writer.finalize();
        Ok(())
    }
}
