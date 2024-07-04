use super::free_variables;
use crate::{
    core::{EdgeIndex, Node, NodeIndex},
    DecisionDNNF, Literal,
};
use rug::Integer;

/// A structure used to count the models of a [`DecisionDNNF`].
///
/// The algorithm takes a time polynomial in the size of the Decision-DNNF.
/// For each node in the formula, the number of models for the subformula rooted at this node is kept and can be queried.
///
/// # Example
///
/// ```
/// use decdnnf_rs::{DecisionDNNF, ModelCounter};
///
/// fn count_models(ddnnf: &DecisionDNNF) {
///     let model_counter = ModelCounter::new(ddnnf);
///     println!("the formula has {} models", model_counter.n_models());
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
    ///
    /// This function computes the number of models for each subformula rooted at a node of the Decision-DNNF.
    #[must_use]
    pub fn new(ddnnf: &'a DecisionDNNF) -> Self {
        let (or_free_vars, root_free_vars) = free_variables::compute(ddnnf);
        let mut result = Self {
            ddnnf,
            n_models: vec![None; ddnnf.nodes().as_slice().len()],
            or_free_vars,
            root_free_vars,
        };
        result.compute_models_from(NodeIndex::from(0));
        result
    }

    fn compute_models_from(&mut self, index: NodeIndex) {
        if self.n_models[usize::from(index)].is_some() {
            return;
        }
        let edge_indices = match &self.ddnnf.nodes()[index] {
            Node::And(edge_indices) | Node::Or(edge_indices) => edge_indices.clone(),
            Node::True | Node::False => vec![],
        };
        for e in edge_indices {
            let target = self.ddnnf.edges()[e].target();
            self.compute_models_from(target);
        }
        let iter_for_children = |c: &'a Vec<EdgeIndex>| {
            c.iter().map(|e| {
                self.n_models[usize::from(self.ddnnf.edges()[*e].target())]
                    .as_ref()
                    .unwrap()
            })
        };
        let mut n = match &self.ddnnf.nodes()[index] {
            Node::And(edge_indices) => iter_for_children(edge_indices).product(),
            Node::Or(edge_indices) => iter_for_children(edge_indices)
                .zip(self.or_free_vars[usize::from(index)].iter().map(Vec::len))
                .map(|(n, w)| n.clone() << w)
                .sum(),
            Node::True => Integer::ONE.clone(),
            Node::False => Integer::ZERO,
        };
        if index == NodeIndex::from(0) {
            n <<= self.root_free_vars.len();
        }
        self.n_models[usize::from(index)] = Some(n);
    }

    /// Returns the number of models of the whole formula, including the free variables.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn n_models(&self) -> &Integer {
        self.n_models[0].as_ref().unwrap()
    }

    pub(crate) fn n_models_from(&self, index: NodeIndex) -> &Integer {
        self.n_models[usize::from(index)].as_ref().unwrap()
    }

    /// Returns the [`DecisionDNNF`] which models are counted.
    #[must_use]
    pub fn ddnnf(&self) -> &DecisionDNNF {
        self.ddnnf
    }

    pub(crate) fn root_free_vars(&self) -> &[Literal] {
        &self.root_free_vars
    }

    pub(crate) fn or_free_vars(&self) -> &[Vec<Vec<Literal>>] {
        &self.or_free_vars
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn model_count(instance: &str, n_vars: Option<usize>) -> usize {
        let mut ddnnf = D4Reader::read(instance.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let model_counter = ModelCounter::new(&ddnnf);
        model_counter.n_models().to_usize_wrapping()
    }

    #[test]
    fn test_ok() {
        assert_eq!(
            4,
            model_count(
                "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n",
                None
            )
        );
    }

    #[test]
    fn test_true_no_vars() {
        assert_eq!(1, model_count("t 1 0\n", None));
    }

    #[test]
    fn test_true_one_var() {
        assert_eq!(2, model_count("t 1 0\n", Some(1)));
    }

    #[test]
    fn test_true_two_vars() {
        assert_eq!(4, model_count("t 1 0\n", Some(2)));
    }

    #[test]
    fn test_false() {
        assert_eq!(0, model_count("f 1 0\n", None));
    }

    #[test]
    fn test_clause() {
        assert_eq!(
            3,
            model_count(
                r"
                o 1 0
                o 2 0
                t 3 0
                2 3 -1 -2 0
                2 3 1 0
                1 2 0",
                None
            )
        );
    }

    #[test]
    fn test_implied_lit() {
        assert_eq!(
            2,
            model_count(
                r"
                o 1 0
                o 2 0
                t 3 0
                f 4 0
                2 3 -1 0
                2 4 1 0
                1 2 0",
                Some(2)
            )
        );
    }
}
