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
/// Internal, a literal has a variable index and a polarity.
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

    /// Sets the current literal polarity to false.
    ///
    /// This has no effect if the literal has already a false polarity.
    pub fn set_negative(&mut self) {
        self.0 |= 1;
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
/// Note that there aren't literal nodes: they are encoded as arcs targeting true nodes and propagated literals.
#[derive(Debug)]
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
#[derive(Debug)]
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
}

/// A Decision-DNNF formula.
///
/// Decision-DNNF formulas (formerly defined as _Decision Graphs_) were defined here:
///
/// Hélène Fargier, Pierre Marquis:
/// [On the Use of Partially Ordered Decision Graphs in Knowledge Compilation and Quantified Boolean Formulae.](http://www.cril.univ-artois.fr/~marquis/fargier-marquis-aaai06.pdf) AAAI 2006: 42-47
///
/// Decision-DNNFs are built by readers; see e.g. [`D4Reader`](crate::D4Reader).
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

    /// Updates the number of variables.
    ///
    /// The new number must be higher than the current number of variables.
    /// This function is useful when you load a Decision DNNF in which the last variables are free; in this case, the formula itself is not sufficient to deduce the real number of variables.
    /// For example, when considering the trivial, true, Decision-DNNF, the formula resumes to the `true` constant  whatever the number of variables.
    /// Calling this function indicates real number of variables this formula relies on.
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
    ///
    /// In case the number of variables was updated by a call to [`update_n_vars`](Self::update_n_vars), then the updated value is returned.
    #[must_use]
    pub fn n_vars(&self) -> usize {
        self.n_vars
    }

    pub(crate) fn nodes(&self) -> &NodeVec {
        &self.nodes
    }

    pub(crate) fn edges(&self) -> &EdgeVec {
        &self.edges
    }

    /// Returns the free variables.
    ///
    /// See [`FreeVariables`] for more information.
    pub fn free_vars(&self) -> &FreeVariables {
        self.free_vars.get_or_init(|| FreeVariables::compute(self))
    }
}

macro_rules! index_type {
    ($type_name:ident, $index_name:ident, $vec_index_name:ident) => {
        #[doc = concat!("An index type dedicated to [`", stringify!($type_name), "`] objects.")]
        #[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
        #[derive(Debug)]
        pub struct $vec_index_name(Vec<$type_name>);

        impl $vec_index_name {
            #[doc = concat!("Returns a ", stringify!($vec_index_name), " as a slice of [`", stringify!($type_name), "`].")]
            #[allow(dead_code)]
            #[must_use]
            pub fn as_slice(&self) -> &[$type_name] {
                &self.0
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
