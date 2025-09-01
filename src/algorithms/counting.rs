use crate::{
    core::{EdgeIndex, Node, NodeIndex},
    DecisionDNNF, Literal,
};
use rug::Integer;
use std::sync::OnceLock;

static INTEGER_ZERO: Integer = Integer::ZERO;

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
    assumptions: Option<Vec<Option<bool>>>,
    n_models: OnceLock<Vec<Option<Integer>>>,
    partial_models: bool,
}

impl<'a> ModelCounter<'a> {
    /// Builds a new model counter given a formula.
    ///
    /// This function can both count the number of full or partial models.
    #[must_use]
    pub fn new(ddnnf: &'a DecisionDNNF, partial_models: bool) -> Self {
        Self {
            ddnnf,
            assumptions: None,
            n_models: OnceLock::new(),
            partial_models,
        }
    }

    /// Set assumption literals, reducing the number of models.
    ///
    /// The only models to be considered are the ones that contain all the literals marked as assumptions.
    /// The set of assumptions must involved at most once each variable.
    ///
    /// # Panics
    ///
    /// This function panics if the set of assumptions involves the same variable multiple times.
    pub fn set_assumptions(&mut self, assumptions: &[Literal]) {
        let mut assumps = vec![None; self.ddnnf.n_vars()];
        for a in assumptions {
            assert!(a.var_index() < self.ddnnf.n_vars(), "undefined variable");
            assert!(
                assumps[a.var_index()].replace(a.polarity()).is_none(),
                "multiple definition of the same variable in assumptions"
            );
        }
        self.assumptions = Some(assumps);
        self.n_models.take();
    }

    fn get_or_compute_n_models(&self) -> &[Option<Integer>] {
        self.n_models.get_or_init(|| {
            let (n_root_free_vars, or_children_free_vars_len) = self.free_vars_params();
            let mut n_models = vec![None; self.ddnnf.nodes().as_slice().len()];
            compute_models_from(
                self.ddnnf,
                n_root_free_vars,
                &or_children_free_vars_len,
                NodeIndex::from(0),
                &mut n_models,
                &self.assumptions,
            );
            n_models
        })
    }

