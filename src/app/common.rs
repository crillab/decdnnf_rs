use anyhow::{Context, Result};
use crusti_app_helper::{info, Arg, ArgMatches};
use decdnnf_rs::{D4Reader, DecisionDNNF, Literal};
use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

const ARG_INPUT: &str = "ARG_INPUT";

pub(crate) fn arg_input_var<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_INPUT)
        .short("i")
        .long("input")
        .empty_values(false)
        .multiple(false)
        .help("the input file that contains the Decision-DNNF formula")
        .required(true)
}

const ARG_N_VARS: &str = "ARG_N_VARS";

pub(crate) fn arg_n_vars<'a>() -> Arg<'a, 'a> {
    Arg::with_name(ARG_N_VARS)
        .long("n-vars")
        .empty_values(false)
        .multiple(false)
        .help(
            "sets the number of variables (must be higher are equal to the highest variable index)",
        )
}

pub(crate) fn read_input_ddnnf(
    arg_matches: &crusti_app_helper::ArgMatches<'_>,
) -> Result<DecisionDNNF> {
    let file_reader = create_input_file_reader(arg_matches)?;
    let mut ddnnf = D4Reader::read(file_reader).context("while parsing the input Decision-DNNF")?;
    if let Some(str_n) = arg_matches.value_of(ARG_N_VARS) {
        let n = str::parse::<usize>(str_n)
            .context("while parsing the number of variables provided on the command line")?;
        ddnnf.update_n_vars(n);
    }
    Ok(ddnnf)
}

pub(crate) fn create_input_file_reader(arg_matches: &ArgMatches<'_>) -> Result<BufReader<File>> {
    let input_file_canonicalized = realpath_from_arg(arg_matches, ARG_INPUT)?;
    info!("reading input file {:?}", input_file_canonicalized);
    Ok(BufReader::new(File::open(input_file_canonicalized)?))
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
