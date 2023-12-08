use anyhow::{anyhow, Result};
use std::{
    fmt::{Debug, Display},
    str::FromStr,
};

pub type EdgeIndex = usize;

/// A structure representing a literal.
///
/// Internal, a literal has a variable index and a polarity.
/// The variable indices begin at 0.
///
/// Such literals can be built from DIMACS representations usinf the [`From`] trait for isize.
/// When a literal is displayed, the DIMACS representation is used.
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
    /// This function returns `true` for the positive literal and `false` for the negative one.
    #[must_use]
    pub fn polarity(&self) -> bool {
        self.0 & 1 == 0
    }

    /// Returns the literal with the same variable index but the opposite polarity.
    #[must_use]
    pub fn flip(&self) -> Literal {
        Literal(self.0 ^ 1)
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
/// Note that there aren't literal nodes: they are encoded as arcs targeting true nodes and propagated literals.
pub enum Node {
    /// A conjunction node, associated with the edges to its children.
    And(Vec<EdgeIndex>),
    /// A disjunction node, associated with the edges to its children.
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
        };
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
pub struct Edge {
    target: EdgeIndex,
    propagated: Vec<Literal>,
}

impl Edge {
    /// Returns the target of the edge.
    #[must_use]
    pub fn target(&self) -> EdgeIndex {
        self.target
    }

    /// Returns the literals propagated by the edge.
    #[must_use]
    pub fn propagated(&self) -> &[Literal] {
        &self.propagated
    }

    pub(crate) fn from_raw_data(target: EdgeIndex, propagated: Vec<Literal>) -> Self {
        Self { target, propagated }
    }
}

/// A Decision-DNNF formula.
pub struct DecisionDNNF {
    n_vars: usize,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl DecisionDNNF {
    pub(crate) fn from_raw_data(n_vars: usize, nodes: Vec<Node>, edges: Vec<Edge>) -> Self {
        Self {
            n_vars,
            nodes,
            edges,
        }
    }

    /// Updates the number of variables.
    /// The new number must be higher than the current number of variables.
    ///
    /// # Panics
    ///
    /// This function panics if the new number of variables is lower than the current.
    pub fn update_n_vars(&mut self, n_vars: usize) {
        assert!(
            n_vars >= self.n_vars,
            "cannot reduce the number of variables"
        );
        self.n_vars = n_vars;
    }

    /// Returns the number of variables involved in this Decision-DNNF.
    #[must_use]
    pub fn n_vars(&self) -> usize {
        self.n_vars
    }

    pub(crate) fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub(crate) fn edges(&self) -> &[Edge] {
        &self.edges
    }
}