    fn free_vars_params(&self) -> (usize, Vec<Vec<usize>>) {
        if self.partial_models {
            (
                0,
                self.ddnnf
                    .nodes()
                    .as_slice()
                    .iter()
                    .map(|n| {
                        if let Node::Or(children) = n {
                            vec![0; children.len()]
                        } else {
                            vec![]
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            let free_variables = self.ddnnf.free_vars();
            if let Some(assumps) = &self.assumptions {
                let n_free_vars = |lits: &[Literal]| {
                    lits.iter()
                        .filter(|l| assumps[l.var_index()].is_none())
                        .count()
                };
                (
                    n_free_vars(free_variables.root_free_vars()),
                    (0..self.ddnnf.n_nodes())
                        .map(|i| {
                            free_variables
                                .or_free_vars()
                                .iter_child_free_vars(i)
                                .map(n_free_vars)
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>(),
                )
            } else {
                (
                    free_variables.root_free_vars().len(),
                    (0..self.ddnnf.n_nodes())
                        .map(|i| {
                            free_variables
                                .or_free_vars()
                                .iter_child_free_vars_lengths(i)
                                .collect::<Vec<_>>()
                        })
                        .collect::<Vec<_>>(),
                )
            }
        }
    }

    /// Returns the number of counted elements of the whole formula.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn global_count(&self) -> &Integer {
        self.get_or_compute_n_models()[0].as_ref().unwrap()
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
        self.get_or_compute_n_models()[usize::from(index)]
            .as_ref()
            .unwrap()
    }

    /// Returns a Boolean value indicating if partial models are computed (`false` is returns in case full models are computed).
    #[must_use]
    pub fn partial_models(&self) -> bool {
        self.partial_models
    }
}

fn compute_models_from<'a>(
    ddnnf: &'a DecisionDNNF,
    n_root_free_vars: usize,
    or_children_free_vars_len: &[Vec<usize>],
    index: NodeIndex,
    n_models: &mut [Option<Integer>],
    assumptions: &Option<Vec<Option<bool>>>,
) {
    if n_models[usize::from(index)].is_some() {
        return;
    }
    let edge_indices = match &ddnnf.nodes()[index] {
        Node::And(edge_indices) | Node::Or(edge_indices) => edge_indices.clone(),
        Node::True | Node::False => vec![],
    };
    let in_contradiction_with_assumptions = |e: EdgeIndex| {
        if let Some(assumps) = assumptions {
            ddnnf.edges()[e]
                .propagated()
                .iter()
                .any(|l| assumps[l.var_index()].is_some_and(|p| p != l.polarity()))
        } else {
            false
        }
    };
    for e in edge_indices {
        if in_contradiction_with_assumptions(e) {
            continue;
        }
        let target = ddnnf.edges()[e].target();
        compute_models_from(
            ddnnf,
            n_root_free_vars,
            or_children_free_vars_len,
            target,
            n_models,
            assumptions,
        );
    }
    let iter_for_children = |c: &'a Vec<EdgeIndex>| {
        c.iter().map(|e| {
            if in_contradiction_with_assumptions(*e) {
                return &INTEGER_ZERO;
            }
            n_models[usize::from(ddnnf.edges()[*e].target())]
                .as_ref()
                .unwrap()
        })
    };
    let mut n = match &ddnnf.nodes()[index] {
        Node::And(edge_indices) => iter_for_children(edge_indices).product(),
        Node::Or(edge_indices) => iter_for_children(edge_indices)
            .zip(or_children_free_vars_len[usize::from(index)].iter())
            .map(|(n, w)| n.clone() << w)
            .sum(),
        Node::True => Integer::ONE.clone(),
        Node::False => Integer::ZERO,
    };
    if index == NodeIndex::from(0) {
        n <<= n_root_free_vars;
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
        assert_counts_under_assumptions(
            instance,
            n_vars,
            expected_model_count,
            expected_path_count,
            None,
        );
    }

    fn assert_counts_under_assumptions(
        instance: &str,
        n_vars: Option<usize>,
        expected_model_count: usize,
        expected_path_count: usize,
        assumptions: Option<Vec<isize>>,
    ) {
        let mut ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let mut model_counter = ModelCounter::new(&ddnnf, false);
        if let Some(assumps) = assumptions.clone() {
            model_counter.set_assumptions(
                &assumps
                    .iter()
                    .map(|i| Literal::from(*i))
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(
            expected_model_count,
            model_counter.global_count().to_usize_wrapping()
        );
        let mut path_counter = ModelCounter::new(&ddnnf, true);
        if let Some(assumps) = assumptions {
            path_counter.set_assumptions(
                &assumps
                    .iter()
                    .map(|i| Literal::from(*i))
                    .collect::<Vec<_>>(),
            );
        }
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

    #[test]
    fn test_assumptions() {
        let instance = r"
        o 1 0
        a 2 0
        a 3 0
        t 4 0
        1 2 0
        1 3 0
        2 4 1 2 0
        3 4 -1 3 0
        ";
        let check = |assumptions, models, paths| {
            assert_counts_under_assumptions(instance, None, models, paths, assumptions);
        };
        check(None, 4, 2);
        check(Some(vec![-1]), 2, 1);
        check(Some(vec![1]), 2, 1);
        check(Some(vec![-2]), 1, 1);
        check(Some(vec![2]), 3, 2);
        check(Some(vec![-1, -2]), 1, 1);
        check(Some(vec![-1, 2]), 1, 1);
        check(Some(vec![1, -2]), 0, 0);
        check(Some(vec![1, 2]), 2, 1);
        check(Some(vec![-1, -2, -3]), 0, 0);
        check(Some(vec![-1, -2, 3]), 1, 1);
        check(Some(vec![-1, 2, -3]), 0, 0);
        check(Some(vec![-1, 2, 3]), 1, 1);
        check(Some(vec![1, -2, -3]), 0, 0);
        check(Some(vec![1, -2, 3]), 0, 0);
        check(Some(vec![1, 2, -3]), 1, 1);
        check(Some(vec![1, 2, 3]), 1, 1);
    }

    #[test]
    fn test_count_under_assumptions_top() {
        let instance = "t 1 0\n";
        let mut ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        ddnnf.update_n_vars(1);
        assert_counts_under_assumptions("t 1 0\n", Some(1), 2, 1, Some(vec![]));
        assert_counts_under_assumptions("t 1 0\n", Some(1), 1, 1, Some(vec![-1]));
        assert_counts_under_assumptions("t 1 0\n", Some(1), 1, 1, Some(vec![1]));
    }
}
