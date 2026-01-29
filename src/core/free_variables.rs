use crate::{
    core::{InvolvedVars, Node, NodeIndex},
    Assumptions, DecisionDNNF, Literal,
};

/// A structure used to computes the free variables, i.e. the variables that does not appear in some models.
///
/// Free variables can appear in two cases: when they do not appear at all in the formula, which we call *root free variables*,
/// or when they appear in some, but not all, children of a disjunction node, which we call *OR free variables*.
/// This function computes both kinds of free variables.
///
/// Variables are encoded as literals, the polarity of which must be ignored.
#[derive(Debug, Clone)]
pub struct FreeVariables {
    root_free_vars: Vec<Literal>,
    or_free_vars: OrFreeVariables,
}

impl FreeVariables {
    /// Computes the free variables of the given Decision-DNNF.
    pub(crate) fn compute(ddnnf: &DecisionDNNF) -> Self {
        let n_nodes = ddnnf.nodes().as_slice().len();
        let mut involved_vars = vec![None; n_nodes];
        let mut or_free_vars = vec![vec![]; n_nodes];
        compute_free_vars_from(
            ddnnf,
            NodeIndex::from(0),
            &mut involved_vars,
            &mut or_free_vars,
        );
        let root_free_vars = involved_vars[0]
            .as_ref()
            .unwrap()
            .iter_missing_literals()
            .collect::<Vec<_>>();
        Self {
            root_free_vars,
            or_free_vars: OrFreeVariables::build(or_free_vars),
        }
    }

    /// Returns the root free variables.
    #[must_use]
    pub fn root_free_vars(&self) -> &[Literal] {
        &self.root_free_vars
    }

    /// Returns the OR free variables.
    ///
    /// See [`OrFreeVariables`] for more information.
    #[must_use]
    pub fn or_free_vars(&self) -> &OrFreeVariables {
        &self.or_free_vars
    }

    pub(crate) fn apply_assumptions(&self, assumptions: &Assumptions) -> Self {
        Self {
            root_free_vars: self
                .root_free_vars
                .iter()
                .filter(|l| assumptions[l.var_index()].is_none())
                .copied()
                .collect(),
            or_free_vars: self.or_free_vars.apply_assumptions(assumptions),
        }
    }

    pub(crate) fn take(self) -> (Vec<Literal>, OrFreeVariables) {
        (self.root_free_vars, self.or_free_vars)
    }
}

fn compute_free_vars_from(
    ddnnf: &DecisionDNNF,
    from: NodeIndex,
    involved_vars: &mut [Option<InvolvedVars>],
    or_free_vars: &mut [Vec<Vec<Literal>>],
) {
    if involved_vars[usize::from(from)].is_some() {
        return;
    }
    involved_vars[usize::from(from)] = Some(compute_involved_vars(
        ddnnf,
        from,
        involved_vars,
        or_free_vars,
    ));
    if let Node::Or(edges) = &ddnnf.nodes()[from] {
        for edge_index in edges {
            let edge = &ddnnf.edges()[*edge_index];
            let target = edge.target();
            let mut involved_in_child =
                involved_vars[usize::from(target)].as_ref().unwrap().clone();
            involved_in_child.set_literals(edge.propagated());
            or_free_vars[usize::from(from)].push(
                InvolvedVars::iter_xor_neg_literals(
                    &involved_in_child,
                    involved_vars[usize::from(from)].as_ref().unwrap(),
                )
                .collect(),
            );
        }
    }
}

fn compute_involved_vars(
    ddnnf: &DecisionDNNF,
    node: NodeIndex,
    involved_vars: &mut [Option<InvolvedVars>],
    or_free_vars: &mut [Vec<Vec<Literal>>],
) -> InvolvedVars {
    let mut union = InvolvedVars::new(ddnnf.n_vars());
    match &ddnnf.nodes()[node] {
        Node::And(edges) | Node::Or(edges) => {
            for edge_index in edges {
                let edge = &ddnnf.edges()[*edge_index];
                let target = edge.target();
                compute_free_vars_from(ddnnf, target, involved_vars, or_free_vars);
                union |= involved_vars[usize::from(target)].as_ref().unwrap();
                union.set_literals(edge.propagated());
            }
        }
        Node::True | Node::False => {}
    }
    union
}

/// A structure used to efficiently handle the free variables located at disjunction nodes.
#[derive(Debug, Clone)]
pub struct OrFreeVariables {
    indices_and_lengths: Vec<Vec<(usize, usize)>>,
    data: Vec<Literal>,
}

impl OrFreeVariables {
    fn build(mut input_vec: Vec<Vec<Vec<Literal>>>) -> Self {
        let mut indices_and_lengths = Vec::new();
        let mut data = Vec::new();
        for var_data in &mut input_vec {
            let mut var_indices_and_lengths = Vec::with_capacity(var_data.len());
            for free_vars in var_data {
                var_indices_and_lengths.push((data.len(), free_vars.len()));
                data.append(free_vars);
            }
            indices_and_lengths.push(var_indices_and_lengths);
        }
        Self {
            indices_and_lengths,
            data,
        }
    }

    /// Sets the polarity associated with the literals as negative for all the free variables.
    pub(crate) fn set_negative_literals(&mut self) {
        self.data.iter_mut().for_each(Literal::set_negative);
    }

