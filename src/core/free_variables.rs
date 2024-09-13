use crate::{
    core::{InvolvedVars, Node, NodeIndex},
    DecisionDNNF, Literal,
};

/// A structure used to computes the free variables, ie. the variables that does not appear in (some) models.
///
/// Free variables can appear in two cases: when they do not appear at all in the formula (what we call *root free variables*),
/// or when they appear in some child of a disjunction node but not all the children (what we call *OR free variables*).
/// This function computes both kinds of free variables, and returns a tuple containing first the OR free variables and then the root free variables.
///
/// The OR free variables are returned as a vector acting as a mapping from node indices to a structure (`Vec<Vec<Literal>>`) depicting the free variables when selecting a child of this node.
/// If the index belongs to a node that is not a disjunction, the structure is an empty vector.
/// In case the node is a disjunction, this vector is a mapping from the children indices to the variables that are free when this child is selected to form a model.
///
/// The root free variables are simply returned as a vector of literal.
///
/// The literals encoding the free variables are always the negative ones.
#[derive(Debug)]
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
    ///
    /// See the structure documentation for more information.
    #[must_use]
    pub fn root_free_vars(&self) -> &[Literal] {
        &self.root_free_vars
    }

    /// Returns the OR free variables.
    ///
    /// See the structure documentation for more information.
    #[must_use]
    pub fn or_free_vars(&self) -> &OrFreeVariables {
        &self.or_free_vars
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
            for l in edge.propagated() {
                involved_in_child.set_literal(*l);
            }
            involved_in_child.xor_assign(involved_vars[usize::from(from)].as_ref().unwrap());
            or_free_vars[usize::from(from)].push(involved_in_child.iter_neg_literals().collect());
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
                union.or_assign(involved_vars[usize::from(target)].as_ref().unwrap());
                for l in edge.propagated() {
                    union.set_literal(*l);
                }
            }
        }
        Node::True | Node::False => {}
    }
    union
}

/// A structure used to handle efficiently the free variables located at disjunction nodes.
///
/// This structures allows to store polarity associated with these variables, since they are stored as literals.
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
    pub fn set_negative_literals(&mut self) {
        self.data.iter_mut().for_each(Literal::set_negative);
    }

    /// Given a disjunction node index, iterates over the number of free variables each child has.
    pub fn iter_child_free_vars_lengths(&self, var: usize) -> impl Iterator<Item = usize> + '_ {
        self.indices_and_lengths[var]
            .iter()
            .map(|(_, length)| *length)
    }

    /// Iterates over the free variables of a given disjunction node child.
    #[must_use]
    pub fn child_free_vars(&self, var: usize, child_index: usize) -> &[Literal] {
        let (start, length) = self.indices_and_lengths[var][child_index];
        &self.data[start..start + length]
    }

    /// Iterates mutably over the free variables of a given disjunction node child.
    pub fn child_free_vars_mut(&mut self, var: usize, child_index: usize) -> &mut [Literal] {
        let (start, length) = self.indices_and_lengths[var][child_index];
        &mut self.data[start..start + length]
    }
}
