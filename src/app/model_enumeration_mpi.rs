use super::{common, model_writer::ModelWriter};
use anyhow::anyhow;
use crusti_app_helper::{info, App, AppSettings, Arg, SubCommand};
use decdnnf_rs::{DirectAccessEngine, ModelCounter, ModelEnumerator};
use mpi::{
    datatype::{DynBuffer, DynBufferMut},
    topology::SimpleCommunicator,
    traits::{Communicator, Destination, Source},
    Rank,
};
use rug::Integer;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Default)]
pub struct Command;

const CMD_NAME: &str = "model-enumeration-mpi";

const ARG_COMPACT_FREE_VARS: &str = "ARG_COMPACT_FREE_VARS";
const ARG_DO_NOT_PRINT: &str = "ARG_DO_NOT_PRINT";
const ARG_MPI: &str = "ARG_MPI";

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
                Arg::with_name(ARG_DO_NOT_PRINT)
                    .long("do-not-print")
                    .takes_value(false)
                    .help("do not print the models (for testing purpose)"),
            )
            .arg(
                Arg::with_name(ARG_MPI)
                    .long("mpi")
                    .takes_value(false)
                    .help("use MPI"),
            )
    }

    fn execute(&self, arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
        enum_default_mpi(arg_matches)
    }
}

const MPI_TAG_INTERVAL: i32 = 0;
const MPI_TAG_BOUND: i32 = 1;
const MPI_TAG_COUNT: i32 = 2;

fn enum_default_mpi(arg_matches: &crusti_app_helper::ArgMatches<'_>) -> anyhow::Result<()> {
    let universe = mpi::initialize().unwrap();
    let world = universe.world();
    let current_rank = world.rank();
    let n_ranks = world.size();
    info!("MPI rank {current_rank} out of {}", n_ranks - 1);
    if n_ranks < 2 {
        return Err(anyhow!("number of ranks must be at least 2"));
    }
    let ddnnf = common::read_and_check_input_ddnnf(arg_matches)?;
    let compact_display = arg_matches.is_present(ARG_COMPACT_FREE_VARS);
    let model_counter = ModelCounter::new(&ddnnf, compact_display);
    if current_rank == 0 {
        master_worker(&world, &model_counter, arg_matches);
    } else {
        computer_worker(&world, &model_counter, arg_matches);
    }
    Ok(())
}

fn master_worker(
    current_world: &SimpleCommunicator,
    model_counter: &ModelCounter,
    arg_matches: &crusti_app_helper::ArgMatches<'_>,
) {
    let n_models = model_counter.global_count();
    info!("formula has {n_models} models");
    let mut next_min_bound = Integer::ZERO;
    let mut workers_n_enumerated = Integer::ZERO;
    let mut workers_n_models = Integer::ZERO;
    let mut n_workers = current_world.size() - 1;
    loop {
        let (_, status) = current_world
            .any_process()
            .receive_with_tag::<usize>(MPI_TAG_INTERVAL);
        let sender = status.source_rank();
        let min_bound = next_min_bound.clone();
        if &min_bound >= n_models {
            current_world.process_at_rank(sender).send(&false);
            workers_n_enumerated +=
                get_deserializable::<Integer>(current_world, sender, MPI_TAG_COUNT);
            workers_n_models += get_deserializable::<Integer>(current_world, sender, MPI_TAG_COUNT);
            n_workers -= 1;
            if n_workers == 0 {
                break;
            }
        }
        next_min_bound = Integer::from(&min_bound + MT_BATCH_SIZE);
        if &next_min_bound > n_models {
            next_min_bound = Integer::from(n_models);
        }
        current_world.process_at_rank(sender).send(&true);
        send_serializable(current_world, sender, &min_bound, MPI_TAG_BOUND);
        send_serializable(current_world, sender, &next_min_bound, MPI_TAG_BOUND);
    }
    write_summary_for(
        arg_matches.is_present(ARG_COMPACT_FREE_VARS),
        &workers_n_enumerated,
        &workers_n_models,
    );
}

fn computer_worker(
    current_world: &SimpleCommunicator,
    model_counter: &ModelCounter,
    arg_matches: &crusti_app_helper::ArgMatches<'_>,
) {
    let compact_display = arg_matches.is_present(ARG_COMPACT_FREE_VARS);
    let mut model_writer = ModelWriter::new_locked(
        model_counter.ddnnf().n_vars(),
        compact_display,
        arg_matches.is_present(ARG_DO_NOT_PRINT),
    );
    let mut model_iterator = ModelEnumerator::new(model_counter.ddnnf(), compact_display);
    let direct_access_engine = DirectAccessEngine::new(model_counter);
    loop {
        current_world
            .process_at_rank(0)
            .send_with_tag(&0, MPI_TAG_INTERVAL);
        let (has_next, _) = current_world.process_at_rank(0).receive::<bool>();
        if has_next {
            let mut min_bound: Integer = get_deserializable(current_world, 0, MPI_TAG_BOUND);
            let max_bound: Integer = get_deserializable(current_world, 0, MPI_TAG_BOUND);
            let mut model = model_iterator
                .jump_to(&direct_access_engine, min_bound.clone())
                .unwrap();
            loop {
                model_writer.write_model_ordered(model);
                min_bound += 1;
                if min_bound == max_bound {
                    break;
                }
                model = model_iterator.compute_next_model().unwrap();
            }
        } else {
            send_serializable(current_world, 0, model_writer.n_enumerated(), MPI_TAG_COUNT);
            send_serializable(current_world, 0, model_writer.n_models(), MPI_TAG_COUNT);
            break;
        }
    }
    model_writer.finalize();
}

fn send_serializable<T>(current_world: &SimpleCommunicator, rank: Rank, integer: &T, tag: i32)
where
    T: ?Sized + Serialize,
{
    let data = bincode::serialize(integer).unwrap();
    let buffer = DynBuffer::new(data.as_slice());
    current_world
        .process_at_rank(rank)
        .send_with_tag(&buffer.len(), tag);
    current_world
        .process_at_rank(rank)
        .buffered_send_with_tag(&buffer, tag);
}

fn get_deserializable<T>(current_world: &SimpleCommunicator, rank: Rank, tag: i32) -> T
where
    T: DeserializeOwned,
{
    let (data_size, _) = current_world
        .process_at_rank(rank)
        .receive_with_tag::<usize>(tag);
    let mut data: Vec<u8> = vec![0; data_size];
    let mut buffer = DynBufferMut::new(&mut data);
    current_world
        .process_at_rank(rank)
        .receive_into_with_tag(&mut buffer, tag);
    let buffer_content: &[u8] = buffer.downcast().unwrap();
    bincode::deserialize_from(buffer_content).unwrap()
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
