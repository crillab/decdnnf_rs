use crate::{
    core::{Node, NodeIndex},
    CNFFormula, DecisionDNNF, Literal, ModelCounter, ModelFinder,
};
use anyhow::{anyhow, Context, Result};
use rug::Integer;
use rustc_hash::{FxHashMap, FxHasher};
use std::{
    collections::hash_map::Entry,
    hash::{Hash, Hasher},
};

/// An object that detects node equivalences in [`DecisionDNNF`] formulas.
///
/// This object is intended to be used on formulas in which there is no equivalencies between ancestors, i.e. there is no conjunction or disjunction node with a single edge on which there is no propagated literals.
/// If you are ensure this property holds, you can remove such equivalences thanks to the [`EquivalentNodesMerging::merge_equivalent_ancestors`] function.
///
/// It proceeds by performing a DFS, computing a key for each node it passes by using the number of models contained in the subformula and the variables it involves.
/// In case this key corresponds to the one of another node, an extensive equivalence search is executed.
pub struct EquivalentNodesMerging {
    equivalences: Vec<Option<NodeIndex>>,
    has_equivalence: Vec<NodeIndex>,
}

impl EquivalentNodesMerging {
    /// Merge equivalent parent/child nodes.
    #[allow(clippy::missing_panics_doc)]
    pub fn merge_equivalent_ancestors(dnnf: &mut DecisionDNNF) -> usize {
        let mut n_removed = 0;
        let mut i = 0;
        let n_nodes = dnnf.n_nodes();
        while i < n_nodes {
            let eq_child = match &dnnf.nodes()[NodeIndex::from(i)] {
                Node::And(items) | Node::Or(items) => {
                    if items.len() == 1 {
                        let edge = &dnnf.edges()[*items.first().unwrap()];
                        if edge.propagated().is_empty() {
                            Some(edge.target())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Node::True | Node::False => None,
            };
            if let Some(child_index) = eq_child {
                dnnf.nodes_mut().as_mut_slice()[i] = dnnf.nodes()[child_index].clone();
                n_removed += 1;
            } else {
                i += 1;
            }
        }
        n_removed
    }

    /// Builds a new equivalence computer given model counter for the formula we are interested in.
    ///
    /// The [`DecisionDNNF`] borrowed by the model counter must be equivalent to the CNF formula.
    ///
    /// # Errors
    ///
    /// This object is intended to be used on formulas in which there is no equivalencies between ancestors, i.e. there is no conjunction or disjunction node with a single edge on which there is no propagated literals.
    /// If it is not the case, an error is returned.
    /// If you are ensure this property holds, you can remove such equivalences thanks to the [`EquivalentNodesMerging::merge_equivalent_ancestors`] function.
    pub fn search(model_counter: &ModelCounter, cnf: &CNFFormula) -> Result<Self> {
        let mut eq_search_data = EquivalenceSearchData::new(model_counter, cnf);
        eq_search_data
            .search_from(0.into())
            .context("while searching equivalent nodes")?;
        Ok(Self::from(eq_search_data))
    }

    /// Replaces the equivalences in the formula.
    pub fn replace_in_formula(&self, ddnnf: &mut DecisionDNNF) {
        if self.has_equivalence.is_empty() {
            return;
        }
        for edge in ddnnf.edges_mut().as_mut_slice() {
            if let Some(replacement) = self.equivalences[usize::from(edge.target())] {
                edge.set_target(replacement);
            }
        }
    }

    /// Returns the number of equivalences discovered by the algorithm.
    #[must_use]
    pub fn n_equivalences(&self) -> usize {
        self.has_equivalence.len()
    }

    /// Returns the set of equivalence as couples.
    ///
    /// A couple `(x,y)` yielded by this function means that node at index `x` has been proved as equivalent to the node at index `y`.
    /// The way the algorithm proceeds ensures that a node index which is the first element of the couple cannot appear in another couple.
    #[allow(clippy::missing_panics_doc)]
    pub fn equivalences(&self) -> impl Iterator<Item = (NodeIndex, NodeIndex)> + '_ {
        self.has_equivalence
            .iter()
            .map(|index| (self.equivalences[usize::from(*index)].unwrap(), *index))
    }
}

type NodeData<'a> = (&'a Integer, u64);

type NodeDataMap<'a> = FxHashMap<NodeData<'a>, Vec<(NodeIndex, Vec<Literal>)>>;

struct EquivalenceSearchData<'a> {
    model_counter: &'a ModelCounter<'a>,
    cnf: &'a CNFFormula,
    equivalences: Vec<Option<NodeIndex>>,
    has_equivalence: Vec<NodeIndex>,
    node_seen: Vec<bool>,
    node_cache: NodeDataMap<'a>,
    propagated_literals: Vec<Literal>,
}

impl<'a> EquivalenceSearchData<'a> {
    fn new(model_counter: &'a ModelCounter, cnf: &'a CNFFormula) -> Self {
        let n_nodes = model_counter.ddnnf().n_nodes();
        Self {
            model_counter,
            cnf,
            equivalences: vec![None; n_nodes],
            has_equivalence: vec![],
            node_seen: vec![false; n_nodes],
            node_cache: FxHashMap::default(),
            propagated_literals: vec![],
        }
    }

