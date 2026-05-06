use crate::{core::CNFFormula, Literal};
use anyhow::{anyhow, Context, Result};
use cnf_parser::Output;
use std::io::Read;

/// A reader for the DIMACS CNF format.
pub struct Reader;

impl Reader {
    /// Reads a DIMACS CNF instance and returns it.
    ///
    /// # Errors
    ///
    /// This function returns an error if the formula is incorrect or a reading issue appears.
    pub fn read<R>(reader: R) -> Result<CNFFormula>
    where
        R: Read,
    {
        let mut cnf_builder = CNFFormulaBuilder::default();
        let mut io_reader = cnf_parser::IoReader(reader);
        cnf_parser::parse_cnf(&mut io_reader, &mut cnf_builder)
            .map_err(|e| match e {
                cnf_parser::Error::Output(e) => anyhow!("{}", e.root_cause()),
                _ => anyhow!("parsing library error: {e:?}"),
            })
            .context("while parsing a CNF formula")?;
        Ok(cnf_builder.into())
    }
}

#[derive(Default)]
struct CNFFormulaBuilder {
    n_vars: usize,
    n_clauses: usize,
    data: Vec<Literal>,
    clause_indices: Vec<usize>,
    in_clause: bool,
}

impl Output for CNFFormulaBuilder {
    type Error = anyhow::Error;

    fn problem(&mut self, num_variables: u32, num_clauses: u32) -> Result<(), Self::Error> {
        self.n_vars = num_variables as usize;
        self.n_clauses = num_clauses as usize;
        self.data.reserve((num_clauses as usize) << 1);
        self.clause_indices.reserve_exact(num_clauses as usize);
        self.in_clause = false;
        Ok(())
    }

    fn literal(&mut self, literal: cnf_parser::Literal) -> Result<(), Self::Error> {
        if !self.in_clause {
            self.clause_indices.push(self.data.len());
            self.in_clause = true;
        }
        let lit = Literal::from(literal.into_value().get() as isize);
        if lit.var_index() >= self.n_vars {
            return Err(anyhow!("number of variables is different than declared"));
        }
        self.data.push(lit);
        Ok(())
    }

    fn finalize_clause(&mut self) -> Result<(), Self::Error> {
        self.in_clause = false;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), Self::Error> {
        if self.n_clauses == self.clause_indices.len() {
            Ok(())
        } else {
            Err(anyhow!("number of clauses is different than declared"))
        }
    }
}

impl From<CNFFormulaBuilder> for CNFFormula {
    fn from(value: CNFFormulaBuilder) -> Self {
        CNFFormula::from_data(value.n_vars, value.data, value.clause_indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Literal;

    #[test]
    fn test_ok() {
        let instance = "p cnf 2 2\n-1 -2 0\n1 2 0\n";
        let cnf = Reader::read(instance.as_bytes()).unwrap();
        assert_eq!(2, cnf.n_vars());
        assert_eq!(
            vec![
                &[Literal::from(-1), Literal::from(-2)],
                &[Literal::from(1), Literal::from(2)],
            ],
            cnf.iter_clauses().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_empty() {
        let instance = "p cnf 0 0\n";
        let cnf = Reader::read(instance.as_bytes()).unwrap();
        assert_eq!(0, cnf.n_vars());
        assert_eq!(
            vec![] as Vec<&[Literal]>,
            cnf.iter_clauses().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_var_out_of_bounds() {
        let instance = "p cnf 2 2\n-1 -2 0\n1 3 0\n";
        if let Err(e) = Reader::read(instance.as_bytes()) {
            assert_eq!(
                "number of variables is different than declared",
                format!("{}", e.root_cause())
            );
        } else {
            panic!();
        }
    }

    #[test]
    fn test_not_enough_clauses() {
        let instance = "p cnf 2 3\n-1 -2 0\n1 2 0\n";
        if let Err(e) = Reader::read(instance.as_bytes()) {
            assert_eq!(
                "number of clauses is different than declared",
                format!("{}", e.root_cause())
            );
        } else {
            panic!();
        }
    }

    #[test]
    fn test_too_much_clauses() {
        let instance = "p cnf 2 1\n-1 -2 0\n1 2 0\n";
        if let Err(e) = Reader::read(instance.as_bytes()) {
            assert_eq!(
                "number of clauses is different than declared",
                format!("{}", e.root_cause())
            );
        } else {
            panic!();
        }
    }

    #[test]
    #[should_panic(expected = "")]
    fn test_unexpected_character() {
        let instance = "p cnf 2 1\n-1 -a 0\n1 2 0\n";
        let _ = Reader::read(instance.as_bytes());
    }
}
