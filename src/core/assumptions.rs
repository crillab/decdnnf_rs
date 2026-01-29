use std::ops::Index;

use crate::Literal;

/// A structure that is used to handle literal assumptions.
#[derive(Debug, Clone)]
pub struct Assumptions {
    literals: Vec<Literal>,
    mapping: Vec<Option<bool>>,
}

impl Assumptions {
    /// Creates a new object to handle assumptions, provided the assumptions and the number of variables in the formula under consideration.
    ///
    /// Assumptions can then be iterated using the [`iter`](Self::iter) function or their definition can be queried using the [`Index`] trait.
    ///
    /// # Panics
    ///
    /// This method will panic if the index of a literal is equal to or greater than the number of variables.
    #[must_use]
    pub fn new(n_vars: usize, assumptions: Vec<Literal>) -> Self {
        let mut mapping = vec![None; n_vars];
        for a in &assumptions {
            mapping[a.var_index()] = Some(a.polarity());
        }
        Self {
            literals: assumptions,
            mapping,
        }
    }

    /// Returns a slice containing the literals.
    #[must_use]
    pub fn as_slice(&self) -> &[Literal] {
        &self.literals
    }
}

impl Index<usize> for Assumptions {
    type Output = Option<bool>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.mapping[index]
    }
}
