use super::free_variables;
use crate::{
    core::{EdgeIndex, Node, NodeIndex},
    DecisionDNNF, Literal,
};
use rug::Integer;

/// A structure used to count the models of a [`DecisionDNNF`].
///
/// The algorithm takes a time polynomial in the size of the Decision-DNNF.
///
/// # Example
///
/// ```
/// use decdnnf_rs::{Counter, DecisionDNNF, ModelCounter};
///
/// fn count_models(ddnnf: &DecisionDNNF) {
///     let model_counter = ModelCounter::new(ddnnf);
///     println!("the formula has {} models", model_counter.global_count());
/// }
/// # count_models(&decdnnf_rs::D4Reader::read("t 1 0".as_bytes()).unwrap())
/// ```
pub struct ModelCounter<'a> {
    ddnnf: &'a DecisionDNNF,
    n_models: Vec<Option<Integer>>,
    or_free_vars: Vec<Vec<Vec<Literal>>>,
    root_free_vars: Vec<Literal>,
}

impl<'a> ModelCounter<'a> {
    /// Builds a new model counter given a formula.
    #[must_use]
    pub fn new(ddnnf: &'a DecisionDNNF) -> Self {
        let (or_free_vars, root_free_vars) = free_variables::compute(ddnnf);
        let mut n_models = vec![None; ddnnf.nodes().as_slice().len()];
        compute_models_from(
            ddnnf,
            &root_free_vars,
            &|index| or_free_vars[usize::from(index)].iter().map(Vec::len),
            NodeIndex::from(0),
            &mut n_models,
        );
        Self {
            ddnnf,
            n_models,
            or_free_vars,
            root_free_vars,
        }
    }
}

/// A structure used to count the paths of a [`DecisionDNNF`].
///
/// The algorithm takes a time polynomial in the size of the Decision-DNNF.
///
/// # Example
///
/// ```
/// use decdnnf_rs::{Counter, DecisionDNNF, PathCounter};
///
/// fn count_paths(ddnnf: &DecisionDNNF) {
///     let path_counter = PathCounter::new(ddnnf);
///     println!("the formula has {} paths", path_counter.global_count());
/// }
/// # count_paths(&decdnnf_rs::D4Reader::read("t 1 0".as_bytes()).unwrap())
/// ```
pub struct PathCounter<'a> {
    ddnnf: &'a DecisionDNNF,
    n_models: Vec<Option<Integer>>,
    or_free_vars: Vec<Vec<Vec<Literal>>>,
    root_free_vars: Vec<Literal>,
}

impl<'a> PathCounter<'a> {
    /// Builds a new path counter given a formula.
    #[must_use]
    pub fn new(ddnnf: &'a DecisionDNNF) -> Self {
        let (or_free_vars, root_free_vars) = free_variables::compute(ddnnf);
        let mut n_models = vec![None; ddnnf.nodes().as_slice().len()];
        compute_models_from(
            ddnnf,
            &[],
            &|index| match &ddnnf.nodes()[usize::from(index)] {
                Node::Or(children) => std::iter::repeat(0).take(children.len()),
                _ => unreachable!(),
            },
            NodeIndex::from(0),
            &mut n_models,
        );
        Self {
            ddnnf,
            n_models,
            or_free_vars,
            root_free_vars,
        }
    }
}

/// A trait for objects that count elements on a Decision-DNNF.
pub trait Counter {
    /// Returns the number of counted elements of the whole formula.
    #[must_use]
    fn global_count(&self) -> &Integer;

    /// Returns the [`DecisionDNNF`] which elements are counted.
    #[must_use]
    fn ddnnf(&self) -> &DecisionDNNF;
}

pub(crate) trait CounterPrivate {
    fn count_from(&self, index: NodeIndex) -> &Integer;

    fn root_free_vars(&self) -> &[Literal];

    fn or_free_vars(&self) -> &[Vec<Vec<Literal>>];
}

