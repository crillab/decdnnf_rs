use anyhow::{anyhow, Context, Result};
use clap::{Arg, ArgMatches};
use decdnnf_rs::{D4Reader, DecisionDNNF, DecisionDNNFChecker, Literal};
use log::{info, warn};
use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
    time::SystemTime,
};

const ARG_INPUT: &str = "ARG_INPUT";
const ARG_N_VARS: &str = "ARG_N_VARS";
const ARG_DO_NOT_CHECK_DDNNF: &str = "ARG_DO_NOT_CHECK_DDNNF";

pub(crate) fn args_input<'a>() -> Vec<Arg<'a, 'a>> {
    vec![
        Arg::with_name(ARG_INPUT)
            .short("i")
            .long("input")
            .empty_values(false)
            .multiple(false)
            .help("the input file that contains the Decision-DNNF formula")
            .required(true),
        Arg::with_name(ARG_N_VARS)
            .long("n-vars")
            .empty_values(false)
            .multiple(false)
            .help(
                "sets the number of variables (must be higher are equal to the highest variable index)",
            ),
        Arg::with_name(ARG_DO_NOT_CHECK_DDNNF)
            .long("do-not-check")
            .takes_value(false)
            .help("do not check the correctness of the input Decision-DNNF"),
    ]
}

pub(crate) fn read_input_ddnnf(arg_matches: &ArgMatches<'_>) -> Result<DecisionDNNF> {
    log_time_for_step("Decision-DNNF reading", || {
        read_input_ddnnf_step(arg_matches)
    })
}

pub(crate) fn read_input_ddnnf_step(arg_matches: &ArgMatches<'_>) -> Result<DecisionDNNF> {
    let input_file_canonicalized = realpath_from_arg(arg_matches, ARG_INPUT)?;
    info!("reading input file {:?}", input_file_canonicalized);
    let file_reader = BufReader::new(File::open(input_file_canonicalized)?);
    let mut reader = D4Reader::default();
    let do_not_check = arg_matches.is_present(ARG_DO_NOT_CHECK_DDNNF);
    if do_not_check {
        info!("skipping input formula correctness checks; an incorrect input formula may lead to an undefined behavior");
        reader.set_do_not_check(true);
    }
    let mut ddnnf = reader
        .read(file_reader)
        .context("while parsing the input Decision-DNNF")?;
    if let Some(str_n) = arg_matches.value_of(ARG_N_VARS) {
        let n = str::parse::<usize>(str_n)
            .context("while parsing the number of variables provided on the command line")?;
        ddnnf.update_n_vars(n);
    }
    if !do_not_check {
        let checking_data = DecisionDNNFChecker::check(&ddnnf);
        for w in checking_data.warnings() {
            warn!("{w}");
        }
        if let Some(e) = checking_data.error() {
            return Err(anyhow!("{e}"));
        }
    }
    info!("number of variables: {}", ddnnf.n_vars());
    info!("number of nodes: {}", ddnnf.n_nodes());
    info!("number of edges: {}", ddnnf.n_edges());
    Ok(ddnnf)
}

fn realpath_from_arg(arg_matches: &ArgMatches<'_>, arg: &str) -> Result<PathBuf> {
    let file_path = arg_matches.value_of(arg).unwrap();
    fs::canonicalize(PathBuf::from(file_path))
        .with_context(|| format!(r#"while opening file "{file_path}""#))
}

pub(crate) fn print_dimacs_model(model: &[Literal]) {
    print!("v");
    for l in model {
        print!(" {l}");
    }
    println!(" 0");
}

pub(crate) fn log_time_for_step<F, T>(step_name: &str, step: F) -> T
where
    F: FnOnce() -> T,
{
    let start_time = SystemTime::now();
    info!("starting {step_name}");
    let result = step();
    info!("{step_name} took {:?}", start_time.elapsed().unwrap());
    result
}
