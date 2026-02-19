use super::ModelCounter;
use crate::{Assumptions, DecisionDNNF, Literal};
use anyhow::{anyhow, Result};
use rug::Integer;
use std::{cell::RefCell, rc::Rc};

/// An object that, given a complete order on the models of a [`DecisionDNNF`] computed internally, allows the k-th model to be returned.
///
/// This is the ordered counterpart of [`DirectAccessEngine`](crate::DirectAccessEngine).
/// The order of the models is determined when the object is created and is represented as a list of literals.
/// The order of the models will be the same for two equivalent formulas, even if their structures differ.
pub struct OrderedDirectAccessEngine<'a> {
    ddnnf: &'a DecisionDNNF,
    global_n_models: RefCell<Option<Integer>>,
    order: Vec<Literal>,
    assumptions: Option<Rc<Assumptions>>,
}

impl<'a> OrderedDirectAccessEngine<'a> {
    /// Builds a new [`OrderedDirectAccessEngine`] given a [`DecisionDNNF`] and an order.
    ///
    /// The order must include exactly one instance of each variable from the problem, even those defined but not present in the formula.
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
        Ok(Self {
            ddnnf,
            global_n_models: RefCell::new(None),
            order,
            assumptions: None,
        })
    }

    /// Set assumption literals, reducing the number of models.
    ///
    /// The only models to be considered are those that contain all the literals marked as assumptions.
    pub fn set_assumptions(&mut self, assumptions: Rc<Assumptions>) {
        self.assumptions = Some(assumptions);
        *self.global_n_models.borrow_mut() = None;
    }

    /// Returns the model at the given index.
    ///
    /// In case there is less models than the index, [`None`] is returned.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn model(&self, mut n: Integer) -> Option<Vec<Literal>> {
        if self.global_n_models.borrow().is_none() {
            let mut model_counter = ModelCounter::new(self.ddnnf, false);
            if let Some(a) = &self.assumptions {
                model_counter.set_assumptions(Rc::clone(a));
            }
            *self.global_n_models.borrow_mut() = Some(Integer::from(model_counter.global_count()));
        }
        if &n >= self.global_n_models.borrow().as_ref().unwrap() {
            return None;
        }
        let n_vars = self.ddnnf.n_vars();
        let mut model = Vec::with_capacity(n_vars);
        let mut model_counter = ModelCounter::new(self.ddnnf, false);
        while model.len() != n_vars {
            let mut new_lit = self.order[model.len()];
            if let Some(a) = &self.assumptions {
                if let Some(p) = a[new_lit.var_index()] {
                    if new_lit.polarity() != p {
                        new_lit = new_lit.flip();
                    }
                    model.push(new_lit);
                    continue;
                }
            }
            model.push(new_lit);
            let mut mc_assumptions = Assumptions::new(n_vars, model.clone());
            if let Some(a) = &self.assumptions {
                mc_assumptions.union(a);
            }
            model_counter.set_assumptions(Rc::new(mc_assumptions));
            let current_n_models = model_counter.global_count();
            if &n >= current_n_models {
                let popped = model.pop().unwrap();
                model.push(popped.flip());
                n -= current_n_models;
            }
        }
        Some(model)
    }

    /// Returns the underlying formula.
    #[must_use]
    pub fn ddnnf(&self) -> &DecisionDNNF {
        self.ddnnf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{D4Reader, DecisionDNNFReader};

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

    #[test]
    fn test_no_such_model_index() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(1);
        let engine = OrderedDirectAccessEngine::new(&ddnnf, vec![Literal::from(1)]).unwrap();
        assert!(engine.model(Integer::from(2)).is_none());
    }

    #[test]
    fn test_lexico() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(1);
        let engine = OrderedDirectAccessEngine::new(&ddnnf, vec![Literal::from(-1)]).unwrap();
        assert_eq!(
            Some(vec![Literal::from(-1)]),
            engine.model(Integer::from(0))
        );
        assert_eq!(Some(vec![Literal::from(1)]), engine.model(Integer::from(1)));
        assert_eq!(None, engine.model(Integer::from(2)));
    }

    #[test]
    fn test_assumptions_first() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let mut engine =
            OrderedDirectAccessEngine::new(&ddnnf, vec![Literal::from(-1), Literal::from(-2)])
                .unwrap();
        engine.set_assumptions(Rc::new(Assumptions::new(2, vec![Literal::from(1)])));
        assert_eq!(
            Some(vec![Literal::from(1), Literal::from(-2)]),
            engine.model(Integer::from(0))
        );
        assert_eq!(
            Some(vec![Literal::from(1), Literal::from(2)]),
            engine.model(Integer::from(1))
        );
        assert_eq!(None, engine.model(Integer::from(2)));
    }

    #[test]
    fn test_assumptions_last() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let mut engine =
            OrderedDirectAccessEngine::new(&ddnnf, vec![Literal::from(-1), Literal::from(-2)])
                .unwrap();
        engine.set_assumptions(Rc::new(Assumptions::new(2, vec![Literal::from(2)])));
        assert_eq!(
            Some(vec![Literal::from(-1), Literal::from(2)]),
            engine.model(Integer::from(0))
        );
        assert_eq!(
            Some(vec![Literal::from(1), Literal::from(2)]),
            engine.model(Integer::from(1))
        );
        assert_eq!(None, engine.model(Integer::from(2)));
    }
}
