use crate::{
    core::{Node, NodeIndex},
    DecisionDNNF, Literal, ModelCounter,
};
use rug::Integer;

/// An object that, given an (internally computed) complete order on the models of a [`DecisionDNNF`], allows to return the k-th model.
pub struct DirectAccessEngine<'a> {
    model_counter: &'a ModelCounter<'a>,
}

impl<'a> DirectAccessEngine<'a> {
    /// Builds a new [`DirectAccessEngine`] given a [`ModelCounter`].
    ///
    /// The formula under consideration is the one of the model counter.
    #[must_use]
    pub fn new(model_counter: &'a ModelCounter<'a>) -> Self {
        Self { model_counter }
    }
}

impl<'a> DirectAccessEngine<'a> {
    /// Returns the number of models of the formula.
    #[must_use]
    pub fn n_models(&self) -> &Integer {
        self.model_counter.global_count()
    }

    /// Returns the model at the given index.
    #[must_use]
    pub fn model(&self, mut n: Integer) -> Option<Vec<Option<Literal>>> {
        if n >= *self.model_counter.global_count() {
            return None;
        }
        let mut model = vec![None; self.model_counter.ddnnf().n_vars()];
        update_model_with_free_vars(
            &mut model,
            &mut n,
            self.ddnnf().free_vars().root_free_vars(),
            self.model_counter.partial_models(),
        );
        self.build_model_from(&mut model, n, NodeIndex::from(0), &mut |_, _| {});
        Some(model)
    }

    /// Returns the model at the given index, along its model graph.
    ///
    /// If the index is higher than the number of models, [`None`] is returned.
    #[must_use]
    pub fn model_with_graph(&self, mut n: Integer) -> Option<(Vec<Option<Literal>>, Vec<usize>)> {
        if n >= *self.model_counter.global_count() {
            return None;
        }
        let mut model = vec![None; self.model_counter.ddnnf().n_vars()];
        let mut model_graph = vec![0; self.model_counter.ddnnf().nodes().as_slice().len()];
        update_model_with_free_vars(
            &mut model,
            &mut n,
            self.ddnnf().free_vars().root_free_vars(),
            self.model_counter.partial_models(),
        );
        self.build_model_from(
            &mut model,
            n,
            NodeIndex::from(0),
            &mut |node_index, child_index| {
                model_graph[usize::from(node_index)] = child_index;
            },
        );
        Some((model, model_graph))
    }

    fn build_model_from<F>(
        &self,
        model: &mut [Option<Literal>],
        mut n: Integer,
        index: NodeIndex,
        on_or_child_selection: &mut F,
    ) where
        F: FnMut(NodeIndex, usize),
    {
        match &self.model_counter.ddnnf().nodes()[index] {
            Node::And(edges) => {
                for edge in edges.iter().rev() {
                    let edge = &self.model_counter.ddnnf().edges()[*edge];
                    edge.propagated()
                        .iter()
                        .for_each(|p| model[p.var_index()] = Some(*p));
                    let target = edge.target();
                    let mut child_n_models = self.model_counter.count_from(target).to_owned();
                    n.div_rem_mut(&mut child_n_models);
                    self.build_model_from(model, child_n_models, target, on_or_child_selection);
                }
            }
            Node::Or(edges) => {
                for (i, edge) in edges.iter().enumerate() {
                    let edge = &self.model_counter.ddnnf().edges()[*edge];
                    let target = edge.target();
                    let child_n_models = self.model_counter.count_from(target);
                    let child_free_vars = self
                        .ddnnf()
                        .free_vars()
                        .or_free_vars()
                        .child_free_vars(usize::from(index), i);
                    let total_child_n_models = if self.model_counter.partial_models() {
                        Integer::from(child_n_models)
                    } else {
                        Integer::from(child_n_models << child_free_vars.len())
                    };
                    if n < total_child_n_models {
                        update_model_with_free_vars(
                            model,
                            &mut n,
                            child_free_vars,
                            self.model_counter.partial_models(),
                        );
                        on_or_child_selection(index, i);
                        edge.propagated()
                            .iter()
                            .for_each(|p| model[p.var_index()] = Some(*p));
                        self.build_model_from(model, n, target, on_or_child_selection);
                        return;
                    }
                    n -= total_child_n_models;
                }
            }
            Node::True => {}
            Node::False => unreachable!(),
        }
    }

    /// Returns the underlying ddnnf.
    #[must_use]
    pub fn ddnnf(&self) -> &DecisionDNNF {
        self.model_counter.ddnnf()
    }
}

