use super::cli_manager;
use super::common;
use anyhow::Context;
use clap::App;
use clap::Arg;
use clap::ArgMatches;
use clap::{AppSettings, SubCommand};
use decdnnf_rs::D4Writer;
use decdnnf_rs::DecisionDNNFWriter;
use decdnnf_rs::EquivalentNodesMerging;
use decdnnf_rs::ModelCounter;
use decdnnf_rs::OrphanFinder;
use log::info;
use log::warn;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "reduce-formula";

const ARG_EXPORT_TO_FILE: &str = "EXPORT_TO_FILE";

impl<'a> super::command::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("reduces an input decision-DNNF")
            .setting(AppSettings::DisableVersion)
            .args(&common::args_input())
            .arg(cli_manager::logging_level_cli_arg())
            .arg(common::arg_input_cnf())
            .arg(
                Arg::with_name(ARG_EXPORT_TO_FILE)
                    .short("o")
                    .long("output-file")
                    .empty_values(false)
                    .multiple(false)
                    .help("export the decision-DNNF to a file instead of standard output"),
            )
    }

    fn execute(&self, arg_matches: &ArgMatches<'_>) -> anyhow::Result<()> {
        let mut ddnnf = common::read_input_ddnnf(arg_matches)?;
        let mut cnf = common::read_input_cnf(arg_matches)?;
        if cnf.n_vars() != ddnnf.n_vars() {
            warn!("the number of variables differs between the CNF and the d-DNNF; setting the maximal value for both");
            let n_vars = usize::max(cnf.n_vars(), ddnnf.n_vars());
            cnf.update_n_vars(n_vars);
            ddnnf.update_n_vars(n_vars);
        }
        info!("starting formula reduction");
        let n_eq_children_removed = EquivalentNodesMerging::merge_equivalent_ancestors(&mut ddnnf);
        info!("removed {n_eq_children_removed} equivalent child nodes");
        let model_counter = ModelCounter::new(&ddnnf, true);
        let model_reducer = EquivalentNodesMerging::search(&model_counter, &cnf)?;
        model_reducer.replace_in_formula(&mut ddnnf);
        info!(
            "removed {} equivalent nodes",
            model_reducer.n_equivalences()
        );
        let orphan_reducer = OrphanFinder::search(&ddnnf)
            .expect("formula has cycle (bug in model_reducer.replace_in_formula ?)");
        orphan_reducer.remove_from_formula(&mut ddnnf);
        info!(
            "removed {} orphan nodes",
            orphan_reducer.orphans_nodes().len()
        );
        info!("number of nodes after reduction: {}", ddnnf.n_nodes());
        info!("number of edges after reduction: {}", ddnnf.n_edges());
        let (str_out, unbuffered_out): (String, Box<dyn Write>) =
            match arg_matches.value_of(ARG_EXPORT_TO_FILE) {
                None => ("standard output".to_string(), Box::new(std::io::stdout())),
                Some(path) => {
                    let file = File::create(path).context("while creating the output file")?;
                    let str_path = std::fs::canonicalize(PathBuf::from(path))
                        .with_context(|| format!(r#"while opening file "{path}""#))?;
                    (format!("{}", str_path.display()), Box::new(file))
                }
            };
        info!("writing decision-DNNF to {str_out}");
        let mut out = BufWriter::new(unbuffered_out);
        D4Writer.write(&mut out, &ddnnf)
    }
}
