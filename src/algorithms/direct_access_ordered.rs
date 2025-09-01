use super::ModelCounter;
use crate::{DecisionDNNF, Literal};
use anyhow::{anyhow, Result};
use rug::Integer;

/// An object that, given an (internally computed) complete order on the models of a [`DecisionDNNF`], allows to return the k-th model.
///
/// This is the ordered counterpart of [`DirectAccessEngine`](crate::DirectAccessEngine).
/// The order of the models is given at the time the object is created as a list of literals.
/// The order of the models will be the same for two equivalent formulas, even if they have a different structure.
pub struct OrderedDirectAccessEngine<'a> {
    ddnnf: &'a DecisionDNNF,
    global_n_models: Integer,
    order: Vec<Literal>,
}

impl<'a> OrderedDirectAccessEngine<'a> {
    /// Builds a new [`DirectAccessOrderedEngine`] given a [`DecisionDNNF`] and an order.
    ///
    /// The order must contain exactly one literal of each variable in the problem, including those that are defined but not present in the formula.
    ///
    /// # Errors
    ///
    /// An error is returned if the order is incorrect.
    pub fn new(ddnnf: &'a DecisionDNNF, order: Vec<Literal>) -> Result<Self> {
        let mut sorted = order.clone();
        sorted.sort_unstable_by_key(Literal::var_index);
        if ddnnf.n_vars() != order.len()
            || (0..ddnnf.n_vars())
                .zip(sorted.iter())
                .any(|(i, l)| i != l.var_index())
        {
            return Err(anyhow!("order must involve all variables exactly once"));
        }
        let model_counter = ModelCounter::new(ddnnf, false);
        let global_n_models = Integer::from(model_counter.global_count());
        Ok(Self {
            ddnnf,
            global_n_models,
            order,
        })
    }

    /// Returns the model at the given index.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn model(&self, mut n: Integer) -> Option<Vec<Literal>> {
        if n > self.global_n_models {
            return None;
        }
        let n_vars = self.ddnnf.n_vars();
        let mut model = Vec::with_capacity(n_vars);
        let mut model_counter = ModelCounter::new(self.ddnnf, false);
        while model.len() != n_vars {
            model.push(self.order[model.len()]);
            model_counter.set_assumptions(&model);
            let current_n_models = model_counter.global_count();
            if &n > current_n_models {
                let popped = model.pop().unwrap();
                model.push(popped.flip());
                n -= current_n_models;
            }
        }
        Some(model)
    }

    /// Returns the underlying ddnnf.
    #[must_use]
    pub fn ddnnf(&self) -> &DecisionDNNF {
        self.ddnnf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn compute_models_ordered(ddnnf: &DecisionDNNF, order: &[isize]) -> Vec<Vec<isize>> {
        let lit_order = order.iter().map(|i| Literal::from(*i)).collect::<Vec<_>>();
        let engine = OrderedDirectAccessEngine::new(ddnnf, lit_order).unwrap();
        let model_counter = ModelCounter::new(engine.ddnnf(), false);
        let n_models = model_counter.global_count();
        let mut actual = Vec::with_capacity(n_models.to_usize_wrapping());
        for i in 0..n_models.to_usize_wrapping() {
            let m = engine.model(i.into()).unwrap();
            actual.push(m.iter().map(|l| isize::from(*l)).collect());
        }
        actual
    }

    #[test]
    fn test_same_formula_different_orders() {
        let str_ddnnf = "t 1 0";
        let mut ddnnf = D4Reader::default().read(str_ddnnf.as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let mut model_sets = Vec::new();
        let mut check_for = |order| {
            let new_model_set = compute_models_ordered(&ddnnf, order);
            assert!(!model_sets.contains(&new_model_set));
            model_sets.push(new_model_set);
        };
        check_for(&[-1, -2]);
        check_for(&[-1, 2]);
        check_for(&[1, -2]);
        check_for(&[1, 2]);
        check_for(&[-2, -1]);
        check_for(&[-2, 1]);
        check_for(&[2, -1]);
        check_for(&[2, 1]);
    }

    #[test]
    fn test_equivalent_formula_same_order() {
        let mut models = None;
        let models_ref = &mut models;
        let mut check_for = |str_ddnnf: &str| {
            let ddnnf = D4Reader::default().read(str_ddnnf.as_bytes()).unwrap();
            let new_model_set = compute_models_ordered(&ddnnf, &[-1, -2, -3]);
            if let Some(m) = models_ref {
                assert_eq!(m, &new_model_set);
            } else {
                *models_ref = Some(new_model_set);
            }
        };
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 -2 0\n3 4 1 3 0\n");
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 -2 0\n3 4 3 1 0\n");
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -2 -1 0\n3 4 1 3 0\n");
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -2 -1 0\n3 4 3 1 0\n");
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n3 4 -1 -2 0\n2 4 1 3 0\n");
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n3 4 -1 -2 0\n2 4 3 1 0\n");
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n3 4 -2 -1 0\n2 4 1 3 0\n");
        check_for("o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n3 4 -2 -1 0\n2 4 3 1 0\n");
    }

    #[test]
    fn test_wrong_order_missing_var() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        assert!(OrderedDirectAccessEngine::new(&ddnnf, vec![Literal::from(1)]).is_err());
    }

    #[test]
    fn test_wrong_order_unknown_var() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        assert!(
            OrderedDirectAccessEngine::new(&ddnnf, vec![Literal::from(1), Literal::from(3)])
                .is_err()
        );
    }

    #[test]
    fn test_wrong_order_multiple_instance_of_var() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        assert!(
            OrderedDirectAccessEngine::new(&ddnnf, vec![Literal::from(1), Literal::from(1)])
                .is_err()
        );
    }
}
