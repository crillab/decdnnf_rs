use crate::{
    core::{BottomUpVisitor, InvolvedVars},
    DecisionDNNF, Literal, NodeIndex,
};

/// A bottom-up visitor used for an algorithm that checks if a Decision-DNNF is correct (i.e. it is really a Decision-DNNF).
#[derive(Clone, Default)]
pub struct CheckingVisitor;

/// The data used by the [`CheckingVisitor`] structure.
#[derive(Clone)]
pub struct CheckingVisitorData {
    error: Option<String>,
    warnings: Vec<String>,
    is_false_node: bool,
    involved_vars: InvolvedVars,
}

impl CheckingVisitorData {
    fn new_error(message: String) -> Self {
        Self {
            error: Some(message),
            warnings: vec![],
            is_false_node: false,
            involved_vars: InvolvedVars::empty(),
        }
    }

    fn new_involved_vars(involved_vars: InvolvedVars) -> Self {
        Self {
            error: None,
            warnings: vec![],
            is_false_node: false,
            involved_vars,
        }
    }

    fn new_for_leaf(n_vars: usize, is_false_node: bool) -> Self {
        Self {
            error: None,
            warnings: vec![],
            is_false_node,
            involved_vars: InvolvedVars::new(n_vars),
        }
    }

    /// Return an option containing an error, if one was discovered during the traversal.
    #[must_use]
    pub fn get_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl BottomUpVisitor<CheckingVisitorData> for CheckingVisitor {
    fn merge_for_and(
        &self,
        _ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], CheckingVisitorData)>,
    ) -> CheckingVisitorData {
        if let Some(error) = get_error(&children) {
            return error;
        }
        let involved_in_children = children
            .iter()
            .map(|(propagated, child)| {
                let mut bv = child.involved_vars.clone();
                propagated.iter().for_each(|l| bv.set_literal(*l));
                bv
            })
            .collect::<Vec<_>>();
        for i in 0..involved_in_children.len() - 1 {
            for j in i + 1..involved_in_children.len() {
                let mut intersection = involved_in_children[i].clone();
                intersection.and_assign(&involved_in_children[j]);
                if intersection.any() {
                    return CheckingVisitorData::new_error(format!(
                        "AND children share variables (AND node index is {})",
                        usize::from(*path.last().unwrap())
                    ));
                }
            }
        }
        CheckingVisitorData::new_involved_vars(InvolvedVars::union(involved_in_children))
    }

    fn merge_for_or(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], CheckingVisitorData)>,
    ) -> CheckingVisitorData {
        if let Some(error) = get_error(&children) {
            return error;
        }
        let mut warnings = Vec::new();
        for i in 0..children.len() - 1 {
            if children[i].1.is_false_node {
                continue;
            }
            for j in i + 1..children.len() {
                if !children[j].1.is_false_node && !are_contradictory(children[i].0, children[j].0)
                {
                    warnings.push(format!("OR children at indices {i} and {j} may not be contradictory (OR node index is {})", usize::from(*path.last()
                .unwrap())));
                }
            }
        }
        let involved_vars = children.iter().fold(
            InvolvedVars::new(ddnnf.n_vars()),
            |mut acc, (propagated, child_data)| {
                acc.or_assign(&child_data.involved_vars);
                acc.set_literals(propagated);
                acc
            },
        );
        let mut result = CheckingVisitorData::new_involved_vars(involved_vars);
        result.warnings = warnings;
        result
    }

    fn new_for_true(&self, ddnnf: &DecisionDNNF, _path: &[NodeIndex]) -> CheckingVisitorData {
        CheckingVisitorData::new_for_leaf(ddnnf.n_vars(), false)
    }

    fn new_for_false(&self, ddnnf: &DecisionDNNF, _path: &[NodeIndex]) -> CheckingVisitorData {
        CheckingVisitorData::new_for_leaf(ddnnf.n_vars(), true)
    }
}

fn get_error(children: &[(&[Literal], CheckingVisitorData)]) -> Option<CheckingVisitorData> {
    children
        .iter()
        .position(|(_, child)| child.error.is_some())
        .map(|p| children[p].1.clone())
}

fn are_contradictory(p0: &[Literal], p1: &[Literal]) -> bool {
    p0.iter().any(|l| p1.contains(&l.flip()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::BottomUpTraversal, D4Reader};

    #[test]
    fn test_not_decomposable() {
        let str_ddnnf = "a 1 0\nt 2 0\n1 2 1 0\n1 2 -1 0";
        let ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        let traversal = BottomUpTraversal::new(Box::<CheckingVisitor>::default());
        let result = traversal.traverse(&ddnnf);
        assert_eq!(
            "AND children share variables (AND node index is 0)",
            result.error.unwrap()
        );
    }

    #[test]
    fn test_not_determinist() {
        let str_ddnnf = "o 1 0\nt 2 0\n1 2 1 0\n1 2 1 0";
        let ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        let traversal = BottomUpTraversal::new(Box::<CheckingVisitor>::default());
        let result = traversal.traverse(&ddnnf);
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
        let ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        let traversal = BottomUpTraversal::new(Box::<CheckingVisitor>::default());
        let result = traversal.traverse(&ddnnf);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_or_determinism_with_false_node() {
        let str_ddnnf = "o 1 0\nt 2 0\nf 3 0\n1 2 1 0\n1 3 0";
        let ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        let traversal = BottomUpTraversal::new(Box::<CheckingVisitor>::default());
        let result = traversal.traverse(&ddnnf);
        assert!(result.error.is_none());
    }
}
