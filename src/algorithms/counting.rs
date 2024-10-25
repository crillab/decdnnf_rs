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
/// use decdnnf_rs::{DecisionDNNF, ModelCounter};
///
/// fn count_models(ddnnf: &DecisionDNNF) {
///     let model_counter = ModelCounter::new(ddnnf, false);
///     println!("the formula has {} models", model_counter.global_count());
/// }
/// # count_models(&decdnnf_rs::D4Reader::default().read("t 1 0".as_bytes()).unwrap())
/// ```
pub struct ModelCounter<'a> {
    ddnnf: &'a DecisionDNNF,
    n_models: Vec<Option<Integer>>,
    partial_models: bool,
}

impl<'a> ModelCounter<'a> {
    /// Builds a new model counter given a formula.
    ///
    /// This function can both count the number of full or partial models.
    #[must_use]
    pub fn new(ddnnf: &'a DecisionDNNF, partial_models: bool) -> Self {
        let mut n_models = vec![None; ddnnf.nodes().as_slice().len()];
        let free_variables = ddnnf.free_vars();
        if partial_models {
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
        } else {
            compute_models_from(
                ddnnf,
                free_variables.root_free_vars(),
                &|index| {
                    free_variables
                        .or_free_vars()
                        .iter_child_free_vars_lengths(usize::from(index))
                },
                NodeIndex::from(0),
                &mut n_models,
            );
        }
        Self {
            ddnnf,
            n_models,
            partial_models,
        }
    }

    /// Returns the number of counted elements of the whole formula.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn global_count(&self) -> &Integer {
        self.n_models[0].as_ref().unwrap()
    }

    /// Returns the [`DecisionDNNF`] which elements are counted.
    #[must_use]
    pub fn ddnnf(&self) -> &DecisionDNNF {
        self.ddnnf
    }

    /// Returns the number of counted elements of the subfomula rooted at the node which index is given.
    ///
    /// # Panics
    ///
    /// This function panics if the provided node index is greater or equal to the number of nodes of the formula.
    #[must_use]
    pub fn count_from(&self, index: NodeIndex) -> &Integer {
        self.n_models[usize::from(index)].as_ref().unwrap()
    }

    /// Returns a Boolean value indicating if partial models are computed (`false` is returns in case full models are computed).
    #[must_use]
    pub fn partial_models(&self) -> bool {
        self.partial_models
    }
}

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
        let mut ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let model_counter = ModelCounter::new(&ddnnf, false);
        assert_eq!(
            expected_model_count,
            model_counter.global_count().to_usize_wrapping()
        );
        let path_counter = ModelCounter::new(&ddnnf, true);
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
