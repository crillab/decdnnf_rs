use crate::{
    core::{InvolvedVars, Node, NodeIndex},
    DecisionDNNF, Literal,
};

/// An algorithm used for an algorithm that checks if a Decision-DNNF is correct.
///
/// The tests consists in checking the decomposability of the conjunction nodes and the determinism of the disjunction nodes.
/// When using this checker, a violation of the decomposability property will trigger an error.
/// However, the checking of the determinism is partial, in the sense a disjunction node that is determinist may not be recognized as is by this checker.
/// For this reason, potential faults on determinism simply triggers warnings.
/// Thus, even if the checking process does not returns an error, a check of the list of the warnings emitted during the search should be done.
///
/// The detection of an error stops the checking process.
/// This is not the case when a warning is raised.
///
/// # Example
///
/// ```
/// use decdnnf_rs::{DecisionDNNF, DecisionDNNFChecker};
///
/// fn check_decision_dnnf(ddnnf: &DecisionDNNF) {
///     let checking_result = DecisionDNNFChecker::check(&ddnnf);
///     if let Some(e) = checking_result.error() {
///         println!("got an error: {e}");
///     } else {
///         println!("no error detected");
///     }
///     let warnings = checking_result.warnings();
///     println!("got {} warnings", warnings.len());
///     for (i,w) in warnings.iter().enumerate() {
///         println!("warning {i}: {w}");
///     }
/// }
/// # check_decision_dnnf(&decdnnf_rs::D4Reader::default().read("t 1 0".as_bytes()).unwrap())
/// ```
#[derive(Default)]
pub struct DecisionDNNFChecker {
    error: Option<String>,
    warnings: Vec<String>,
}

impl DecisionDNNFChecker {
    /// Performs the check on the provided Decision-DNNF.
    ///
    /// Call the [`error`](DecisionDNNFChecker::error) and [`warnings`](DecisionDNNFChecker::warnings) on the returnd object the get the results.
    #[must_use]
    pub fn check(ddnnf: &DecisionDNNF) -> DecisionDNNFChecker {
        let mut result = DecisionDNNFChecker::default();
        let mut involved_vars = vec![None; ddnnf.nodes().as_slice().len()];
        let mut is_false_node = vec![false; ddnnf.nodes().as_slice().len()];
        Self::check_from(
            ddnnf,
            &mut result,
            &mut involved_vars,
            &mut is_false_node,
            0.into(),
        );
        result
    }

    /// Returns the message associated with the error discovered during the search, if any.
    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Returns the messages associated with the warnings discovered during the search, if any.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    fn check_from(
        ddnnf: &DecisionDNNF,
        result: &mut DecisionDNNFChecker,
        involved_vars: &mut Vec<Option<InvolvedVars>>,
        is_false_node: &mut [bool],
        from: NodeIndex,
    ) {
        if result.error.is_some() {
            return;
        }
        Self::check_from_children_of(ddnnf, result, involved_vars, is_false_node, from);
        let involved = match &ddnnf.nodes()[from] {
            Node::And(children) => {
                let mut union: Option<InvolvedVars> = None;
                for edge_index in children {
                    let edge = &ddnnf.edges()[*edge_index];
                    let target = usize::from(edge.target());
                    let involved_in_child = involved_vars[target].as_ref().unwrap();
                    if let Some(ref mut u) = union {
                        let mut intersection = involved_in_child.clone();
                        intersection.set_literals(edge.propagated());
                        intersection.and_assign(u);
                        if intersection.any() {
                            result.error = Some(format!(
                                "AND children share variables (AND node index is {})",
                                usize::from(from)
                            ));
                            return;
                        }
                    }
                    Self::create_or_union_with(&mut union, involved_in_child, edge.propagated());
                }
                union.unwrap()
            }
            Node::Or(children) => {
                let mut union: Option<InvolvedVars> = None;
                for (i, edge_index_i) in children.iter().enumerate().take(children.len() - 1) {
                    let edge_i = &ddnnf.edges()[*edge_index_i];
                    let target_i = usize::from(edge_i.target());
                    if is_false_node[target_i] {
                        continue;
                    }
                    for (j, edge_index_j) in children.iter().enumerate().skip(i + 1) {
                        let edge_j = &ddnnf.edges()[*edge_index_j];
                        let target_j = usize::from(edge_j.target());
                        if !is_false_node[target_j]
                            && !Self::are_contradictory(edge_i.propagated(), edge_j.propagated())
                        {
                            result.warnings.push(format!("OR children at indices {i} and {j} may not be contradictory (OR node index is {})", usize::from(from)));
                        }
                    }
                    let involved_in_child = involved_vars[target_i].as_ref().unwrap();
                    Self::create_or_union_with(&mut union, involved_in_child, edge_i.propagated());
                }
                let edge_n = &ddnnf.edges()[*children.last().unwrap()];
                let target_n = usize::from(edge_n.target());
                let involved_in_child = involved_vars[target_n].as_ref().unwrap();
                Self::create_or_union_with(&mut union, involved_in_child, edge_n.propagated());
                union.unwrap()
            }
            Node::True => InvolvedVars::new(ddnnf.n_vars()),
            Node::False => {
                is_false_node[usize::from(from)] = true;
                InvolvedVars::new(ddnnf.n_vars())
            }
        };
        involved_vars[usize::from(from)] = Some(involved);
    }

