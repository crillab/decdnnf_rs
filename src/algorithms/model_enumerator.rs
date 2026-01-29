use crate::{
    core::{EdgeIndex, Node, NodeIndex},
    Assumptions, DecisionDNNF, DirectAccessEngine, Literal, OrFreeVariables,
};
use rug::Integer;
use std::rc::Rc;

/// A structure used to enumerate the models of a [`DecisionDNNF`].
///
/// After creating an enumerator with the [`new`](Self::new) function, call [`compute_next_model`](Self::compute_next_model) until you receive [`None`].
/// Each call that is not [`None`] returns a model, in which free variables may be eluded (see below).
/// The algorithm takes time that is polynomial in the number of models and space that is polynomial in the size of the Decision-DNNF.
///
/// When creating the enumerator, indicate whether you want to elude the free variables.
/// If you choose not to elude the free variables, the algorithm will perform a traditional enumeration.
/// The models returned by [`compute_next_model`](Self::compute_next_model) will contain exactly one literal by variable (i.e. no literal will be [`None`]).
///
/// If you choose to elude the free variables, then they will be absent from the models (replaced by [`None`]).
/// In this case, the algorithm won't produce one model for each literal polarity, but rather one model in which the variable is absent.
/// Eluding free variables results in shorter enumerations since each partial model represents a number of models equal to two to the power of the number of eluded variables.
/// For more information on this kind of enumeration, see the research paper *[Leveraging Decision-DNNF Compilation for Enumerating Disjoint Partial Models](https://doi.org/10.24963/kr.2024/48))*.
///
/// # Examples
///
/// Printing models like SAT solvers:
///
/// ```
/// use decdnnf_rs::{DecisionDNNF, ModelEnumerator};
///
/// fn print_models(ddnnf: &DecisionDNNF) {
///     let mut model_enumerator = ModelEnumerator::new(&ddnnf, false);
///     while let Some(model) = model_enumerator.compute_next_model() {
///         print!("v");
///         for opt_l in model {
///             if let Some(l) = opt_l {
///                 print!(" {}", isize::from(*l));
///             }
///         }
///         println!(" 0");
///     }
/// }
/// # print_models(&decdnnf_rs::D4Reader::default().read("t 1 0".as_bytes()).unwrap())
/// ```
///
/// Free variables elusion:
///
/// ```
/// use decdnnf_rs::{D4Reader, Literal, ModelEnumerator};
///
/// // A Decision-DNNF with two models: -1 2 and 1 2
/// let ddnnf = D4Reader::default().read(r"
/// a 1 0
/// t 2 0
/// 1 2 2 0
/// ".as_bytes()).unwrap();
///
/// // a model sorting function, for comparison purpose
/// let sort = |models: &mut [Vec<isize>]| {
///     models.iter_mut().for_each(|m| {
///         m.sort_unstable_by_key(|l| l.unsigned_abs());
///     });
///     models.sort_unstable();
/// };
///
/// // no free variable elusion
/// let mut enumerator = ModelEnumerator::new(&ddnnf, false);
/// let mut models: Vec<Vec<isize>> = Vec::new();
/// while let Some(model) = enumerator.compute_next_model() {
///     models.push(model.iter().filter_map(|opt_l| opt_l.map(|l| isize::from(l))).collect());
/// }
/// sort(&mut models);
/// let mut expected = vec![vec![-1, 2], vec![1, 2]];
/// sort(&mut expected);
/// assert_eq!(expected, models);
///
/// // with free variable elusion
/// let mut enumerator = ModelEnumerator::new(&ddnnf, true);
/// let mut models: Vec<Vec<isize>> = Vec::new();
/// while let Some(model) = enumerator.compute_next_model() {
///     models.push(model.iter().filter_map(|opt_l| opt_l.map(|l| isize::from(l))).collect());
/// }
/// assert_eq!(vec![vec![2]], models);
/// ```
#[derive(Debug, Clone)]
pub struct ModelEnumerator<'a> {
    ddnnf: &'a DecisionDNNF,
    or_edge_indices: Vec<usize>,
    or_free_vars_assignments: OrFreeVariables,
    root_free_vars_assignment: Vec<Literal>,
    first_computed: bool,
    model: Vec<Option<Literal>>,
    has_model: bool,
    elude_free_vars: bool,
    assumptions: Option<Rc<Assumptions>>,
}