macro_rules! counter_impl {
    ($type: ty) => {
        impl Counter for $type {
            fn global_count(&self) -> &Integer {
                self.n_models[0].as_ref().unwrap()
            }

            fn ddnnf(&self) -> &DecisionDNNF {
                self.ddnnf
            }
        }

        impl CounterPrivate for $type {
            fn count_from(&self, index: NodeIndex) -> &Integer {
                self.n_models[usize::from(index)].as_ref().unwrap()
            }

            fn root_free_vars(&self) -> &[Literal] {
                &self.root_free_vars
            }

            fn or_free_vars(&self) -> &[Vec<Vec<Literal>>] {
                &self.or_free_vars
            }
        }
    };
}

counter_impl!(ModelCounter<'_>);
counter_impl!(PathCounter<'_>);

fn compute_models_from<'a, F, G>(
    ddnnf: &'a DecisionDNNF,
    root_free_vars: &[Literal],
    or_children_free_vars_len: &F,
    index: NodeIndex,
    n_models: &mut [Option<Integer>],
) where
    F: Fn(NodeIndex) -> G,
    G: Iterator<Item = usize>,
{
    if n_models[usize::from(index)].is_some() {
        return;
    }
    let edge_indices = match &ddnnf.nodes()[index] {
        Node::And(edge_indices) | Node::Or(edge_indices) => edge_indices.clone(),
        Node::True | Node::False => vec![],
    };
    for e in edge_indices {
        let target = ddnnf.edges()[e].target();
        compute_models_from(
            ddnnf,
            root_free_vars,
            or_children_free_vars_len,
            target,
            n_models,
        );
    }
    let iter_for_children = |c: &'a Vec<EdgeIndex>| {
        c.iter().map(|e| {
            n_models[usize::from(ddnnf.edges()[*e].target())]
                .as_ref()
                .unwrap()
        })
    };
    let mut n = match &ddnnf.nodes()[index] {
        Node::And(edge_indices) => iter_for_children(edge_indices).product(),
        Node::Or(edge_indices) => iter_for_children(edge_indices)
            .zip(or_children_free_vars_len(index))
            .map(|(n, w)| n.clone() << w)
            .sum(),
        Node::True => Integer::ONE.clone(),
        Node::False => Integer::ZERO,
    };
    if index == NodeIndex::from(0) {
        n <<= root_free_vars.len();
    }
    n_models[usize::from(index)] = Some(n);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn assert_counts(
        instance: &str,
        n_vars: Option<usize>,
        expected_model_count: usize,
        expected_path_count: usize,
    ) {
        let mut ddnnf = D4Reader::read(instance.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let model_counter = ModelCounter::new(&ddnnf);
        assert_eq!(
            expected_model_count,
            model_counter.global_count().to_usize_wrapping()
        );
        let path_counter = PathCounter::new(&ddnnf);
        assert_eq!(
            expected_path_count,
            path_counter.global_count().to_usize_wrapping()
        );
    }

    #[test]
    fn test_ok() {
        assert_counts(
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n",
            None,
            4,
            4,
        );
    }

    #[test]
    fn test_true_no_vars() {
        assert_counts("t 1 0\n", None, 1, 1);
    }

    #[test]
    fn test_true_one_var() {
        assert_counts("t 1 0\n", Some(1), 2, 1);
    }

    #[test]
    fn test_true_two_vars() {
        assert_counts("t 1 0\n", Some(2), 4, 1);
    }

    #[test]
    fn test_false() {
        assert_counts("f 1 0\n", None, 0, 0);
    }

    #[test]
    fn test_clause() {
        assert_counts(
            r"
                o 1 0
                o 2 0
                t 3 0
                2 3 -1 -2 0
                2 3 1 0
                1 2 0",
            None,
            3,
            2,
        );
    }

    #[test]
    fn test_implied_lit() {
        assert_counts(
            r"
                o 1 0
                o 2 0
                t 3 0
                f 4 0
                2 3 -1 0
                2 4 1 0
                1 2 0",
            Some(2),
            2,
            1,
        );
    }
}