    fn push_propagated(&mut self, literals: &[Literal]) {
        self.propagated_literals.extend(literals);
    }

    fn pop_propagated(&mut self, n: usize) {
        self.propagated_literals
            .truncate(self.propagated_literals.len() - n);
    }

    fn search_from(&mut self, node_index: NodeIndex) -> Result<()> {
        if self.node_seen[usize::from(node_index)] {
            return Ok(());
        }
        self.node_seen[usize::from(node_index)] = true;
        let ddnnf = self.model_counter.ddnnf();
        match &ddnnf.nodes()[node_index] {
            Node::And(children_indices) | Node::Or(children_indices) => {
                let model_count = self.model_counter.count_from(node_index);
                let mut hasher = FxHasher::default();
                for int_data in ddnnf.free_vars().involved_vars(node_index).data() {
                    int_data.hash(&mut hasher);
                }
                let involved_vars_hash = hasher.finish();
                if children_indices.len() == 1
                    && ddnnf.edges()[children_indices[0]].propagated().is_empty()
                {
                    return Err(anyhow!("a child must not be equivalent to its ancestor for equivalent nodes merging to work; see documentation"));
                }
                let key = (model_count, involved_vars_hash);
                if let Some(candidates) = self.node_cache.get(&key) {
                    for (candidate, candidate_propagated) in candidates {
                        if self.are_equiv(
                            node_index,
                            &self.propagated_literals,
                            *candidate,
                            candidate_propagated,
                        ) {
                            self.equivalences[usize::from(node_index)] = Some(*candidate);
                            self.has_equivalence.push(node_index);
                            return Ok(());
                        }
                    }
                }
                let propagated = self.propagated_literals.clone();
                match self.node_cache.entry(key) {
                    Entry::Occupied(mut e) => {
                        e.get_mut().push((node_index, propagated));
                    }
                    Entry::Vacant(e) => {
                        e.insert(vec![(node_index, propagated)]);
                    }
                }
                for edge_index in children_indices {
                    let edge = &ddnnf.edges()[*edge_index];
                    self.push_propagated(edge.propagated());
                    self.search_from(edge.target())?;
                    self.pop_propagated(edge.propagated().len());
                }
            }
            Node::True | Node::False => {}
        }
        Ok(())
    }

    fn are_equiv(
        &self,
        n1: NodeIndex,
        propagated1: &[Literal],
        n2: NodeIndex,
        propagated2: &[Literal],
    ) -> bool {
        self.is_implied(propagated1, n2) && self.is_implied(propagated2, n1)
    }

    fn is_implied(&self, propagated1: &[Literal], n2: NodeIndex) -> bool {
        let subdnnf = self.model_counter.ddnnf().subformula(n2);
        let involved_vars = self.model_counter.ddnnf().free_vars().involved_vars(n2);
        let model_finder = ModelFinder::new(&subdnnf);
        let mut propagations = Propagations::new(self.model_counter.ddnnf().n_vars());
        propagations.push_propagated(propagated1);
        let mut assumptions = vec![];
        'for_each_clause: for cl in self.cnf.iter_clauses() {
            assumptions.clear();
            assumptions.extend_from_slice(cl);
            let mut involved_in_subformula = false;
            let mut i = 0;
            while i < assumptions.len() {
                match propagations.is_propagated(assumptions[i]) {
                    Some(true) => continue 'for_each_clause,
                    Some(false) => {
                        assumptions.swap_remove(i);
                    }
                    None => {
                        if involved_vars.is_set(assumptions[i]) {
                            involved_in_subformula = true;
                        }
                        assumptions[i] = assumptions[i].flip();
                        i += 1;
                    }
                }
            }
            if !assumptions.is_empty()
                && involved_in_subformula
                && model_finder
                    .find_model_under_assumptions(&assumptions)
                    .is_some()
            {
                return false;
            }
        }
        true
    }
}

impl From<EquivalenceSearchData<'_>> for EquivalentNodesMerging {
    fn from(value: EquivalenceSearchData) -> Self {
        Self {
            equivalences: value.equivalences,
            has_equivalence: value.has_equivalence,
        }
    }
}

struct Propagations {
    propagated_literals: Vec<Literal>,
    pos_lit_is_propagated: Vec<bool>,
    neg_lit_is_propagated: Vec<bool>,
}

impl Propagations {
    fn new(n_vars: usize) -> Self {
        Self {
            propagated_literals: vec![],
            pos_lit_is_propagated: vec![false; n_vars],
            neg_lit_is_propagated: vec![false; n_vars],
        }
    }

    fn push_propagated(&mut self, literals: &[Literal]) {
        for l in literals {
            self.propagated_literals.push(*l);
            let var_index = l.var_index();
            if l.polarity() {
                debug_assert!(!self.pos_lit_is_propagated[var_index]);
                debug_assert!(!self.neg_lit_is_propagated[var_index]);
                self.pos_lit_is_propagated[var_index] = true;
            } else {
                debug_assert!(!self.pos_lit_is_propagated[var_index]);
                debug_assert!(!self.neg_lit_is_propagated[var_index]);
                self.neg_lit_is_propagated[var_index] = true;
            }
        }
    }

