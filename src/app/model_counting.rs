use anyhow::{anyhow, Context, Result};
use crusti_app_helper::{info, App, AppSettings, Arg, ArgMatches, SubCommand};
use decdnnf_rs::{
    BiBottomUpVisitor, BottomUpTraversal, CheckingVisitor, D4Reader, ModelCountingVisitor,
};
use std::{
    fs::{self, File},
    io::BufReader,
    path::PathBuf,
};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-counting";

const ARG_INPUT: &str = "ARG_INPUT";
const ARG_N_VARS: &str = "ARG_N_VARS";

impl<'a> crusti_app_helper::Command<'a> for Command {
    fn name(&self) -> &str {
        CMD_NAME
    }

    fn clap_subcommand(&self) -> App<'a, 'a> {
        SubCommand::with_name(CMD_NAME)
            .about("counts the models of the formula")
            .setting(AppSettings::DisableVersion)
            .arg(
                Arg::with_name(ARG_INPUT)
                    .short("i")
                    .long("input")
                    .empty_values(false)
                    .multiple(false)
                    .help("the input file that contains the Decision-DNNF formula")
                    .required(true),
            )
            .arg(
                Arg::with_name(ARG_N_VARS)
                    .long("n-vars")
                    .empty_values(false)
                    .multiple(false)
                    .help("sets the number of variables (must be higher are equal to the highest variable index)"),
            )
            .arg(crusti_app_helper::logging_level_cli_arg())
    }

    fn execute(&self, arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
        let file_reader = create_input_file_reader(arg_matches)?;
        let mut ddnnf =
            D4Reader::read(file_reader).context("while parsing the input Decision-DNNF")?;
        if let Some(str_n) = arg_matches.value_of(ARG_N_VARS) {
            let n = str::parse::<usize>(str_n)
                .context("while parsing the number of variables provided on the command line")?;
            ddnnf.update_n_vars(n);
        }
        let traversal_visitor = BiBottomUpVisitor::new(
            Box::<CheckingVisitor>::default(),
            Box::<ModelCountingVisitor>::default(),
        );
        let traversal_engine = BottomUpTraversal::new(Box::new(traversal_visitor));
        let (checking_data, model_counting_data) = traversal_engine.traverse(&ddnnf);
        if let Some(e) = checking_data.get_error() {
            return Err(anyhow!("{e}"));
        }
        println!("{}", model_counting_data.n_models());
        Ok(())
    }
}

fn create_input_file_reader(arg_matches: &ArgMatches<'_>) -> Result<BufReader<File>> {
    let input_file_canonicalized = realpath_from_arg(arg_matches, ARG_INPUT)?;
    info!("reading input file {:?}", input_file_canonicalized);
    Ok(BufReader::new(File::open(input_file_canonicalized)?))
}

fn realpath_from_arg(arg_matches: &ArgMatches<'_>, arg: &str) -> Result<PathBuf> {
    let file_path = arg_matches.value_of(arg).unwrap();
    fs::canonicalize(PathBuf::from(file_path))
        .with_context(|| format!(r#"while opening file "{file_path}""#))
}