fn update_model_with_free_vars(
    model: &mut [Option<Literal>],
    n: &mut Integer,
    free_vars: &[Literal],
    update_with_none: bool,
) {
    for (i, v) in free_vars.iter().rev().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        if update_with_none {
            model[v.var_index()] = None;
        } else if n.get_bit(i as u32) {
            model[v.var_index()] = Some(v.flip());
        } else {
            model[v.var_index()] = Some(*v);
        }
    }
    if !update_with_none {
        *n >>= free_vars.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn assert_models_eq(
        str_ddnnf: &str,
        expected_models: Vec<Vec<isize>>,
        expected_partial_models: Vec<Vec<isize>>,
        expected_graphs: Vec<Vec<usize>>,
        expected_partial_graphs: Vec<Vec<usize>>,
        n_vars: Option<usize>,
    ) {
        let mut ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let expected_models_with_graphs = expected_models
            .into_iter()
            .zip(expected_graphs)
            .collect::<Vec<_>>();
        let model_counter = ModelCounter::new(&ddnnf, false);
        let engine = DirectAccessEngine::new(&model_counter);
        let actual_models = compute_models(&engine);
        assert_eq!(expected_models_with_graphs, actual_models);
        let expected_partial_models_with_graphs = expected_partial_models
            .into_iter()
            .zip(expected_partial_graphs)
            .collect::<Vec<_>>();
        let path_counter = ModelCounter::new(&ddnnf, true);
        let engine = DirectAccessEngine::new(&path_counter);
        let actual_partial_models = compute_models(&engine);
        assert_eq!(expected_partial_models_with_graphs, actual_partial_models);
    }

    fn compute_models(engine: &DirectAccessEngine) -> Vec<(Vec<isize>, Vec<usize>)> {
        let n_models = engine.n_models();
        let mut actual = Vec::with_capacity(n_models.to_usize_wrapping());
        for i in 0..n_models.to_usize_wrapping() {
            let m0 = engine.model(i.into()).unwrap();
            let (m1, g) = engine.model_with_graph(i.into()).unwrap();
            assert_eq!(m0, m1);
            actual.push((
                m1.into_iter()
                    .filter_map(|opt_l| opt_l.map(isize::from))
                    .collect(),
                g,
            ));
        }
        actual
    }

    #[test]
    fn test_unsat() {
        assert_models_eq("f 1 0\n", vec![], vec![], vec![], vec![], None);
    }

    #[test]
    fn test_single_model() {
        assert_models_eq(
            "a 1 0\nt 2 0\n1 2 1 0\n",
            vec![vec![1]],
            vec![vec![1]],
            vec![vec![0, 0]],
            vec![vec![0, 0]],
            None,
        );
    }

    #[test]
    fn test_tautology() {
        assert_models_eq(
            "t 1 0\n",
            vec![vec![-1], vec![1]],
            vec![vec![]],
            vec![vec![0], vec![0]],
            vec![vec![0], vec![0]],
            Some(1),
        );
    }

    #[test]
    fn test_tautology_two_vars() {
        assert_models_eq(
            "t 1 0\n",
            vec![vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            vec![vec![]],
            vec![vec![0], vec![0], vec![0], vec![0]],
            vec![vec![0], vec![0], vec![0], vec![0]],
            Some(2),
        );
    }

    #[test]
    fn test_or() {
        assert_models_eq(
            "o 1 0\nt 2 0\n1 2 -1 0\n 1 2 1 0\n",
            vec![vec![-1], vec![1]],
            vec![vec![-1], vec![1]],
            vec![vec![0, 0], vec![1, 0]],
            vec![vec![0, 0], vec![1, 0]],
            None,
        );
    }

    #[test]
    fn test_or_with_free_var() {
        assert_models_eq(
            "o 1 0\nt 2 0\n1 2 -1 0\n 1 2 1 0\n",
            vec![vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            vec![vec![-1], vec![1]],
            vec![vec![0, 0], vec![0, 0], vec![1, 0], vec![1, 0]],
            vec![vec![0, 0], vec![1, 0]],
            Some(2),
        );
    }

    #[test]
    fn test_and() {
        assert_models_eq(
            "a 1 0\nt 2 0\n1 2 -1 0\n 1 2 -2 0\n",
            vec![vec![-1, -2]],
            vec![vec![-1, -2]],
            vec![vec![0, 0]],
            vec![vec![0, 0]],
            None,
        );
    }

    #[test]
    fn test_and_or() {
        assert_models_eq(
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n",
            vec![vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            vec![vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            vec![
                vec![0, 0, 0, 0],
                vec![0, 0, 1, 0],
                vec![0, 1, 0, 0],
                vec![0, 1, 1, 0],
            ],
            vec![
                vec![0, 0, 0, 0],
                vec![0, 0, 1, 0],
                vec![0, 1, 0, 0],
                vec![0, 1, 1, 0],
            ],
            None,
        );
    }

    #[test]
    fn test_or_and() {
        assert_models_eq(
            "o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 -2 0\n3 4 1 0\n3 4 2 0\n",
            vec![vec![-1, -2], vec![1, 2]],
            vec![vec![-1, -2], vec![1, 2]],
            vec![vec![0, 0, 0, 0], vec![1, 0, 0, 0]],
            vec![vec![0, 0, 0, 0], vec![1, 0, 0, 0]],
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
            vec![vec![-1, -2], vec![1]],
            vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 1, 0]],
            vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 1, 0]],
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
            vec![vec![-1]],
            vec![vec![0, 0, 0, 0], vec![0, 0, 0, 0]],
            vec![vec![0, 0, 0, 0], vec![0, 0, 0, 0]],
            Some(2),
        );
    }
}
