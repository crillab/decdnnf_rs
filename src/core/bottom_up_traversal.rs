use crate::{DecisionDNNF, Literal, Node};

/// A structure used to apply algorithms on a Decision-DNNF in a bottom-up fashion.
/// 
/// Algorithms that want to use this object must use a structure implementing the [`BottomUpVisitor`] trait.
pub struct BottomUpTraversal<T> {
    visitor: Box<dyn BottomUpVisitor<T>>,
}

pub trait BottomUpVisitor<T> {
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
    /// Builds a new traversal structure given an algorithms working in a bottom-up fashion.
    #[must_use] pub fn new(visitor: Box<dyn BottomUpVisitor<T>>) -> Self {
        Self { visitor }
    }

    /// Make the traversal, applying the algorithm given at this object creation time.
    #[must_use] pub fn traverse(&self, ddnnf: &DecisionDNNF) -> T {
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

pub struct BiBottomUpVisitor<T, U> {
    visitor_t: Box<dyn BottomUpVisitor<T>>,
    visitor_u: Box<dyn BottomUpVisitor<U>>,
}

impl<T, U> BottomUpVisitor<(T, U)> for BiBottomUpVisitor<T, U> {
    fn merge_for_and(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[usize],
        children: Vec<(&[Literal], (T, U))>,
    ) -> (T, U) {
        let (children_t, children_u) = children
            .into_iter()
            .map(|(propagated, (c_t, c_u))| ((propagated, c_t), (propagated, c_u)))
            .unzip();
        (
            self.visitor_t.merge_for_and(ddnnf, path, children_t),
            self.visitor_u.merge_for_and(ddnnf, path, children_u),
        )
    }

    fn merge_for_or(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[usize],
        children: Vec<(&[Literal], (T, U))>,
    ) -> (T, U) {
        let (children_t, children_u) = children
            .into_iter()
            .map(|(propagated, (c_t, c_u))| ((propagated, c_t), (propagated, c_u)))
            .unzip();
        (
            self.visitor_t.merge_for_or(ddnnf, path, children_t),
            self.visitor_u.merge_for_or(ddnnf, path, children_u),
        )
    }

    fn new_for_true(&self, ddnnf: &DecisionDNNF, path: &[usize]) -> (T, U) {
        (
            self.visitor_t.new_for_true(ddnnf, path),
            self.visitor_u.new_for_true(ddnnf, path),
        )
    }

    fn new_for_false(&self, ddnnf: &DecisionDNNF, path: &[usize]) -> (T, U) {
        (
            self.visitor_t.new_for_false(ddnnf, path),
            self.visitor_u.new_for_false(ddnnf, path),
        )
    }
}
