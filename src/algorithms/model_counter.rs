use crate::{
    core::{BottomUpVisitor, InvolvedVars},
    DecisionDNNF, Literal,
};
use rug::Integer;

/// A bottom-up visitor used for the model counting algorithm.
#[derive(Default)]
pub struct ModelCountingVisitor;

/// The data used by the [`ModelCountingVisitor`] structure.
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
        path: &[usize],
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
        path: &[usize],
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

    fn new_for_true(&self, ddnnf: &DecisionDNNF, path: &[usize]) -> ModelCountingVisitorData {
        adapt_for_root(
            ModelCountingVisitorData::new_for_leaf(ddnnf.n_vars(), 1),
            path,
        )
    }

    fn new_for_false(&self, ddnnf: &DecisionDNNF, path: &[usize]) -> ModelCountingVisitorData {
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

fn adapt_for_root(mut data: ModelCountingVisitorData, path: &[usize]) -> ModelCountingVisitorData {
    if path.len() == 1 {
        data.n_models *= 1 << data.involved_vars.count_zeros();
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::BottomUpTraversal, D4Reader};

    #[test]
    fn test_ok() {
        let str_ddnnf =
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n";
        let ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        let traversal = BottomUpTraversal::new(Box::<ModelCountingVisitor>::default());
        let result = traversal.traverse(&ddnnf);
        assert_eq!(4, result.n_models);
    }
}