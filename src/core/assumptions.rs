use crate::Literal;
use std::ops::Index;

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
    /// It will also panic is more than one literal references the same variable.
    #[must_use]
    pub fn new(n_vars: usize, assumptions: Vec<Literal>) -> Self {
        let mut mapping = vec![None; n_vars];
        for a in &assumptions {
            assert!(
                mapping[a.var_index()].is_none(),
                "assumption set multiple times"
            );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok() {
        let assumption_lits = vec![Literal::from(2), Literal::from(-3)];
        let assumptions = Assumptions::new(4, assumption_lits.clone());
        assert_eq!(assumptions.as_slice(), assumption_lits.as_slice());
        assert_eq!(None, assumptions[0]);
        assert_eq!(Some(true), assumptions[1]);
        assert_eq!(Some(false), assumptions[2]);
        assert_eq!(None, assumptions[3]);
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 4 but the index is 4")]
    fn test_out_of_bounds() {
        let assumptions = Assumptions::new(4, vec![]);
        let _ = assumptions[4];
    }

    #[test]
    #[should_panic(expected = "assumption set multiple times")]
    fn test_redefinition() {
        let _ = Assumptions::new(4, vec![Literal::from(1), Literal::from(1)]);
    }
}
