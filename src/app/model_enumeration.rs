use super::common;
use crusti_app_helper::{info, App, AppSettings, Arg, SubCommand};
use decdnnf_rs::{
    BottomUpTraversal, CheckingVisitor, DecisionDNNF, Literal, ModelEnumerator, ModelFinder,
};
use std::io::{BufWriter, StdoutLock, Write};

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
    let ddnnf = load_ddnnf(arg_matches)?;
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
    Ok(())
}

fn enum_decision_tree(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
    let ddnnf = load_ddnnf(arg_matches)?;
    let mut model_writer = ModelWriter::new(
        ddnnf.n_vars(),
        arg_matches.is_present(ARG_COMPACT_FREE_VARS),
        arg_matches.is_present(ARG_DO_NOT_PRINT),
    );
    let model_finder = ModelFinder::new(&ddnnf);
    if let Some(model) = model_finder.find_model() {
        enum_decision_tree_from(&model_finder, &mut vec![], &model, &mut model_writer);
    }
    model_writer.finalize();
    Ok(())
}

fn enum_decision_tree_from(
    model_finder: &ModelFinder,
    assumptions: &mut Vec<Literal>,
    last_model: &[Literal],
    model_writer: &mut ModelWriter,
) {
    if assumptions.len() == last_model.len() {
        model_writer.write_model_no_opt(last_model);
        return;
    }
    let new_var_in_model = *last_model
        .iter()
        .find(|l| l.var_index() == assumptions.len())
        .unwrap();
    assumptions.push(new_var_in_model);
    enum_decision_tree_from(model_finder, assumptions, last_model, model_writer);
    assumptions.pop();
    assumptions.push(new_var_in_model.flip());
    if let Some(model) = model_finder.find_model_under_assumptions(assumptions) {
        enum_decision_tree_from(model_finder, assumptions, &model, model_writer);
    }
    assumptions.pop();
}

fn load_ddnnf(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<DecisionDNNF> {
    let ddnnf = common::read_input_ddnnf(arg_matches)?;
    let traversal_visitor = Box::<CheckingVisitor>::default();
    let traversal_engine = BottomUpTraversal::new(traversal_visitor);
    let checking_data = traversal_engine.traverse(&ddnnf);
    common::print_warnings_and_errors(&checking_data)?;
    Ok(ddnnf)
}

struct ModelWriter {
    pattern: Vec<u8>,
    sign_location: Vec<usize>,
    buf: BufWriter<StdoutLock<'static>>,
    n_enumerated: usize,
    n_models: usize,
    compact_display: bool,
    do_not_print: bool,
}

impl ModelWriter {
    fn new(n_vars: usize, compact_display: bool, do_not_print: bool) -> Self {
        let mut sign_location = Vec::with_capacity(n_vars);
        let mut pattern = Vec::new();
        pattern.push(b'v');
        for i in 1..=n_vars {
            pattern.push(b' ');
            sign_location.push(pattern.len());
            pattern.push(b' ');
            pattern.extend_from_slice(format!("{i}").as_bytes());
        }
        pattern.extend_from_slice(" 0 \n".as_bytes());
        Self {
            pattern,
            sign_location,
            buf: BufWriter::with_capacity(128 * 1024, std::io::stdout().lock()),
            n_enumerated: 0,
            n_models: 0,
            compact_display,
            do_not_print,
        }
    }

    fn write_model_ordered(&mut self, model: &[Option<Literal>]) {
        self.n_enumerated += 1;
        if self.do_not_print {
            self.n_models += 1 << model.iter().filter(|opt| opt.is_none()).count();
            return;
        }
        let mut current_n_models = 1;
        model
            .iter()
            .zip(self.sign_location.iter())
            .for_each(|(opt_l, o)| {
                if let Some(l) = opt_l {
                    if l.polarity() {
                        self.pattern[*o] = b' ';
                    } else {
                        self.pattern[*o] = b'-';
                    }
                } else {
                    self.pattern[*o] = b'*';
                    current_n_models <<= 1;
                }
            });
        let _ = self.buf.write_all(&self.pattern);
        self.n_models += current_n_models;
    }

    fn write_model_no_opt(&mut self, model: &[Literal]) {
        self.n_enumerated += 1;
        self.n_models += 1;
        if self.do_not_print {
            return;
        }
        for l in model {
            if l.polarity() {
                self.pattern[self.sign_location[l.var_index()]] = b' ';
            } else {
                self.pattern[self.sign_location[l.var_index()]] = b'-';
            }
        }
        let _ = self.buf.write_all(&self.pattern);
    }

    fn finalize(mut self) {
        self.buf.flush().unwrap();
        if self.compact_display {
            info!(
                "enumerated {} compact models corresponding to {} models",
                self.n_enumerated, self.n_models
            );
        } else {
            info!("enumerated {} models", self.n_enumerated);
        }
    }
}
