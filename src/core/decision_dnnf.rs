use crate::FreeVariables;
use anyhow::{anyhow, Result};
use std::{
    fmt::{Debug, Display},
    ops::Index,
    str::FromStr,
    sync::OnceLock,
};

/// A structure representing a literal.
///
/// Internally, a literal has a variable index and a polarity.
/// The variable indices begin at 0.
///
/// Such literals can be built from DIMACS representations using the [`From`] trait for isize.
/// When a literal is displayed, the DIMACS representation is used.
///
/// # Example
///
/// ```
/// use decdnnf_rs::Literal;
///
/// let l = Literal::from(1);
/// assert_eq!(0, l.var_index());
/// assert!(l.polarity());
/// assert_eq!(0, l.flip().var_index());
/// assert!(!l.flip().polarity());
/// assert_eq!("1", format!("{l}"));
/// ```
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Literal(usize);

impl Literal {
    /// Returns the variable index.
    /// Variable indices begin at 0.
    #[must_use]
    pub fn var_index(&self) -> usize {
        self.0 >> 1
    }

    /// Returns the polarity of the literal.
    ///
    /// This function returns `true` for a positive literal and `false` for a negative one.
    #[must_use]
    pub fn polarity(&self) -> bool {
        self.0 & 1 == 0
    }

    /// Returns the literal with the same variable index, but opposite polarity.
    #[must_use]
    pub fn flip(&self) -> Literal {
        Literal(self.0 ^ 1)
    }

    /// Sets the current literal polarity to false.
    ///
    /// This has no effect if the literal already has a false polarity.
    pub fn set_negative(&mut self) {
        self.0 |= 1;
    }

    pub(crate) fn into_usize(self) -> usize {
        self.0
    }

    pub(crate) fn from_usize(n: usize) -> Self {
        Self(n)
    }
}

impl From<isize> for Literal {
    fn from(value: isize) -> Self {
        let mut u = (value.unsigned_abs() - 1) << 1;
        if value < 0 {
            u |= 1;
        }
        Literal(u)
    }
}

impl From<Literal> for isize {
    fn from(l: Literal) -> Self {
        let abs = isize::try_from(l.var_index() + 1).unwrap();
        if l.polarity() {
            abs
        } else {
            -abs
        }
    }
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.polarity() {
            write!(f, "-")?;
        }
        write!(f, "{}", self.var_index() + 1)
    }
}

impl Debug for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

/// A Decision-DNNF node.
///
/// Note that there are no literal nodes; rather, they are encoded as arcs that target true nodes and propagate literals.
/// See [`DecisionDNNF`] for more information on the internal representation of such formulas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A conjunction node, associated with edges targeting its children.
    And(Vec<EdgeIndex>),
    /// A disjunction node, associated with edges targeting its children.
    Or(Vec<EdgeIndex>),
    /// A true node.
    True,
    /// A false node.
    False,
}

impl Node {
    pub(crate) fn add_edge(&mut self, index: EdgeIndex) -> Result<()> {
        match self {
            Node::And(v) | Node::Or(v) => v.push(index),
            Node::False | Node::True => return Err(anyhow!("cannot add an edge from a leaf node")),
        }
        Ok(())
    }
}

impl FromStr for Node {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "a" => Ok(Node::And(Vec::new())),
            "o" => Ok(Node::Or(Vec::new())),
            "t" => Ok(Node::True),
            "f" => Ok(Node::False),
            _ => Err(anyhow!("cannot build a DNNF node from {s}")),
        }
    }
}

/// An edge targets a node and propagates literals, in the spirit of recent [d4](https://github.com/crillab/d4) versions.
/// See [`DecisionDNNF`] for more information on the internal representation of such formulas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    target: NodeIndex,
    propagated: Vec<Literal>,
}

impl Edge {
    /// Returns the target of the edge.
    #[must_use]
    pub fn target(&self) -> NodeIndex {
        self.target
    }

    /// Returns the literals propagated by the edge.
    #[must_use]
    pub fn propagated(&self) -> &[Literal] {
        &self.propagated
    }

    pub(crate) fn from_raw_data(target: NodeIndex, propagated: Vec<Literal>) -> Self {
        Self { target, propagated }
    }

    /// Replace the target with the provided value.
    pub fn set_target(&mut self, target: NodeIndex) {
        self.target = target;
    }
}

/// A Decision-DNNF formula.
///
/// Decision-DNNF formulas (formerly defined as _Decision Graphs_) were defined here:
///
/// Hélène Fargier, Pierre Marquis:
/// [On the Use of Partially Ordered Decision Graphs in Knowledge Compilation and Quantified Boolean Formulae.](http://www.cril.univ-artois.fr/~marquis/fargier-marquis-aaai06.pdf) AAAI 2006: 42-47
///
/// Decision-DNNFs are built by readers; see e.g. [`D4Reader`](crate::D4Reader).
/// Internally, they are represented by a vector of [`Node`] and a vector of [`Edge`].
/// The first node of the vector is the root of the formula. The indices of the edges starting from conjunction and disjunction nodes are contained within those nodes.
/// Edges contain the index of the target node and the literals that are propagated by following them.
/// See the [d4](https://github.com/crillab/d4) repository for more information on the propagated literals.
#[derive(Debug)]
pub struct DecisionDNNF {
    n_vars: usize,
    nodes: NodeVec,
    edges: EdgeVec,
    free_vars: OnceLock<FreeVariables>,
}

