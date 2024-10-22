use super::{Edge, EdgeIndex, Node, NodeIndex};
use crate::{DecisionDNNF, Literal};

/// A structure used to apply algorithms on a Decision-DNNF in a bottom-up fashion.
///
/// Algorithms that want to use this object must use a structure implementing the [`BottomUpVisitor`] trait.
///
/// The bottom-up search follows all the paths from the root to the leaves.
/// Since Decision-DNNFs are graphs, this means that if a node has multiple ancestors, then it will be reached multiple times.
/// This makes algorithms using the [`BottomUpVisitor`] take a higher computation time but a lower memory usage than algorithms that would take advantage of caching techniques.
///
/// # Example
///
/// ```
/// use decdnnf_rs::{BottomUpTraversal, ModelCountingVisitor, DecisionDNNF};
///
/// fn model_counting(ddnnf: &DecisionDNNF) {
///     let checker_visitor = Box::<ModelCountingVisitor>::default();
///     let traversal_engine = BottomUpTraversal::new(checker_visitor);
///     let mc_data = traversal_engine.traverse(&ddnnf);
///     println!("formula has {} models", mc_data.n_models());
/// }
/// # model_counting(&decdnnf_rs::D4Reader::read("t 1 0".as_bytes()).unwrap())
/// ```
pub struct BottomUpTraversal<T> {
    visitor: Box<dyn BottomUpVisitor<T>>,
}

/// A trait to be implemented by objects traversing Decision-DNNF formulas in a bottom-up fashion using a [`BottomUpTraversal`].
///
/// This trait contains functions that returns data when a formula node is traversed.
/// Each of these functions is dedicated to a kind of node.
/// Since the traversal is bottom-up, functions associated with internal nodes take as input children nodes that have already been computed by the object itself.
/// These functions returns a data type which is a parameter of the trait.
///
/// For an example of implementation, see e.g. the source code of [`ModelCountingVisitor`](crate::ModelCountingVisitor).
pub trait BottomUpVisitor<T> {
    /// Creates new data from an and node which children data are given.
    fn merge_for_and(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], T)>,
    ) -> T;

    /// Creates new data from an or node which children data are given.
    fn merge_for_or(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], T)>,
    ) -> T;

    /// Creates new data from a true node.
    fn new_for_true(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> T;

    /// Creates new data from a false node.
    fn new_for_false(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> T;
}

impl<T> BottomUpTraversal<T> {
    /// Builds a new traversal structure given an algorithms working in a bottom-up fashion.
    #[must_use]
    pub fn new(visitor: Box<dyn BottomUpVisitor<T>>) -> Self {
        Self { visitor }
    }

    /// Make the traversal, applying the algorithm given at this object creation time.
    ///
    /// The data resulting from the traversal of the root node is returned.
    #[must_use]
    pub fn traverse(&self, ddnnf: &DecisionDNNF) -> T {
        let mut path = Vec::with_capacity(ddnnf.n_vars());
        self.traverse_for(ddnnf, 0.into(), &mut path)
    }

    fn traverse_for(
        &self,
        ddnnf: &DecisionDNNF,
        node_index: NodeIndex,
        path: &mut Vec<NodeIndex>,
    ) -> T {
        path.push(node_index);
        let mut compute_new_children = |v: &[EdgeIndex]| {
            v.iter()
                .map(|e| {
                    let edge: &Edge = &ddnnf.edges()[*e];
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

/// A Bottom-up visitor made to decorate a pair of underlying visitors.
pub struct BiBottomUpVisitor<T, U> {
    visitor_t: Box<dyn BottomUpVisitor<T>>,
    visitor_u: Box<dyn BottomUpVisitor<U>>,
}

impl<T, U> BiBottomUpVisitor<T, U> {
    /// Builds a new visitor that decorates the given pair of visitors.
    #[must_use]
    pub fn new(
        visitor_t: Box<dyn BottomUpVisitor<T>>,
        visitor_u: Box<dyn BottomUpVisitor<U>>,
    ) -> Self {
        Self {
            visitor_t,
            visitor_u,
        }
    }
}

impl<T, U> BottomUpVisitor<(T, U)> for BiBottomUpVisitor<T, U> {
    fn merge_for_and(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
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
        path: &[NodeIndex],
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

    fn new_for_true(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> (T, U) {
        (
            self.visitor_t.new_for_true(ddnnf, path),
            self.visitor_u.new_for_true(ddnnf, path),
        )
    }

    fn new_for_false(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> (T, U) {
        (
            self.visitor_t.new_for_false(ddnnf, path),
            self.visitor_u.new_for_false(ddnnf, path),
        )
    }
}
