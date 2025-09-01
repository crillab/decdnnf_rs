use std::str::FromStr;

use crate::app::model_writer::ModelWriter;

use super::cli_manager;
use super::common;
use anyhow::Context;
use clap::App;
use clap::ArgMatches;
use clap::{AppSettings, Arg, SubCommand};
use decdnnf_rs::DirectAccessEngine;
use decdnnf_rs::Literal;
use decdnnf_rs::ModelCounter;
use decdnnf_rs::OrderedDirectAccessEngine;
use log::info;
use rug::Integer;

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "direct-access";

const ARG_INDEX: &str = "ARG_INDEX";
pub(crate) const ARG_LEXICOGRAPHIC_ORDER: &str = "ARG_LEXICOGRAPHIC_ORDER";

impl<'a> super::command::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about(
                "returns the model at a given index in the ordered list of models of the formula",
            )
            .setting(AppSettings::DisableVersion)
            .args(&common::args_input())
            .arg(cli_manager::logging_level_cli_arg())
            .arg(arg_lexicographic_order())
            .arg(
                Arg::with_name(ARG_INDEX)
                    .short("n")
                    .long("index")
                    .required(true)
                    .empty_values(false)
                    .multiple(false)
                    .help("sets the index of the model"),
            )
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let ddnnf = common::read_input_ddnnf(arg_matches)?;
        let index = Integer::from_str(arg_matches.value_of(ARG_INDEX).unwrap())
            .context("while parsing the model index")?;
        let model_counter = ModelCounter::new(&ddnnf, false);
        let n_models = model_counter.global_count();
        info!("formula has {n_models} models");
        let mut model_writer = ModelWriter::new_locked(ddnnf.n_vars(), false, false);
        let engine = direct_access_engine(arg_matches, &model_counter);
        let model = engine(index);
        model_writer.write_model_ordered(&model);
        model_writer.finalize();
        Ok(())
    }
}

pub(crate) fn arg_lexicographic_order<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_LEXICOGRAPHIC_ORDER)
        .long("lexicographic-order")
        .takes_value(false)
        .help("applies a lexicographic order on the models")
}

pub(crate) fn direct_access_engine<'a>(
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