impl DecisionDNNF {
    pub(crate) fn from_raw_data(n_vars: usize, nodes: Vec<Node>, edges: Vec<Edge>) -> Self {
        Self {
            n_vars,
            nodes: NodeVec(nodes),
            edges: EdgeVec(edges),
            free_vars: OnceLock::new(),
        }
    }

    /// Creates a new (sub)formula from an existing one.
    ///
    /// The new formula is rooted by the node of the initial formula, the index of which is given by the parameter `root`.
    /// The number of variables considered in the subformula is the same as in the initial formula.
    #[must_use]
    pub fn subformula(&self, root: NodeIndex) -> Self {
        SubformulaBuilder::build_from_new_root(self, root).into()
    }

    /// Updates the number of variables.
    ///
    /// The new number must be greater than the current number of variables.
    /// This function is useful when loading a Decision-DNNF in which the last variables are free. In this case, the formula itself is insufficient to deduce the actual number of variables.
    /// For example, the formula for the trivial, true Decision-DNNF reduces to the constant true, regardless of the number of variables.
    /// Calling this function indicates the real number of variables that the formula relies on.
    ///
    /// # Panics
    ///
    /// This function panics if the new number of variables is lower than the current number.
    pub fn update_n_vars(&mut self, n_vars: usize) {
        assert!(
            n_vars >= self.n_vars,
            "cannot reduce the number of variables"
        );
        self.n_vars = n_vars;
    }

    /// Returns the number of variables involved in the Decision-DNNF.
    ///
    /// If the number of variables is updated by calling [`update_n_vars`](Self::update_n_vars), then the updated value is returned.
    #[must_use]
    pub fn n_vars(&self) -> usize {
        self.n_vars
    }

    /// Returns the vector of nodes of the Decision-DNNF.
    pub fn nodes(&self) -> &NodeVec {
        &self.nodes
    }

    pub(crate) fn nodes_mut(&mut self) -> &mut NodeVec {
        &mut self.nodes
    }

    /// Returns the number of nodes in the formula.
    #[must_use]
    pub fn n_nodes(&self) -> usize {
        self.nodes.as_slice().len()
    }

    /// Returns the vector of edges of the Decision-DNNF.
    pub fn edges(&self) -> &EdgeVec {
        &self.edges
    }

    pub(crate) fn edges_mut(&mut self) -> &mut EdgeVec {
        &mut self.edges
    }

    /// Returns the number of edges in the formula.
    #[must_use]
    pub fn n_edges(&self) -> usize {
        self.edges.as_slice().len()
    }

    /// Returns the free variables.
    ///
    /// See [`FreeVariables`] for more information.
    pub fn free_vars(&self) -> &FreeVariables {
        self.free_vars.get_or_init(|| FreeVariables::compute(self))
    }
}

struct SubformulaBuilder<'a> {
    formula: &'a DecisionDNNF,
    old_to_new_node_index: Vec<Option<usize>>,
    new_nodes: Vec<Node>,
    new_edges: Vec<Edge>,
}

impl<'a> SubformulaBuilder<'a> {
    fn build_from_new_root(decision_dnnf: &'a DecisionDNNF, root: NodeIndex) -> Self {
        let mut builder = Self {
            formula: decision_dnnf,
            old_to_new_node_index: vec![None; decision_dnnf.n_nodes()],
            new_nodes: vec![],
            new_edges: vec![],
        };
        builder.copy_nodes_from(root);
        builder.copy_edges();
        builder
    }

    fn copy_nodes_from(&mut self, old_index: NodeIndex) {
        if self.old_to_new_node_index[usize::from(old_index)].is_some() {
            return;
        }
        self.old_to_new_node_index[usize::from(old_index)] = Some(self.new_nodes.len());
        self.new_nodes.push(self.formula.nodes()[old_index].clone());
        match &self.formula.nodes()[old_index] {
            Node::And(edge_indices) | Node::Or(edge_indices) => {
                for edge_index in edge_indices {
                    self.copy_nodes_from(self.formula.edges()[*edge_index].target());
                }
            }
            Node::True | Node::False => {}
        }
    }

    fn copy_edges(&mut self) {
        for node in &mut self.new_nodes {
            match node {
                Node::And(edge_indices) | Node::Or(edge_indices) => {
                    for edge_index in edge_indices {
                        let mut new_edge = self.formula.edges()[*edge_index].clone();
                        new_edge.target = self.old_to_new_node_index[usize::from(new_edge.target)]
                            .unwrap()
                            .into();
                        *edge_index = self.new_edges.len().into();
                        self.new_edges.push(new_edge);
                    }
                }
                Node::True | Node::False => {}
            }
        }
    }
}