impl<'a> ModelEnumerator<'a> {
    /// Builds a new model enumerator for a [`DecisionDNNF`].
    ///
    /// The second parameter specifies whether free variables should be excluded from models.
    /// See the top-level [`ModelEnumerator`] documentation for more information about free variable elusion.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(ddnnf: &'a DecisionDNNF, elude_free_vars: bool) -> Self {
        let n_nodes = ddnnf.nodes().as_slice().len();
        let free_vars = ddnnf.free_vars();
        let root_free_vars_assignment = free_vars.root_free_vars().to_vec();
        let or_free_vars_assignments = free_vars.or_free_vars().clone();
        let mut model = vec![None; ddnnf.n_vars()];
        assert!(Self::update_model_with_propagations(
            &mut model,
            &root_free_vars_assignment,
            elude_free_vars,
            None,
        ));
        Self {
            ddnnf,
            or_edge_indices: vec![0; n_nodes],
            or_free_vars_assignments,
            root_free_vars_assignment,
            first_computed: false,
            model,
            has_model: true,
            elude_free_vars,
            assumptions: None,
        }
    }

    /// Set assumption literals, reducing the number of models.
    ///
    /// The only models to be considered are those that contain all the literals marked as assumptions.
    /// The set of assumptions must involve each variable at most once.
    ///
    /// The enumeration process is [`reset`](Self::reset) by a call to this method.
    ///
    /// # Panics
    ///
    /// This function panics if the set of assumptions includes the same variable more than once.
    pub fn set_assumptions(&mut self, assumptions: Rc<Assumptions>) {
        let n_nodes = self.ddnnf.nodes().as_slice().len();
        self.or_edge_indices = vec![0; n_nodes];
        self.first_computed = false;
        self.model = vec![None; self.ddnnf.n_vars()];
        self.has_model = true;
        for a in assumptions.as_slice() {
            self.model[a.var_index()] = Some(*a);
        }
        let (root_free_vars, or_free_vars) = self
            .ddnnf
            .free_vars()
            .apply_assumptions(&assumptions)
            .take();
        self.assumptions = Some(assumptions);
        self.root_free_vars_assignment = root_free_vars;
        self.or_free_vars_assignments = or_free_vars;
        assert!(Self::update_model_with_propagations(
            &mut self.model,
            &self.root_free_vars_assignment,
            self.elude_free_vars,
            self.assumptions.as_deref(),
        ));
    }

    /// Returns the assumptions set by the [`set_assumptions`](Self::set_assumptions) method.
    ///
    /// The assumptions are returned as an [`Option`] indicating whether assumptions have been set.
    pub fn assumptions(&self) -> Option<Rc<Assumptions>> {
        self.assumptions.as_ref().map(Rc::clone)
    }

    /// Resets this enumerator as if it had just been created.
    #[allow(clippy::missing_panics_doc)]
    pub fn reset(&mut self) {
        let n_nodes = self.ddnnf.nodes().as_slice().len();
        self.or_edge_indices = vec![0; n_nodes];
        let free_vars = self.ddnnf.free_vars();
        self.or_free_vars_assignments = free_vars.or_free_vars().clone();
        self.root_free_vars_assignment = free_vars.root_free_vars().to_vec();
        self.first_computed = false;
        let mut model = vec![None; self.ddnnf.n_vars()];
        self.assumptions = None;
        assert!(Self::update_model_with_propagations(
            &mut model,
            &self.root_free_vars_assignment,
            self.elude_free_vars,
            None,
        ));
        self.model = model;
        self.has_model = true;
    }

    /// Loads the model which the given id thanks to a [`DirectAccessEngine`].
    ///
    /// This enumerator is set to the state it would be if it enumerated the models from the first to the one with the given id.
    ///
    /// # Panics
    ///
    /// This function panics if the [`DirectAccessEngine`] does not refer to the same [`DecisionDNNF`] as this object.
    /// It also panics if the [`ModelCounter`](crate::ModelCounter) referenced by the [`DirectAccessEngine`] and the [`ModelEnumerator`] do not share the same assumptions (their [`Rc`] must point to the same [`Assumptions`] object).
    pub fn jump_to(
        &mut self,
        direct_access_engine: &DirectAccessEngine,
        model_id: Integer,
    ) -> Option<&[Option<Literal>]> {
        assert!(
            std::ptr::addr_eq(direct_access_engine.ddnnf(), self.ddnnf),
            "model enumerator and direct access engine do not refer to the same Decision-DNNF"
        );
        assert_eq!(
            direct_access_engine.model_counter().assumptions().is_some(),
            self.assumptions.is_some(),
            "direct access engines (via their model counter) and model enumerators must share the same assumptions object",
        );
        if let Some(a0) = direct_access_engine.model_counter().assumptions() {
            assert!(Rc::ptr_eq(&a0, self.assumptions.as_ref().unwrap()), "direct access engines (via their model counter) and model enumerators must share the same assumptions object");
        }
        let opt_model = direct_access_engine.model_with_graph(model_id);
        if let Some((model, or_edge_indices)) = opt_model {
            self.model = model;
            self.has_model = true;
            self.or_edge_indices = or_edge_indices;
            self.first_computed = true;
            if !self.elude_free_vars {
                for l in &mut self.root_free_vars_assignment {
                    *l = self.model[l.var_index()].unwrap();
                }
                self.or_free_vars_assignments.set_negative_literals();
                self.update_or_free_vars_assignments_from(0.into());
            }
            Some(&self.model)
        } else {
            self.has_model = false;
            None
        }
    }

    fn update_or_free_vars_assignments_from(&mut self, from: NodeIndex) {
        match &self.ddnnf.nodes()[usize::from(from)] {
            Node::And(edges) => {
                for edge in edges {
                    self.update_or_free_vars_assignments_from(
                        self.ddnnf.edges()[usize::from(*edge)].target(),
                    );
                }
            }
            Node::Or(edges) => {
                let selected_child_index = self.or_edge_indices[usize::from(from)];
                for l in self
                    .or_free_vars_assignments
                    .child_free_vars_mut(usize::from(from), selected_child_index)
                {
                    *l = self.model[l.var_index()].unwrap();
                }
                self.update_or_free_vars_assignments_from(
                    self.ddnnf.edges()[edges[selected_child_index]].target(),
                );
            }
            Node::True | Node::False => {}
        }
    }

    /// Computes the next model and returns it.
    /// Returns `None` if all the models have been returned.
    pub fn compute_next_model(&mut self) -> Option<&[Option<Literal>]> {
        if !self.first_computed {
            return self.compute_first_model();
        }
        if !self.has_model {
            return None;
        }
        if !Self::next_free_vars_interpretation(
            &mut self.model,
            &mut self.root_free_vars_assignment,
            self.elude_free_vars,
            self.assumptions.as_deref(),
        ) && !self.next_path_from(NodeIndex::from(0))
        {
            self.has_model = false;
            None
        } else {
            Some(&self.model)
        }
    }

    fn compute_first_model(&mut self) -> Option<&[Option<Literal>]> {
        self.first_computed = true;
        if self.first_path_from(NodeIndex::from(0)) {
            self.has_model = true;
            Some(&self.model)
        } else {
            self.has_model = false;
            None
        }
    }

    fn next_path_from(&mut self, from: NodeIndex) -> bool {
        match &self.ddnnf.nodes()[from] {
            Node::And(edges) => {
                for edge_index in edges.iter().rev() {
                    let edge = &self.ddnnf.edges()[*edge_index];
                    if self.next_path_from(edge.target()) {
                        return true;
                    }
                    self.first_path_from(edge.target());
                }
                false
            }
            Node::Or(edges) => {
                let mut child_index = self.or_edge_indices[usize::from(from)];
                if self.next_or_node_free_vars_interpretation(from, child_index) {
                    return true;
                }
                let edge = &self.ddnnf.edges()[edges[child_index]];
                if !Self::update_model_with_propagations(
                    &mut self.model,
                    edge.propagated(),
                    false,
                    self.assumptions.as_deref(),
                ) {
                    return false;
                }
                if self.next_path_from(edge.target()) {
                    return true;
                }
                loop {
                    if child_index == edges.len() - 1 {
                        return false;
                    }
                    child_index += 1;
                    self.or_edge_indices[usize::from(from)] += 1;
                    if self.update_or_edge(from, edges[child_index]) {
                        break;
                    }
                }
                true
            }
            Node::True | Node::False => false,
        }
    }

    fn next_or_node_free_vars_interpretation(
        &mut self,
        or_node: NodeIndex,
        child_index: usize,
    ) -> bool {
        Self::next_free_vars_interpretation(
            &mut self.model,
            self.or_free_vars_assignments
                .child_free_vars_mut(usize::from(or_node), child_index),
            self.elude_free_vars,
            self.assumptions.as_deref(),
        )
    }

    fn next_free_vars_interpretation(
        model: &mut [Option<Literal>],
        interpretation: &mut [Literal],
        elude_free_vars: bool,
        assumptions: Option<&Assumptions>,
    ) -> bool {
        if elude_free_vars {
            return false;
        }
        let has_next = if let Some(p) = interpretation.iter().rposition(|l| !l.polarity()) {
            interpretation
                .iter_mut()
                .skip(p)
                .for_each(|l| *l = l.flip());
            true
        } else {
            for l in interpretation.iter_mut() {
                *l = l.flip();
            }
            false
        };
        assert!(Self::update_model_with_propagations(
            model,
            interpretation,
            false,
            assumptions
        ));
        has_next
    }

    fn first_path_from(&mut self, from: NodeIndex) -> bool {
        self.or_edge_indices[usize::from(from)] = 0;
        match &self.ddnnf.nodes()[from] {
            Node::And(edges) => {
                for edge_index in edges {
                    let edge = &self.ddnnf.edges()[*edge_index];
                    if !Self::update_model_with_propagations(
                        &mut self.model,
                        edge.propagated(),
                        false,
                        self.assumptions.as_deref(),
                    ) {
                        return false;
                    }
                    if !self.first_path_from(edge.target()) {
                        return false;
                    }
                }
                true
            }
            Node::Or(edges) => {
                for edge_index in edges {
                    if self.update_or_edge(from, *edge_index) {
                        return true;
                    }
                    self.or_edge_indices[usize::from(from)] += 1;
                }
                false
            }
            Node::True => true,
            Node::False => false,
        }
    }

    fn update_or_edge(&mut self, or_node_index: NodeIndex, edge_index: EdgeIndex) -> bool {
        let edge = &self.ddnnf.edges()[edge_index];
        let or_free_vars = self.or_free_vars_assignments.child_free_vars(
            usize::from(or_node_index),
            self.or_edge_indices[usize::from(or_node_index)],
        );
        assert!(Self::update_model_with_propagations(
            &mut self.model,
            or_free_vars,
            self.elude_free_vars,
            self.assumptions.as_deref(),
        ));
        if !Self::update_model_with_propagations(
            &mut self.model,
            edge.propagated(),
            false,
            self.assumptions.as_deref(),
        ) {
            return false;
        }
        self.first_path_from(edge.target())
    }

    #[must_use]
    fn update_model_with_propagations(
        model: &mut [Option<Literal>],
        propagations: &[Literal],
        update_with_none: bool,
        assumptions: Option<&Assumptions>,
    ) -> bool {
        if let Some(assumps) = assumptions {
            for p in propagations {
                if let Some(b) = assumps[p.var_index()] {
                    if b != p.polarity() {
                        return false;
                    }
                }
            }
        }
        for p in propagations {
            model[p.var_index()] = if update_with_none { None } else { Some(*p) };
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{D4Reader, ModelCounter};

    fn assert_models_eq(
        str_ddnnf: &str,
        expected: &[Vec<isize>],
        n_vars: Option<usize>,
        hide_free_vars: bool,
    ) {
        let mut ddnnf = D4Reader::default().read(str_ddnnf.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let mut model_enum = ModelEnumerator::new(&ddnnf, hide_free_vars);
        assert_models_eq_with_enumerator(&mut model_enum, expected, hide_free_vars);
    }

    fn assert_models_eq_with_enumerator(
        model_enum: &mut ModelEnumerator,
        expected: &[Vec<isize>],
        hide_free_vars: bool,
    ) {
        assert_models_eq_with_enumerator_no_jump(model_enum, expected);
        let mut model_counter = ModelCounter::new(model_enum.ddnnf, false);
        if let Some(a) = model_enum.assumptions() {
            model_counter.set_assumptions(a);
        }
        let direct_access = DirectAccessEngine::new(&model_counter);
        if hide_free_vars {
            return;
        }
        for i in 0..expected.len() {
            assert_models_eq_after_jump(
                model_enum.ddnnf,
                &expected[i + 1..],
                hide_free_vars,
                &direct_access,
                i,
            );
        }
    }

    fn assert_models_eq_with_enumerator_no_jump(
        model_enum: &mut ModelEnumerator,
        expected: &[Vec<isize>],
    ) {
        let mut actual = Vec::new();
        while let Some(m) = model_enum.compute_next_model() {
            actual.push(
                m.iter()
                    .filter_map(|opt_l| opt_l.map(isize::from))
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(expected, actual);
    }

    fn assert_models_eq_after_jump(
        ddnnf: &DecisionDNNF,
        expected: &[Vec<isize>],
        hide_free_vars: bool,
        direct_access_engine: &DirectAccessEngine,
        model_id: usize,
    ) {
        let mut model_enum = ModelEnumerator::new(ddnnf, hide_free_vars);
        if let Some(a) = direct_access_engine.model_counter().assumptions() {
            model_enum.set_assumptions(a);
        }
        model_enum.jump_to(direct_access_engine, model_id.into());
        let mut actual = Vec::new();
        while let Some(m) = model_enum.compute_next_model() {
            actual.push(
                m.iter()
                    .filter_map(|opt_l| opt_l.map(isize::from))
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(expected, actual, "for model {model_id}");
    }

    #[test]
    fn test_unsat() {
        assert_models_eq("f 1 0\n", &[], None, false);
    }

    #[test]
    fn test_single_model() {
        assert_models_eq("a 1 0\nt 2 0\n1 2 1 0\n", &[vec![1]], None, false);
    }

    #[test]
    fn test_tautology() {
        assert_models_eq("t 1 0\n", &[vec![-1], vec![1]], Some(1), false);
    }

    #[test]
    fn test_tautology_two_vars() {
        assert_models_eq(
            "t 1 0\n",
            &[vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            Some(2),
            false,
        );
    }

    #[test]
    fn test_or() {
        assert_models_eq(
            "o 1 0\nt 2 0\n1 2 -1 0\n 1 2 1 0\n",
            &[vec![-1], vec![1]],
            None,
            false,
        );
    }

    #[test]
    fn test_or_with_free_var() {
        assert_models_eq(
            "o 1 0\nt 2 0\n1 2 -1 0\n 1 2 1 0\n",
            &[vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            Some(2),
            false,
        );
    }

    #[test]
    fn test_and() {
        assert_models_eq(
            "a 1 0\nt 2 0\n1 2 -1 0\n 1 2 -2 0\n",
            &[vec![-1, -2]],
            None,
            false,
        );
    }

    #[test]
    fn test_and_or() {
        assert_models_eq(
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n",
            &[vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            None,
            false,
        );
    }

    #[test]
    fn test_or_and() {
        assert_models_eq(
            "o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 -2 0\n3 4 1 0\n3 4 2 0\n",
            &[vec![-1, -2], vec![1, 2]],
            None,
            false,
        );
    }

    #[test]
    fn test_2_vars_3_models() {
        assert_models_eq(
            r"o 1 0
            o 2 0
            t 3 0
            2 3 -1 -2 0
            2 3 1 0
            1 2 0
            ",
            &[vec![-1, -2], vec![1, -2], vec![1, 2]],
            None,
            false,
        );
    }

    #[test]
    fn test_implied_lit() {
        assert_models_eq(
            r"o 1 0
            o 2 0
            t 3 0
            f 4 0
            2 3 -1 0
            2 4 1 0
            1 2 0
            ",
            &[vec![-1, -2], vec![-1, 2]],
            Some(2),
            false,
        );
    }

    #[test]
    fn test_hide_free_var_tautology() {
        assert_models_eq("t 1 0", &[vec![]], Some(2), true);
    }

    #[test]
    fn test_counter_models() {
        assert_models_eq(
            r"o 1 0
            t 2 0
            f 3 0
            1 2 -1 -2 0
            1 3 -1 2 0
            1 3 1 -2 0
            1 2 1 2 0
            ",
            &[vec![-1, -2], vec![1, 2]],
            None,
            false,
        );
    }

    #[test]
    fn test_jump_sets_unassigned_or_free_vars_negatively() {
        assert_models_eq(
            r"o 1 0
            o 2 0
            t 3 0
            o 4 0
            4 3 -1 0
            4 3 1 3 0
            2 3 -2 3 0
            2 4 2 0
            1 2 0
            ",
            &[
                vec![-1, -2, 3],
                vec![1, -2, 3],
                vec![-1, 2, -3],
                vec![-1, 2, 3],
                vec![1, 2, 3],
            ],
            None,
            false,
        );
    }

    #[test]
    fn test_reset_root_free_vars() {
        let mut ddnnf = D4Reader::default().read("t 1 0\n".as_bytes()).unwrap();
        ddnnf.update_n_vars(2);
        let expected = &[vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]];
        let mut model_enum = ModelEnumerator::new(&ddnnf, false);
        assert_models_eq_with_enumerator(&mut model_enum, expected, false);
        model_enum.reset();
        assert_models_eq_with_enumerator(&mut model_enum, expected, false);
    }

    #[test]
    fn test_reset_or_free_vars() {
        let mut ddnnf = D4Reader::default()
            .read("o 1 0\nt 2 0\n1 2 -1 0\n1 2 1 2 0\n".as_bytes())
            .unwrap();
        ddnnf.update_n_vars(3);
        let expected = &[
            vec![-1, -2, -3],
            vec![-1, -2, 3],
            vec![-1, 2, -3],
            vec![-1, 2, 3],
            vec![1, 2, -3],
            vec![1, 2, 3],
        ];
        let mut model_enum = ModelEnumerator::new(&ddnnf, false);
        assert_models_eq_with_enumerator(&mut model_enum, expected, false);
        model_enum.reset();
        assert_models_eq_with_enumerator(&mut model_enum, expected, false);
    }

    #[test]
    fn test_model_enumerator_assumptions_root_free_vars() {
        let mut ddnnf = D4Reader::default()
            .read("o 1 0\nt 2 0\n1 2 -1 0\n1 2 1 2 0\n".as_bytes())
            .unwrap();
        ddnnf.update_n_vars(3);
        let mut model_enum = ModelEnumerator::new(&ddnnf, false);
        let assumptions = Rc::new(Assumptions::new(3, vec![Literal::from(3)]));
        model_enum.set_assumptions(assumptions);
        let expected = &[vec![-1, -2, 3], vec![-1, 2, 3], vec![1, 2, 3]];
        assert_models_eq_with_enumerator(&mut model_enum, expected, false);
    }

    #[test]
    fn test_model_enumerator_assumptions_or_free_vars() {
        let ddnnf = D4Reader::default()
            .read("o 1 0\nt 2 0\n1 2 -1 0\n1 2 1 2 0\n".as_bytes())
            .unwrap();
        let mut model_enum = ModelEnumerator::new(&ddnnf, false);
        let assumptions = Rc::new(Assumptions::new(2, vec![Literal::from(2)]));
        model_enum.set_assumptions(assumptions);
        let expected = &[vec![-1, 2], vec![1, 2]];
        assert_models_eq_with_enumerator(&mut model_enum, expected, false);
    }

    #[test]
    fn test_model_enumerator_assumptions_becomes_unsat() {
        let ddnnf = D4Reader::default()
            .read("o 1 0\nt 2 0\n1 2 -1 0\n".as_bytes())
            .unwrap();
        let mut model_enum = ModelEnumerator::new(&ddnnf, false);
        let assumptions = Rc::new(Assumptions::new(12, vec![Literal::from(1)]));
        model_enum.set_assumptions(assumptions);
        let expected = &[];
        assert_models_eq_with_enumerator(&mut model_enum, expected, false);
    }

    #[test]
    fn test_no_such_model_index() {
        let mut ddnnf = D4Reader::default().read("t 1 0".as_bytes()).unwrap();
        ddnnf.update_n_vars(1);
        let counter = ModelCounter::new(&ddnnf, false);
        let engine = DirectAccessEngine::new(&counter);
        let mut enumerator = ModelEnumerator::new(&ddnnf, false);
        assert!(enumerator.jump_to(&engine, Integer::from(3)).is_none());
    }

    #[test]
    fn test_ask_for_one_but_unsat() {
        let ddnnf = D4Reader::default().read("f 1 0".as_bytes()).unwrap();
        let mut enumerator = ModelEnumerator::new(&ddnnf, false);
        assert!(enumerator.compute_next_model().is_none());
    }
}