    fn is_propagated(&self, lit: Literal) -> Option<bool> {
        if lit.polarity() {
            if self.pos_lit_is_propagated[lit.var_index()] {
                Some(true)
            } else if self.neg_lit_is_propagated[lit.var_index()] {
                Some(false)
            } else {
                None
            }
        } else if self.neg_lit_is_propagated[lit.var_index()] {
            Some(true)
        } else if self.pos_lit_is_propagated[lit.var_index()] {
            Some(false)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::Edge, Literal, OrphanFinder};

    #[test]
    fn test_equiv_is_ancestor() {
        let mut ddnnf = DecisionDNNF::from_raw_data(
            1,
            vec![
                Node::Or(vec![0.into()]),
                Node::Or(vec![1.into()]),
                Node::Or(vec![2.into()]),
                Node::True,
            ],
            vec![
                Edge::from_raw_data(1.into(), vec![]),
                Edge::from_raw_data(2.into(), vec![]),
                Edge::from_raw_data(3.into(), vec![Literal::from(1)]),
            ],
        );
        EquivalentNodesMerging::merge_equivalent_ancestors(&mut ddnnf);
        let orphan_finder = OrphanFinder::search(&ddnnf).unwrap();
        orphan_finder.remove_from_formula(&mut ddnnf);
        assert_eq!(
            &[Node::Or(vec![0.into()]), Node::True,],
            ddnnf.nodes().as_slice(),
        );
        assert_eq!(
            &[Edge::from_raw_data(1.into(), vec![Literal::from(1)]),],
            ddnnf.edges().as_slice(),
        );
    }

    #[test]
    fn test_propagations() {
        let mut propagations = Propagations::new(3);
        propagations.push_propagated(&[Literal::from(1), Literal::from(-2)]);
        assert_eq!(Some(false), propagations.is_propagated(Literal::from(-1)));
        assert_eq!(Some(true), propagations.is_propagated(Literal::from(1)));
        assert_eq!(Some(true), propagations.is_propagated(Literal::from(-2)));
        assert_eq!(Some(false), propagations.is_propagated(Literal::from(2)));
        assert_eq!(None, propagations.is_propagated(Literal::from(-3)));
        assert_eq!(None, propagations.is_propagated(Literal::from(3)));
    }

    #[test]
    fn test_equiv() {
        let ddnnf = DecisionDNNF::from_raw_data(
            3,
            vec![
                Node::Or(vec![0.into(), 1.into()]),
                Node::And(vec![2.into()]),
                Node::And(vec![3.into()]),
                Node::True,
            ],
            vec![
                Edge::from_raw_data(1.into(), vec![Literal::from(-1)]),
                Edge::from_raw_data(2.into(), vec![Literal::from(1)]),
                Edge::from_raw_data(3.into(), vec![Literal::from(2), Literal::from(3)]),
                Edge::from_raw_data(3.into(), vec![Literal::from(2), Literal::from(3)]),
            ],
        );
        let cnf = CNFFormula::from_data(
            3,
            [-1, 2, 3, 1, 2, 3]
                .iter()
                .map(|i| Literal::from(*i))
                .collect::<Vec<_>>(),
            vec![0, 3],
        );
        let model_counter = ModelCounter::new(&ddnnf, false);
        let eq_finder = EquivalentNodesMerging::search(&model_counter, &cnf).unwrap();
        assert_eq!(
            vec![(1.into(), 2.into())],
            eq_finder.equivalences().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn test_equiv_clause_with_single_free_var() {
        let ddnnf = DecisionDNNF::from_raw_data(
            3,
            vec![
                Node::And(vec![0.into(), 1.into()]),
                Node::True,
                Node::Or(vec![2.into(), 3.into()]),
                Node::And(vec![4.into()]),
                Node::And(vec![5.into()]),
            ],
            vec![
                Edge::from_raw_data(1.into(), vec![Literal::from(1)]),
                Edge::from_raw_data(2.into(), vec![]),
                Edge::from_raw_data(3.into(), vec![Literal::from(-2)]),
                Edge::from_raw_data(4.into(), vec![Literal::from(2)]),
                Edge::from_raw_data(1.into(), vec![Literal::from(3)]),
                Edge::from_raw_data(1.into(), vec![Literal::from(3)]),
            ],
        );
        let cnf = CNFFormula::from_data(
            3,
            [1, -2, 3, 2, 3]
                .iter()
                .map(|i| Literal::from(*i))
                .collect::<Vec<_>>(),
            vec![0, 1, 3],
        );
        let model_counter = ModelCounter::new(&ddnnf, false);
        let eq_finder = EquivalentNodesMerging::search(&model_counter, &cnf).unwrap();
        assert_eq!(
            vec![(3.into(), 4.into())],
            eq_finder.equivalences().collect::<Vec<_>>(),
        );
    }
}