    fn check_from_children_of(
        ddnnf: &DecisionDNNF,
        result: &mut DecisionDNNFChecker,
        involved_vars: &mut Vec<Option<InvolvedVars>>,
        is_false_node: &mut [bool],
        from: NodeIndex,
    ) {
        match &ddnnf.nodes()[from] {
            Node::And(children) | Node::Or(children) => {
                for edge_index in children {
                    let edge = &ddnnf.edges()[*edge_index];
                    let target = usize::from(edge.target());
                    if involved_vars[target].is_none() {
                        Self::check_from(
                            ddnnf,
                            result,
                            involved_vars,
                            is_false_node,
                            target.into(),
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn create_or_union_with(
        opt_union: &mut Option<InvolvedVars>,
        involved: &InvolvedVars,
        propagated: &[Literal],
    ) {
        if let Some(ref mut u) = opt_union {
            u.or_assign(involved);
            u.set_literals(propagated);
        } else {
            *opt_union = Some(involved.clone());
            opt_union.as_mut().unwrap().set_literals(propagated);
        }
    }

    fn are_contradictory(p0: &[Literal], p1: &[Literal]) -> bool {
        p0.iter().any(|l| p1.contains(&l.flip()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn read_correct_ddnnf(str_ddnnf: &str) -> DecisionDNNF {
        D4Reader::default().read(&mut str_ddnnf.as_bytes()).unwrap()
    }

    #[test]
    fn test_not_decomposable() {
        let str_ddnnf = "a 1 0\nt 2 0\n1 2 1 0\n1 2 -1 0";
        let ddnnf = read_correct_ddnnf(str_ddnnf);
        let result = DecisionDNNFChecker::check(&ddnnf);
        assert_eq!(
            "AND children share variables (AND node index is 0)",
            result.error.unwrap()
        );
    }

    #[test]
    fn test_not_determinist() {
        let str_ddnnf = "o 1 0\nt 2 0\n1 2 1 0\n1 2 1 0";
        let ddnnf = read_correct_ddnnf(str_ddnnf);
        let result = DecisionDNNFChecker::check(&ddnnf);
        assert!(result.error.is_none());
        assert_eq!(
            vec!["OR children at indices 0 and 1 may not be contradictory (OR node index is 0)"],
            result.warnings
        );
    }

    #[test]
    fn test_ok() {
        let str_ddnnf =
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n";
        let ddnnf = read_correct_ddnnf(str_ddnnf);
        let result = DecisionDNNFChecker::check(&ddnnf);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_or_determinism_with_false_node() {
        let str_ddnnf = "o 1 0\nt 2 0\nf 3 0\n1 2 1 0\n1 3 0";
        let ddnnf = read_correct_ddnnf(str_ddnnf);
        let result = DecisionDNNFChecker::check(&ddnnf);
        assert!(result.error.is_none());
    }
}
