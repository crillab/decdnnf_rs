use crate::Literal;

/// A structure that handles a CNF formula.
#[derive(Default)]
pub struct CNFFormula {
    n_vars: usize,
    data: Vec<Literal>,
    clause_indices: Vec<usize>,
}

impl CNFFormula {
    pub(crate) fn from_data(n_vars: usize, data: Vec<Literal>, clause_indices: Vec<usize>) -> Self {
        Self {
            n_vars,
            data,
            clause_indices,
        }
    }

    /// Returns the number of variables involved in the CNF formula.
    #[must_use]
    pub fn n_vars(&self) -> usize {
        self.n_vars
    }

    /// Returns the number of clauses involved in the CNF formula.
    #[must_use]
    pub fn n_clauses(&self) -> usize {
        self.clause_indices.len()
    }

    /// Updates the number of variables.
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

    /// Returns an iterator that yields the clauses of this CNF formula.
    #[must_use]
    pub fn iter_clauses(&self) -> ClauseIterator<'_> {
        ClauseIterator::new(self)
    }
}

/// An iterator that yields the clauses of a CNF formula.
pub struct ClauseIterator<'a> {
    cnf: &'a CNFFormula,
    next_clause_index: usize,
}

impl<'a> ClauseIterator<'a> {
    fn new(cnf: &'a CNFFormula) -> Self {
        Self {
            cnf,
            next_clause_index: 0,
        }
    }
}

impl<'a> Iterator for ClauseIterator<'a> {
    type Item = &'a [Literal];

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_clause_index == self.cnf.clause_indices.len() {
            None
        } else {
            let min_bound = self.cnf.clause_indices[self.next_clause_index];
            self.next_clause_index += 1;
            let max_bound = if self.next_clause_index == self.cnf.clause_indices.len() {
                self.cnf.data.len()
            } else {
                self.cnf.clause_indices[self.next_clause_index]
            };
            Some(&self.cnf.data[min_bound..max_bound])
        }
    }
}
