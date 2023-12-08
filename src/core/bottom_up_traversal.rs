use crate::{DecisionDNNF, Literal, Node};

pub(crate) struct BottomUpTraversal<T> {
    visitor: Box<dyn BottomUpVisitor<T>>,
}

pub(crate) trait BottomUpVisitor<T> {
    fn merge_for_and(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[usize],
        children: Vec<(&[Literal], T)>,
    ) -> T;

    fn merge_for_or(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[usize],
        children: Vec<(&[Literal], T)>,
    ) -> T;

    fn new_for_true(&self, ddnnf: &DecisionDNNF, path: &[usize]) -> T;

    fn new_for_false(&self, ddnnf: &DecisionDNNF, path: &[usize]) -> T;
}

impl<T> BottomUpTraversal<T> {
    pub(crate) fn new(visitor: Box<dyn BottomUpVisitor<T>>) -> Self {
        Self { visitor }
    }

    pub(crate) fn traverse(&self, ddnnf: &DecisionDNNF) -> T {
        let mut path = Vec::with_capacity(ddnnf.n_vars());
        self.traverse_for(ddnnf, 0, &mut path)
    }

    fn traverse_for(&self, ddnnf: &DecisionDNNF, node_index: usize, path: &mut Vec<usize>) -> T {
        path.push(node_index);
        let mut compute_new_children = |v: &[usize]| {
            v.iter()
                .map(|e| {
                    let edge = &ddnnf.edges()[*e];
                    let new_child = self.traverse_for(ddnnf, edge.target(), path);
                    (edge.propagated(), new_child)
                })
                .collect::<Vec<_>>()
        };
        let result = match &ddnnf.nodes()[node_index] {
            Node::And(v) => {
                let new_children = compute_new_children(v);
                self.visitor.merge_for_and(ddnnf, path, new_children)
            }
            Node::Or(v) => {
                let new_children = compute_new_children(v);
                self.visitor.merge_for_or(ddnnf, path, new_children)
            }
            Node::True => self.visitor.new_for_true(ddnnf, path),
            Node::False => self.visitor.new_for_false(ddnnf, path),
        };
        path.pop();
        result
    }
}