    /// Given a disjunction node index, iterates over the number of free variables that each child has.
    pub fn iter_child_free_vars_lengths(&self, var: usize) -> impl Iterator<Item = usize> + '_ {
        self.indices_and_lengths[var]
            .iter()
            .map(|(_, length)| *length)
    }

    /// Returns a slice of the free variables of a given disjunction node's child.
    #[must_use]
    pub fn child_free_vars(&self, var: usize, child_index: usize) -> &[Literal] {
        let (start, length) = self.indices_and_lengths[var][child_index];
        &self.data[start..start + length]
    }

    /// Iterates mutably over the free variables of a given disjunction node child.
    pub(crate) fn child_free_vars_mut(&mut self, var: usize, child_index: usize) -> &mut [Literal] {
        let (start, length) = self.indices_and_lengths[var][child_index];
        &mut self.data[start..start + length]
    }

    /// Iterates over the free variables of the children of a given disjunction.
    pub fn iter_child_free_vars(&self, var: usize) -> impl Iterator<Item = &[Literal]> + '_ {
        self.indices_and_lengths[var]
            .iter()
            .map(|(start, length)| &self.data[*start..*start + *length])
    }

    fn apply_assumptions(&self, assumptions: &Assumptions) -> Self {
        let mut new_indices_and_lengths = Vec::with_capacity(self.indices_and_lengths.len());
        let mut new_data = Vec::with_capacity(self.data.len());
        let mut offset = 0;
        for v in &self.indices_and_lengths {
            let mut new_v = Vec::with_capacity(v.len());
            for (i, l) in v {
                let mut new_data_chunk = self.data[*i..*i + *l]
                    .iter()
                    .filter(|l| assumptions[l.var_index()].is_none())
                    .copied()
                    .collect::<Vec<_>>();
                new_v.push((*i - offset, new_data_chunk.len()));
                offset += *l - new_data_chunk.len();
                new_data.append(&mut new_data_chunk);
            }
            new_indices_and_lengths.push(new_v);
        }
        new_data.shrink_to_fit();
        Self {
            indices_and_lengths: new_indices_and_lengths,
            data: new_data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    #[test]
    fn test_ok() {
        let instance = r"
                o 1 0
                o 2 0
                t 3 0
                2 3 -1 -2 0
                2 3 1 0
                1 2 0";
        let ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        let free_vars = ddnnf.free_vars();
        assert_eq!(&[] as &[Literal], free_vars.root_free_vars());
        assert_eq!(
            vec![
                vec![(0_usize, 0_usize)],
                vec![(0_usize, 0_usize), (0_usize, 1_usize)],
                vec![]
            ],
            free_vars.or_free_vars.indices_and_lengths
        );
        assert_eq!(vec![Literal::from(-2)], free_vars.or_free_vars.data);
    }

    #[test]
    fn test_no_vars() {
        let instance = "t 1 0";
        let ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        let free_vars = ddnnf.free_vars();
        assert_eq!(&[] as &[Literal], free_vars.root_free_vars());
        assert_eq!(
            vec![vec![]] as Vec<Vec<(usize, usize)>>,
            free_vars.or_free_vars.indices_and_lengths
        );
        assert!(free_vars.or_free_vars.data.is_empty());
    }

    #[test]
    fn test_one_var_nothing_free() {
        let instance = r"
                o 1 0
                t 2 0
                1 2 1 0";
        let ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        let free_vars = ddnnf.free_vars();
        assert_eq!(&[] as &[Literal], free_vars.root_free_vars());
        assert_eq!(
            vec![vec![(0_usize, 0_usize)], vec![]],
            free_vars.or_free_vars.indices_and_lengths
        );
        assert!(free_vars.or_free_vars.data.is_empty());
    }

    #[test]
    fn test_apply_assumptions_root() {
        let instance = r"
                o 1 0
                t 2 0
                1 2 3 0";
        let ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        let free_vars = ddnnf.free_vars();
        assert_eq!(
            &[Literal::from(-1), Literal::from(-2)],
            free_vars.root_free_vars()
        );
        assert_eq!(
            vec![vec![(0_usize, 0_usize)], vec![]],
            free_vars.or_free_vars.indices_and_lengths
        );
        assert!(free_vars.or_free_vars.data.is_empty());
        let free_vars_with_assumptions =
            free_vars.apply_assumptions(&Assumptions::new(3, vec![Literal::from(-1)]));
        assert_eq!(
            &[Literal::from(-2)],
            free_vars_with_assumptions.root_free_vars()
        );
        assert_eq!(
            vec![vec![(0_usize, 0_usize)], vec![]],
            free_vars_with_assumptions.or_free_vars.indices_and_lengths
        );
        assert!(free_vars_with_assumptions.or_free_vars.data.is_empty());
    }

    #[test]
    fn test_apply_assumptions_or_node() {
        let instance = r"
                o 1 0
                t 2 0
                1 2 -1 -2 0
                1 2 1 -3 0";
        let ddnnf = D4Reader::default().read(instance.as_bytes()).unwrap();
        let free_vars = ddnnf.free_vars();
        assert!(free_vars.root_free_vars().is_empty());
        assert_eq!(
            vec![vec![(0_usize, 1_usize), (1_usize, 1_usize)], vec![]],
            free_vars.or_free_vars.indices_and_lengths
        );
        assert_eq!(
            vec![Literal::from(-3), Literal::from(-2)],
            free_vars.or_free_vars.data,
        );
        let free_vars_with_assumptions =
            free_vars.apply_assumptions(&Assumptions::new(3, vec![Literal::from(-2)]));
        assert!(free_vars_with_assumptions.root_free_vars().is_empty());
        assert_eq!(
            vec![vec![(0_usize, 1_usize), (1_usize, 0_usize)], vec![]],
            free_vars_with_assumptions.or_free_vars.indices_and_lengths
        );
        assert_eq!(
            vec![Literal::from(-3)],
            free_vars_with_assumptions.or_free_vars.data,
        );
    }
}