impl From<SubformulaBuilder<'_>> for DecisionDNNF {
    fn from(builder: SubformulaBuilder) -> Self {
        DecisionDNNF {
            n_vars: builder.formula.n_vars,
            nodes: NodeVec(builder.new_nodes),
            edges: EdgeVec(builder.new_edges),
            free_vars: OnceLock::new(),
        }
    }
}

macro_rules! index_type {
    ($type_name:ident, $index_name:ident, $vec_index_name:ident) => {
        #[doc = concat!("An index type dedicated to [`", stringify!($type_name), "`] objects.")]
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $index_name(usize);

        impl From<usize> for $index_name {
            fn from(value: usize) -> Self {
                $index_name(value)
            }
        }

        impl From<$index_name> for usize {
            fn from(value: $index_name) -> Self {
                value.0
            }
        }

        #[doc = concat!("A vector of [`", stringify!($type_name), "`] objects.")]
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $vec_index_name(Vec<$type_name>);

        impl $vec_index_name {
            #[doc = concat!("Returns a ", stringify!($vec_index_name), " as a slice of [`", stringify!($type_name), "`].")]
            #[allow(dead_code)]
            #[must_use]
            pub fn as_slice(&self) -> &[$type_name] {
                &self.0
            }

            #[must_use]
            pub(crate) fn as_mut_slice(&mut self) -> &mut [$type_name] {
                &mut self.0
            }

            #[must_use]
            pub(crate) fn as_mut(&mut self) -> &mut Vec<$type_name> {
                &mut self.0
            }
        }

        impl Index<usize> for $vec_index_name {
            type Output = $type_name;

            fn index(&self, index: usize) -> &Self::Output {
                &self.0[index]
            }
        }

        impl Index<$index_name> for $vec_index_name {
            type Output = $type_name;

            fn index(&self, index: $index_name) -> &Self::Output {
                &self.0[usize::from(index)]
            }
        }
    };
}

index_type!(Edge, EdgeIndex, EdgeVec);
index_type!(Node, NodeIndex, NodeVec);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subformula_trivial() {
        let old = DecisionDNNF::from_raw_data(
            2,
            vec![Node::And(vec![0.into()]), Node::True],
            vec![Edge::from_raw_data(1.into(), vec![Literal::from(1)])],
        );
        let new = old.subformula(1.into());
        assert_eq!(new.n_vars(), old.n_vars());
        assert_eq!(new.nodes.as_slice(), &[Node::True]);
        assert_eq!(new.edges.as_slice(), &[]);
    }

    #[test]
    fn test_subformula_and() {
        let old = DecisionDNNF::from_raw_data(
            3,
            vec![
                Node::And(vec![0.into()]),
                Node::And(vec![1.into(), 2.into()]),
                Node::True,
            ],
            vec![
                Edge::from_raw_data(1.into(), vec![Literal::from(1)]),
                Edge::from_raw_data(2.into(), vec![Literal::from(2)]),
                Edge::from_raw_data(2.into(), vec![Literal::from(3)]),
            ],
        );
        let new = old.subformula(1.into());
        assert_eq!(old.n_vars(), new.n_vars());
        assert_eq!(
            &[Node::And(vec![0.into(), 1.into()]), Node::True],
            new.nodes.as_slice(),
        );
        assert_eq!(
            &[
                Edge::from_raw_data(1.into(), vec![Literal::from(2)]),
                Edge::from_raw_data(1.into(), vec![Literal::from(3)]),
            ],
            new.edges.as_slice(),
        );
    }

    #[test]
    fn test_subformula_and_chain() {
        let old = DecisionDNNF::from_raw_data(
            3,
            vec![
                Node::And(vec![0.into()]),
                Node::And(vec![1.into()]),
                Node::And(vec![2.into()]),
                Node::And(vec![3.into()]),
                Node::True,
            ],
            vec![
                Edge::from_raw_data(1.into(), vec![Literal::from(1)]),
                Edge::from_raw_data(2.into(), vec![Literal::from(2)]),
                Edge::from_raw_data(3.into(), vec![Literal::from(3)]),
                Edge::from_raw_data(4.into(), vec![Literal::from(4)]),
            ],
        );
        let new = old.subformula(1.into());
        assert_eq!(old.n_vars(), new.n_vars());
        assert_eq!(
            &[
                Node::And(vec![0.into()]),
                Node::And(vec![1.into()]),
                Node::And(vec![2.into()]),
                Node::True,
            ],
            new.nodes.as_slice(),
        );
        assert_eq!(
            &[
                Edge::from_raw_data(1.into(), vec![Literal::from(2)]),
                Edge::from_raw_data(2.into(), vec![Literal::from(3)]),
                Edge::from_raw_data(3.into(), vec![Literal::from(4)]),
            ],
            new.edges.as_slice(),
        );
    }
}
