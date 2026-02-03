use crate::{
    core::{EdgeIndex, InvolvedVars, Node, NodeIndex},
    DecisionDNNF, Literal,
};

/// A structure used to find models in a [`DecisionDNNF`].
///
/// Queries can involve assumption literals. In this case, the only models under consideration are those that include such literals.
/// If no models include those assumptions, the query will return that no models exist.
///
/// # Example
///
/// ```
/// use decdnnf_rs::{Literal, ModelFinder};
///
/// # use decdnnf_rs::DecisionDNNFReader;
/// # fn gimme_ddnnf() -> decdnnf_rs::DecisionDNNF {let mut r = decdnnf_rs::D4Reader::default().read("t 1 0".as_bytes()).unwrap(); r.update_n_vars(1); r}
/// let ddnnf = gimme_ddnnf();
/// let model_finder = ModelFinder::new(&ddnnf);
/// if let Some(model) = model_finder.find_model() {
///     println!("the formula has models; here is one:");
///     for l in model {
///         print!("{l} ");
///     }
///     println!();
///     if model_finder.find_model_under_assumptions(&[Literal::from(-1)]).is_some() {
///         println!("some of them involve the literal -1");
///     }
///     if model_finder.find_model_under_assumptions(&[Literal::from(1)]).is_some() {
///         println!("some of them involve the literal 1");
///     }
/// } else {
///     println!("the formula has no model");
/// }
/// ```
pub struct ModelFinder<'a> {
    ddnnf: &'a DecisionDNNF,
}

impl<'a> ModelFinder<'a> {
    /// Builds a new model finder given a [`DecisionDNNF`].
    #[must_use]
    pub fn new(ddnnf: &'a DecisionDNNF) -> Self {
        Self { ddnnf }
    }

    /// Search for a model.
    #[must_use]
    pub fn find_model(&self) -> Option<Vec<Literal>> {
        self.find_model_under_assumptions(&[])
    }

    /// Search for a model that is compatible with the provided assumptions.
    ///
    /// # Panics
    ///
    /// Literals must refer to existing variables.
    /// In case the variable index of a literal is higher than the highest variable index in the formula, this function panics.
    #[must_use]
    pub fn find_model_under_assumptions(&self, assumptions: &[Literal]) -> Option<Vec<Literal>> {
        if let Some(l) = assumptions
            .iter()
            .find(|l| l.var_index() >= self.ddnnf.n_vars())
        {
            panic!(
                "no such literal: {l} (the formula has {} variables)",
                self.ddnnf.n_vars()
            );
        }
        let mut pos_assumptions = InvolvedVars::new(self.ddnnf.n_vars());
        let mut neg_assumptions = InvolvedVars::new(self.ddnnf.n_vars());
        for assumption in assumptions {
            if is_compatible_with_assumptions(*assumption, &pos_assumptions, &neg_assumptions) {
                if assumption.polarity() {
                    pos_assumptions.set_literal(*assumption);
                } else {
                    neg_assumptions.set_literal(*assumption);
                }
            }
        }
        let mut model = Vec::with_capacity(self.ddnnf.n_vars());
        if self.find_model_under_assumptions_from_node(
            NodeIndex::from(0),
            &mut model,
            &pos_assumptions,
            &neg_assumptions,
        ) {
            if model.len() < self.ddnnf.n_vars() {
                let mut involved = InvolvedVars::new(self.ddnnf.n_vars());
                involved.set_literals(&model);
                for missing in involved.iter_missing_literals() {
                    if is_compatible_with_assumptions(missing, &pos_assumptions, &neg_assumptions) {
                        model.push(missing);
                    } else {
                        model.push(missing.flip());
                    }
                }
            }
            Some(model)
        } else {
            None
        }
    }

    fn find_model_under_assumptions_from_node(
        &self,
        from: NodeIndex,
        model: &mut Vec<Literal>,
        pos_assumptions: &InvolvedVars,
        neg_assumptions: &InvolvedVars,
    ) -> bool {
        match &self.ddnnf.nodes()[from] {
            Node::And(edge_indices) => {
                for edge_index in edge_indices {
                    if !self.find_model_under_assumptions_from_edge(
                        *edge_index,
                        model,
                        pos_assumptions,
                        neg_assumptions,
                    ) {
                        return false;
                    }
                }
                true
            }
            Node::Or(edge_indices) => {
                for edge_index in edge_indices {
                    if self.find_model_under_assumptions_from_edge(
                        *edge_index,
                        model,
                        pos_assumptions,
                        neg_assumptions,
                    ) {
                        return true;
                    }
                }
                false
            }
            Node::True => true,
            Node::False => false,
        }
    }

    fn find_model_under_assumptions_from_edge(
        &self,
        from: EdgeIndex,
        model: &mut Vec<Literal>,
        pos_assumptions: &InvolvedVars,
        neg_assumptions: &InvolvedVars,
    ) -> bool {
        let old_model_len = model.len();
        let edge = &self.ddnnf.edges()[from];
        if edge
            .propagated()
            .iter()
            .any(|p| !is_compatible_with_assumptions(*p, pos_assumptions, neg_assumptions))
        {
            return false;
        }
        model.append(&mut edge.propagated().to_vec());
        if self.find_model_under_assumptions_from_node(
            edge.target(),
            model,
            pos_assumptions,
            neg_assumptions,
        ) {
            true
        } else {
            model.truncate(old_model_len);
            false
        }
    }
}

