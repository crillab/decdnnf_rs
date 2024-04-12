use crate::{
    core::{BottomUpVisitor, InvolvedVars, NodeIndex},
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
/// use decdnnf_rs::{BottomUpTraversal, DecisionDNNF, ModelCountingVisitor};
///
/// fn check_decision_dnnf(ddnnf: &DecisionDNNF) {
///     let traversal = BottomUpTraversal::new(Box::<ModelCountingVisitor>::default());
///     let result = traversal.traverse(&ddnnf);
///     println!("the formula has {} models", result.n_models());
/// }
/// # check_decision_dnnf(&decdnnf_rs::D4Reader::read("t 1 0".as_bytes()).unwrap())
/// ```
#[derive(Default)]
pub struct ModelCountingVisitor;

/// The data returned by the [`ModelCountingVisitor`] algorithm.
///
/// See its documentation for more information.
pub struct ModelCountingVisitorData {
    n_models: Integer,
    involved_vars: InvolvedVars,
}

impl ModelCountingVisitorData {
    fn new_for_leaf(n_vars: usize, n_models: usize) -> Self {
        Self {
            n_models: Integer::from(n_models),
            involved_vars: InvolvedVars::new(n_vars),
        }
    }

    /// Returns the number of models.
    #[must_use]
    pub fn n_models(&self) -> &Integer {
        &self.n_models
    }
}

impl BottomUpVisitor<ModelCountingVisitorData> for ModelCountingVisitor {
    fn merge_for_and(
        &self,
        _ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], ModelCountingVisitorData)>,
    ) -> ModelCountingVisitorData {
        adapt_for_root(
            merge_children(children, &|v0, v1| {
                v0.n_models.clone() * v1.n_models.clone()
            }),
            path,
        )
    }

    fn merge_for_or(
        &self,
        _ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], ModelCountingVisitorData)>,
    ) -> ModelCountingVisitorData {
        adapt_for_root(
            merge_children(children, &|v0, v1| {
                let mut intersection = v0.involved_vars.clone();
                intersection.and_assign(&v1.involved_vars);
                let intersection_ones = intersection.count_ones();
                v0.n_models.clone() * (1 << (v1.involved_vars.count_ones() - intersection_ones))
                    + v1.n_models.clone()
                        * (1 << (v0.involved_vars.count_ones() - intersection_ones))
            }),
            path,
        )
    }

    fn new_for_true(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> ModelCountingVisitorData {
        adapt_for_root(
            ModelCountingVisitorData::new_for_leaf(ddnnf.n_vars(), 1),
            path,
        )
    }

    fn new_for_false(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> ModelCountingVisitorData {
        adapt_for_root(
            ModelCountingVisitorData::new_for_leaf(ddnnf.n_vars(), 0),
            path,
        )
    }
}

fn merge_children(
    children: Vec<(&[Literal], ModelCountingVisitorData)>,
    n_models_fn: &dyn Fn(&ModelCountingVisitorData, &ModelCountingVisitorData) -> Integer,
) -> ModelCountingVisitorData {
    let new_children = children
        .into_iter()
        .map(|(propagated, mut child)| {
            child.involved_vars.set_literals(propagated);
            child
        })
        .collect::<Vec<_>>();
    new_children
        .into_iter()
        .reduce(|mut acc, to_merge| {
            acc.n_models = n_models_fn(&acc, &to_merge);
            acc.involved_vars.or_assign(&to_merge.involved_vars);
            acc
        })
        .expect("cannot merge an empty set of children")
}

fn adapt_for_root(
    mut data: ModelCountingVisitorData,
    path: &[NodeIndex],
) -> ModelCountingVisitorData {
    if path.len() == 1 {
        data.n_models *= 1 << data.involved_vars.count_zeros();
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::BottomUpTraversal, D4Reader};

    fn model_count(instance: &str, n_vars: Option<usize>) -> usize {
        let mut ddnnf = D4Reader::read(instance.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let traversal = BottomUpTraversal::new(Box::<ModelCountingVisitor>::default());
        let result = traversal.traverse(&ddnnf);
        result.n_models.to_usize_wrapping()
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
