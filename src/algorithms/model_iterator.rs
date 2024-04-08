use crate::{core::InvolvedVars, DecisionDNNF, Literal, Node, NodeIndex};

/// A structure used to enumerate the models of a [`DecisionDNNF`].
#[derive(Debug)]
pub struct ModelIterator<'a> {
    ddnnf: &'a DecisionDNNF,
    or_edge_indices: Vec<usize>,
    or_free_vars: Vec<Vec<Vec<Literal>>>,
    root_free_vars: Vec<Literal>,
    next_model: Option<Vec<Literal>>,
}

impl<'a> ModelIterator<'a> {
    /// Builds a new model iterator for a [`DecisionDNNF`].
    #[must_use]
    pub fn new(ddnnf: &'a DecisionDNNF) -> Self {
        let n_nodes = ddnnf.nodes().as_slice().len();
        let mut result = Self {
            ddnnf,
            or_edge_indices: vec![0; n_nodes],
            or_free_vars: vec![vec![]; n_nodes],
            root_free_vars: vec![],
            next_model: None,
        };
        result.compute_free_vars();
        result.first_path_from(NodeIndex::from(0));
        result.next_model = if let Node::False = ddnnf.nodes()[NodeIndex::from(0)] {
            None
        } else {
            Some(result.current_model())
        };
        result
    }

    fn compute_free_vars(&mut self) {
        let mut involved_vars = vec![None; self.or_free_vars.len()];
        self.compute_free_vars_from(NodeIndex::from(0), &mut involved_vars);
        self.root_free_vars.append(
            &mut involved_vars[0]
                .as_ref()
                .unwrap()
                .iter_missing_literals()
                .collect(),
        );
    }

    fn compute_free_vars_from(
        &mut self,
        from: NodeIndex,
        involved_vars: &mut [Option<InvolvedVars>],
    ) {
        if involved_vars[usize::from(from)].is_some() {
            return;
        }
        involved_vars[usize::from(from)] = Some(self.compute_involved_vars(from, involved_vars));
        if let Node::Or(edges) = &self.ddnnf.nodes()[from] {
            for edge_index in edges {
                let edge = &self.ddnnf.edges()[*edge_index];
                let target = edge.target();
                let mut involved_in_child =
                    involved_vars[usize::from(target)].as_ref().unwrap().clone();
                for l in edge.propagated() {
                    involved_in_child.set_literal(*l);
                }
                involved_in_child.xor_assign(involved_vars[usize::from(from)].as_ref().unwrap());
                self.or_free_vars[usize::from(from)]
                    .push(involved_in_child.iter_pos_literals().collect());
            }
        }
    }

    fn compute_involved_vars(
        &mut self,
        node: NodeIndex,
        involved_vars: &mut [Option<InvolvedVars>],
    ) -> InvolvedVars {
        let mut union = InvolvedVars::new(self.ddnnf.n_vars());
        match &self.ddnnf.nodes()[node] {
            Node::And(edges) | Node::Or(edges) => {
                for edge_index in edges {
                    let edge = &self.ddnnf.edges()[*edge_index];
                    let target = edge.target();
                    self.compute_free_vars_from(target, involved_vars);
                    union.or_assign(involved_vars[usize::from(target)].as_ref().unwrap());
                    for l in edge.propagated() {
                        union.set_literal(*l);
                    }
                }
            }
            Node::True | Node::False => {}
        }
        union
    }

    fn next_path(&mut self) -> bool {
        if Self::next_free_vars_interpretation(&mut self.root_free_vars) {
            true
        } else {
            self.next_path_from(NodeIndex::from(0))
        }
    }

    fn next_path_from(&mut self, from: NodeIndex) -> bool {
        match &self.ddnnf.nodes()[from] {
            Node::And(edges) => {
                for edge in edges.iter().rev() {
                    let target = self.ddnnf.edges()[*edge].target();
                    if self.next_path_from(target) {
                        return true;
                    }
                    self.first_path_from(target);
                }
                false
            }
            Node::Or(edges) => {
                let mut child_index = self.or_edge_indices[usize::from(from)];
                if self.next_or_node_free_vars_interpretation(from, child_index) {
                    return true;
                }
                let mut target = self.ddnnf.edges()[edges[child_index]].target();
                if self.next_path_from(target) {
                    return true;
                }
                loop {
                    if child_index == edges.len() - 1 {
                        return false;
                    }
                    child_index += 1;
                    self.or_edge_indices[usize::from(from)] += child_index;
                    target = self.ddnnf.edges()[edges[child_index]].target();
                    if self.first_path_from(target) {
                        break;
                    }
                }
                true
            }
            Node::True | Node::False => false,
        }
    }

    fn next_or_node_free_vars_interpretation(
        &mut self,
        or_node: NodeIndex,
        child_index: usize,
    ) -> bool {
        Self::next_free_vars_interpretation(
            &mut self.or_free_vars[usize::from(or_node)][child_index],
        )
    }

    fn next_free_vars_interpretation(interpretation: &mut [Literal]) -> bool {
        if let Some(p) = interpretation.iter().rposition(Literal::polarity) {
            interpretation
                .iter_mut()
                .skip(p)
                .for_each(|l| *l = l.flip());
            true
        } else {
            interpretation.iter_mut().for_each(|l| *l = l.flip());
            false
        }
    }

    fn first_path_from(&mut self, from: NodeIndex) -> bool {
        self.or_edge_indices[usize::from(from)] = 0;
        match &self.ddnnf.nodes()[from] {
            Node::And(edges) => {
                for edge in edges {
                    let target = self.ddnnf.edges()[*edge].target();
                    if !self.first_path_from(target) {
                        return false;
                    }
                }
                true
            }
            Node::Or(edges) => {
                for edge in edges {
                    let target = self.ddnnf.edges()[*edge].target();
                    if self.first_path_from(target) {
                        return true;
                    }
                    self.or_edge_indices[usize::from(from)] += 1;
                }
                false
            }
            Node::True => true,
            Node::False => false,
        }
    }

    fn current_model(&self) -> Vec<Literal> {
        let mut model = Vec::with_capacity(self.ddnnf.n_vars());
        model.append(&mut self.root_free_vars.clone());
        self.current_model_from(NodeIndex::from(0), &mut model);
        model
    }

    fn current_model_from(&self, from: NodeIndex, model: &mut Vec<Literal>) {
        match &self.ddnnf.nodes()[from] {
            Node::And(edges) => {
                for edge_index in edges {
                    let edge = &self.ddnnf.edges()[*edge_index];
                    model.append(&mut edge.propagated().to_vec());
                    let target = edge.target();
                    self.current_model_from(target, model);
                }
            }
            Node::Or(children) => {
                let child_index = self.or_edge_indices[usize::from(from)];
                let edge_index = children[child_index];
                model.append(&mut self.or_free_vars[usize::from(from)][child_index].clone());
                let edge = &self.ddnnf.edges()[edge_index];
                model.append(&mut edge.propagated().to_vec());
                let target = edge.target();
                self.current_model_from(target, model);
            }
            Node::True | Node::False => {}
        }
    }
}

impl Iterator for ModelIterator<'_> {
    type Item = Vec<Literal>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.next_model.take();
        self.next_model = if self.next_path() {
            Some(self.current_model())
        } else {
            None
        };
        result
    }
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
        let model_it = ModelIterator::new(&ddnnf);
        let mut actual = model_it
            .inspect(|m| println!("got model {m:?}"))
            .map(|v| v.iter().map(|l| isize::from(*l)).collect::<Vec<_>>())
            .collect::<Vec<_>>();
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