fn is_compatible_with_assumptions(
    l: Literal,
    pos_assumptions: &InvolvedVars,
    neg_assumptions: &InvolvedVars,
) -> bool {
    if l.polarity() {
        !neg_assumptions.is_set(l)
    } else {
        !pos_assumptions.is_set(l)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{D4Reader, DecisionDNNFReader};

    fn get_model(
        str_ddnnf: &str,
        assumptions: &[isize],
        n_vars: Option<usize>,
    ) -> Option<Vec<isize>> {
        let mut ddnnf = D4Reader::default().read(str_ddnnf.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let finder = ModelFinder::new(&ddnnf);
        let assumption_lits = assumptions
            .iter()
            .map(|i| Literal::from(*i))
            .collect::<Vec<_>>();
        let model = finder.find_model_under_assumptions(&assumption_lits);
        model.map(|m| m.into_iter().map(isize::from).collect())
    }

    fn assert_has_model(str_ddnnf: &str, assumptions: &[isize], n_vars: Option<usize>) {
        get_model(str_ddnnf, assumptions, n_vars).unwrap();
    }

    fn assert_has_no_model(str_ddnnf: &str, assumptions: &[isize], n_vars: Option<usize>) {
        assert!(get_model(str_ddnnf, assumptions, n_vars).is_none());
    }

    fn assert_model_eq(
        str_ddnnf: &str,
        assumptions: &[isize],
        mut expected: Vec<isize>,
        n_vars: Option<usize>,
    ) {
        expected.sort_unstable();
        let mut actual = get_model(str_ddnnf, assumptions, n_vars).unwrap();
        actual.sort_unstable();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_unsat() {
        let str_ddnnf = r"
        f 1 0
        ";
        assert_has_no_model(str_ddnnf, &[], None);
    }

    #[test]
    fn test_empty() {
        let str_ddnnf = r"
        t 1 0
        ";
        assert_has_model(str_ddnnf, &[], None);
    }

    #[test]
    fn test_free_var() {
        let str_ddnnf = r"
        t 1 0
        ";
        assert_has_model(str_ddnnf, &[], Some(1));
        assert_model_eq(str_ddnnf, &[-1], vec![-1], Some(1));
        assert_model_eq(str_ddnnf, &[1], vec![1], Some(1));
    }

    #[test]
    fn test_and() {
        let str_ddnnf = r"
        a 1 0
        t 2 0
        1 2 1 0
        1 2 2 0
        ";
        assert_has_model(str_ddnnf, &[], None);
        assert_has_model(str_ddnnf, &[1], None);
        assert_has_model(str_ddnnf, &[2], None);
        assert_model_eq(str_ddnnf, &[1, 2], vec![1, 2], None);
        assert_has_no_model(str_ddnnf, &[-1], None);
        assert_has_no_model(str_ddnnf, &[-2], None);
        assert_has_no_model(str_ddnnf, &[-1, 2], None);
        assert_has_no_model(str_ddnnf, &[1, -2], None);
        assert_has_no_model(str_ddnnf, &[-1, -2], None);
    }

    #[test]
    fn test_or() {
        let str_ddnnf = r"
        o 1 0
        t 2 0
        1 2 -1 -2 0
        1 2 1 2 0
        ";
        assert_has_model(str_ddnnf, &[], None);
        assert_has_model(str_ddnnf, &[1], None);
        assert_has_model(str_ddnnf, &[2], None);
        assert_model_eq(str_ddnnf, &[1, 2], vec![1, 2], None);
        assert_has_model(str_ddnnf, &[-1], None);
        assert_has_model(str_ddnnf, &[-2], None);
        assert_has_no_model(str_ddnnf, &[-1, 2], None);
        assert_has_no_model(str_ddnnf, &[1, -2], None);
        assert_model_eq(str_ddnnf, &[-1, -2], vec![-1, -2], None);
    }

    #[test]
    fn test_and_or() {
        let str_ddnnf = r"
        a 1 0
        o 2 0
        o 3 0
        t 4 0
        1 2 0
        1 3 0
        2 4 -1 0
        2 4 1 0
        3 4 -2 0
        3 4 2 0";
        assert_has_model(str_ddnnf, &[], None);
        assert_has_model(str_ddnnf, &[1], None);
        assert_has_model(str_ddnnf, &[2], None);
        assert_model_eq(str_ddnnf, &[1, 2], vec![1, 2], None);
        assert_has_model(str_ddnnf, &[-1], None);
        assert_has_model(str_ddnnf, &[-2], None);
        assert_model_eq(str_ddnnf, &[-1, 2], vec![-1, 2], None);
        assert_model_eq(str_ddnnf, &[1, -2], vec![1, -2], None);
        assert_model_eq(str_ddnnf, &[-1, -2], vec![-1, -2], None);
    }

    #[test]
    fn test_or_and() {
        let str_ddnnf = r"
        o 1 0
        a 2 0
        a 3 0
        t 4 0
        1 2 0
        1 3 0
        2 4 -1 0
        2 4 -2 0
        3 4 1 0
        3 4 2 0";
        assert_has_model(str_ddnnf, &[], None);
        assert_has_model(str_ddnnf, &[1], None);
        assert_has_model(str_ddnnf, &[2], None);
        assert_model_eq(str_ddnnf, &[1, 2], vec![1, 2], None);
        assert_has_model(str_ddnnf, &[-1], None);
        assert_has_model(str_ddnnf, &[-2], None);
        assert_has_no_model(str_ddnnf, &[-1, 2], None);
        assert_has_no_model(str_ddnnf, &[1, -2], None);
        assert_model_eq(str_ddnnf, &[-1, -2], vec![-1, -2], None);
    }

    #[test]
    #[should_panic(expected = "no such literal: -1 (the formula has 0 variables)")]
    fn test_no_such_literal() {
        let str_ddnnf = "t 1 0";
        assert_has_model(str_ddnnf, &[-1], None);
    }
}
