use crate::{
    core::{Node, NodeIndex},
    Literal, ModelCounter,
};
use rug::Integer;

/// An object that, given an (internally computed) complete order on the models of a [`DecisionDNNF`], allows to return the k-th model.
pub struct DirectAccessEngine<'a> {
    model_counter: &'a ModelCounter<'a>,
}

impl<'a> DirectAccessEngine<'a> {
    /// Builds a new [`DirectAccessEngine`] given a [`ModelCounter`].
    /// The formula under consideration is the one of the model counter.
    #[must_use]
    pub fn new(model_counter: &'a ModelCounter<'a>) -> Self {
        Self { model_counter }
    }

    /// Returns the number of models of the formula.
    #[must_use]
    pub fn n_models(&self) -> &Integer {
        self.model_counter.n_models()
    }

    /// Returns the model at the given index.
    pub fn model(&mut self, mut n: Integer) -> Option<Vec<Literal>> {
        if n >= *self.model_counter.n_models() {
            return None;
        }
        let mut model = vec![Literal::from(1); self.model_counter.ddnnf().n_vars()];
        update_model_with_free_vars(&mut model, &mut n, self.model_counter.root_free_vars());
        self.build_model_from(&mut model, n, NodeIndex::from(0));
        Some(model)
    }

    fn build_model_from(&self, model: &mut [Literal], mut n: Integer, index: NodeIndex) {
        match &self.model_counter.ddnnf().nodes()[index] {
            Node::And(edges) => {
                for edge in edges {
                    let edge = &self.model_counter.ddnnf().edges()[*edge];
                    edge.propagated()
                        .iter()
                        .for_each(|p| model[p.var_index()] = *p);
                    let target = edge.target();
                    let mut child_n_models = self.model_counter.n_models_from(target).to_owned();
                    n.div_rem_mut(&mut child_n_models);
                    self.build_model_from(model, child_n_models, target);
                }
            }
            Node::Or(edges) => {
                let free_vars = &self.model_counter.or_free_vars()[usize::from(index)];
                for (i, edge) in edges.iter().enumerate() {
                    let edge = &self.model_counter.ddnnf().edges()[*edge];
                    let target = edge.target();
                    let child_n_models = self.model_counter.n_models_from(target);
                    let total_child_n_models = Integer::from(child_n_models << free_vars[i].len());
                    if n < total_child_n_models {
                        update_model_with_free_vars(model, &mut n, &free_vars[i]);
                        edge.propagated()
                            .iter()
                            .for_each(|p| model[p.var_index()] = *p);
                        self.build_model_from(model, n, target);
                        return;
                    }
                    n -= total_child_n_models;
                }
            }
            Node::True => {}
            Node::False => unreachable!(),
        }
    }
}

fn update_model_with_free_vars(model: &mut [Literal], n: &mut Integer, free_vars: &[Literal]) {
    for (i, v) in free_vars.iter().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        if n.get_bit(i as u32) {
            model[v.var_index()] = *v;
        } else {
            model[v.var_index()] = v.flip();
        }
    }
    *n >>= free_vars.len();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn assert_models_eq(str_ddnnf: &str, mut expected: Vec<Vec<isize>>, n_vars: Option<usize>) {
        let sort = |v: &mut Vec<Vec<isize>>| {
            v.iter_mut().for_each(|m| m.sort_unstable());
            v.sort_unstable();
        };
        sort(&mut expected);
        let mut ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let model_counter = ModelCounter::new(&ddnnf);
        let mut engine = DirectAccessEngine::new(&model_counter);
        let n_models = engine.n_models();
        let mut actual = Vec::with_capacity(n_models.to_usize_wrapping());
        for i in 0..n_models.to_usize_wrapping() {
            actual.push(
                engine
                    .model(i.into())
                    .unwrap()
                    .into_iter()
                    .map(isize::from)
                    .collect(),
            );
        }
        sort(&mut actual);
        assert_eq!(expected, actual,);
    }

    #[test]
    fn test_unsat() {
        assert_models_eq("f 1 0\n", vec![], None);
    }

    #[test]
    fn test_single_model() {
        assert_models_eq("a 1 0\nt 2 0\n1 2 1 0\n", vec![vec![1]], None);
    }

    #[test]
    fn test_tautology() {
        assert_models_eq("t 1 0\n", vec![vec![-1], vec![1]], Some(1));
    }

    #[test]
    fn test_or() {
        assert_models_eq(
            "o 1 0\nt 2 0\n1 2 -1 0\n 1 2 1 0\n",
            vec![vec![-1], vec![1]],
            None,
        );
    }

    #[test]
    fn test_and() {
        assert_models_eq(
            "a 1 0\nt 2 0\n1 2 -1 0\n 1 2 -2 0\n",
            vec![vec![-1, -2]],
            None,
        );
    }

    #[test]
    fn test_and_or() {
        assert_models_eq(
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n",
            vec![vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            None,
        );
    }

    #[test]
    fn test_or_and() {
        assert_models_eq(
            "o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 -2 0\n3 4 1 0\n3 4 2 0\n",
            vec![vec![-1, -2], vec![1, 2]],
            None,
        );
    }

    #[test]
    fn test_2_vars_3_models() {
        assert_models_eq(
            r"o 1 0
            o 2 0
            t 3 0
            2 3 -1 -2 0
            2 3 1 0
            1 2 0
            ",
            vec![vec![-1, -2], vec![1, -2], vec![1, 2]],
            None,
        );
    }

    #[test]
    fn test_implied_lit() {
        assert_models_eq(
            r"o 1 0
            o 2 0
            t 3 0
            f 4 0
            2 3 -1 0
            2 4 1 0
            1 2 0
            ",
            vec![vec![-1, -2], vec![-1, 2]],
            Some(2),
        );
    }
}
