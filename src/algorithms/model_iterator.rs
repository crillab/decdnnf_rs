use crate::{
    core::{BottomUpVisitor, InvolvedVars},
    DecisionDNNF, Literal, NodeIndex,
};

/// A bottom-up visitor used for the model enumeration algorithm.
#[derive(Default)]
pub struct ModelIteratorVisitor;

/// The data used by the [`ModelIteratorVisitor`] structure.
#[derive(Debug)]
pub struct ModelIteratorVisitorData {
    involved_vars: InvolvedVars,
    children: ModelIteratorVisitorDataChildren,
}

#[derive(Debug)]
enum ModelIteratorVisitorDataChildren {
    SingleModel(Vec<Literal>),
    None,
    Sequence(Vec<(Vec<Literal>, ModelIteratorVisitorData)>),
    Product(Vec<(Vec<Literal>, ModelIteratorVisitorData)>),
}

impl<'a> ModelIteratorVisitorData {
    /// Returns an iterator to the models of the formula.
    #[must_use]
    pub fn iterator(&'a self) -> Box<dyn Iterator<Item = Vec<Literal>> + 'a> {
        match &self.children {
            ModelIteratorVisitorDataChildren::SingleModel(v) => {
                Box::new(std::iter::once(v.clone()))
            }
            ModelIteratorVisitorDataChildren::None => Box::new(std::iter::empty::<Vec<Literal>>()),
            ModelIteratorVisitorDataChildren::Sequence(children) => {
                Box::new(ModelSequenceIterator::new(children))
            }
            ModelIteratorVisitorDataChildren::Product(children) => {
                Box::new(ModelProductIterator::new(children))
            }
        }
    }
}

struct ModelSequenceIterator<'a> {
    children: &'a [(Vec<Literal>, ModelIteratorVisitorData)],
    iterators: Vec<Box<dyn Iterator<Item = Vec<Literal>> + 'a>>,
    next_value: Option<Vec<Literal>>,
    index: usize,
}

impl<'a> ModelSequenceIterator<'a> {
    fn new(children: &'a [(Vec<Literal>, ModelIteratorVisitorData)]) -> Self {
        debug_assert!(!children.is_empty());
        let mut result = Self {
            children,
            iterators: children.iter().map(|c| c.1.iterator()).collect(),
            next_value: Some(vec![]),
            index: 0,
        };
        result.compute_next_value();
        result
    }

    fn compute_next_value(&mut self) {
        let mut nv = None;
        while nv.is_none() && self.index < self.iterators.len() {
            if let Some(v) = self.iterators[self.index].next() {
                nv = Some(v.clone());
            } else {
                self.index += 1;
            }
        }
        if let Some(ref mut v) = nv {
            let next_value_content = self.next_value.as_mut().unwrap();
            next_value_content.clear();
            next_value_content.append(v);
            next_value_content.append(&mut self.children[self.index].0.clone());
        } else {
            self.next_value = None;
        }
    }
}

impl Iterator for ModelSequenceIterator<'_> {
    type Item = Vec<Literal>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut nv) = self.next_value {
            let result = nv.clone();
            self.compute_next_value();
            Some(result)
        } else {
            None
        }
    }
}

struct ModelProductIterator<'a> {
    children: &'a [(Vec<Literal>, ModelIteratorVisitorData)],
    iterators: Vec<Box<dyn Iterator<Item = Vec<Literal>> + 'a>>,
    next_values: Option<Vec<Vec<Literal>>>,
    common: Vec<Literal>,
}

impl<'a> ModelProductIterator<'a> {
    fn new(children: &'a [(Vec<Literal>, ModelIteratorVisitorData)]) -> Self {
        debug_assert!(!children.is_empty());
        let mut iterators: Vec<Box<dyn Iterator<Item = Vec<Literal>> + 'a>> =
            children.iter().map(|c| c.1.iterator()).collect();
        let next_values = iterators
            .iter_mut()
            .map(Iterator::next)
            .collect::<Option<Vec<_>>>();
        let common = children.iter().flat_map(|c| c.0.iter().copied()).collect();
        Self {
            children,
            iterators,
            next_values,
            common,
        }
    }
}

impl Iterator for ModelProductIterator<'_> {
    type Item = Vec<Literal>;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.next_values {
            Some(nv) => {
                let next_value = nv
                    .iter()
                    .flat_map(|v| v.iter().copied())
                    .chain(self.common.iter().copied())
                    .collect::<Vec<_>>();
                let mut last_none = None;
                for i in (0..nv.len()).rev() {
                    let next = self.iterators[i].next();
                    match next {
                        Some(v) => {
                            nv[i] = v;
                            break;
                        }
                        None => last_none = Some(i),
                    }
                }
                match last_none {
                    Some(0) => self.next_values = None,
                    Some(n) => {
                        nv.iter_mut().enumerate().skip(n).for_each(|(i, v)| {
                            self.iterators[i] = self.children[i].1.iterator();
                            *v = self.iterators[i].next().unwrap();
                        });
                    }
                    None => {}
                }
                Some(next_value)
            }
            None => None,
        }
    }
}

impl BottomUpVisitor<ModelIteratorVisitorData> for ModelIteratorVisitor {
    fn merge_for_and(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], ModelIteratorVisitorData)>,
    ) -> ModelIteratorVisitorData {
        let involved_vars = merge_involved_vars(&children, ddnnf.n_vars());
        let new_children = children
            .into_iter()
            .map(|c| (c.0.to_vec(), c.1))
            .collect::<Vec<_>>();
        let data_children = if new_children
            .iter()
            .any(|c| matches!(c.1.children, ModelIteratorVisitorDataChildren::None))
        {
            ModelIteratorVisitorDataChildren::None
        } else if new_children.is_empty() {
            ModelIteratorVisitorDataChildren::SingleModel(vec![])
        } else {
            ModelIteratorVisitorDataChildren::Product(new_children)
        };
        adapt_for_root(
            ModelIteratorVisitorData {
                involved_vars,
                children: data_children,
            },
            path,
        )
    }

    fn merge_for_or(
        &self,
        ddnnf: &DecisionDNNF,
        path: &[NodeIndex],
        children: Vec<(&[Literal], ModelIteratorVisitorData)>,
    ) -> ModelIteratorVisitorData {
        let involved_vars = merge_involved_vars(&children, ddnnf.n_vars());
        let new_children = children
            .into_iter()
            .map(|c| (c.0.to_vec(), c.1))
            .filter(|c| !matches!(c.1.children, ModelIteratorVisitorDataChildren::None))
            .collect::<Vec<_>>();
        let data_children = if new_children.is_empty() {
            ModelIteratorVisitorDataChildren::None
        } else {
            ModelIteratorVisitorDataChildren::Sequence(new_children)
        };
        adapt_for_root(
            ModelIteratorVisitorData {
                involved_vars,
                children: data_children,
            },
            path,
        )
    }

    fn new_for_true(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> ModelIteratorVisitorData {
        adapt_for_root(
            ModelIteratorVisitorData {
                involved_vars: InvolvedVars::new(ddnnf.n_vars()),
                children: ModelIteratorVisitorDataChildren::SingleModel(vec![]),
            },
            path,
        )
    }

    fn new_for_false(&self, ddnnf: &DecisionDNNF, path: &[NodeIndex]) -> ModelIteratorVisitorData {
        adapt_for_root(
            ModelIteratorVisitorData {
                involved_vars: InvolvedVars::new(ddnnf.n_vars()),
                children: ModelIteratorVisitorDataChildren::None,
            },
            path,
        )
    }
}

fn merge_involved_vars(
    children: &[(&[Literal], ModelIteratorVisitorData)],
    n_vars: usize,
) -> InvolvedVars {
    let mut involved_vars = InvolvedVars::new(n_vars);
    for c in children {
        involved_vars.or_assign(&c.1.involved_vars);
        involved_vars.set_literals(c.0);
    }
    involved_vars
}

fn adapt_for_root(data: ModelIteratorVisitorData, path: &[NodeIndex]) -> ModelIteratorVisitorData {
    if path.len() == 1 && data.involved_vars.count_zeros() > 0 {
        let n_vars = data.involved_vars.len();
        let mut new_children = vec![(vec![], data)];
        let true_child = || ModelIteratorVisitorData {
            involved_vars: InvolvedVars::new(n_vars),
            children: ModelIteratorVisitorDataChildren::SingleModel(vec![]),
        };
        let involved_so_far = new_children[0].1.involved_vars.clone();
        for l in involved_so_far.iter_missing_literals() {
            let mut child_involved_vars = InvolvedVars::new(n_vars);
            child_involved_vars.set_literal(l);
            new_children.push((
                vec![],
                ModelIteratorVisitorData {
                    involved_vars: child_involved_vars,
                    children: ModelIteratorVisitorDataChildren::Sequence(vec![
                        (vec![l], true_child()),
                        (vec![l.flip()], true_child()),
                    ]),
                },
            ));
        }
        ModelIteratorVisitorData {
            involved_vars: InvolvedVars::new_all_set(n_vars),
            children: ModelIteratorVisitorDataChildren::Product(new_children),
        }
    } else {
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::BottomUpTraversal, D4Reader};

    fn assert_models_eq(str_ddnnf: &str, mut expected: Vec<Vec<isize>>, n_vars: Option<usize>) {
        let sort = |v: &mut Vec<Vec<isize>>| {
            v.iter_mut().for_each(|m| m.sort_unstable());
            v.sort_unstable();
        };
        sort(&mut expected);
        let mut ddnnf = D4Reader::read(str_ddnnf.as_bytes()).unwrap();
        if let Some(n) = n_vars {
            ddnnf.update_n_vars(n);
        }
        let traversal = BottomUpTraversal::new(Box::<ModelIteratorVisitor>::default());
        let model_it = traversal.traverse(&ddnnf);
        let mut actual = model_it
            .iterator()
            .map(|v| v.iter().map(|l| isize::from(*l)).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        sort(&mut actual);
        assert_eq!(expected, actual,);
    }

    #[test]
    fn test_unsat() {
        assert_models_eq("f 1 0\n", vec![], None);
    }

    #[test]
    fn test_tautology() {
        assert_models_eq("t 1 0\n", vec![vec![-1], vec![1]], Some(1));
    }

    #[test]
    fn test_or() {
        assert_models_eq(
            "o 1 0\nt 2 0\n1 2 -1 0\n 1 2 1 0\n",
            vec![vec![-1], vec![1]],
            None,
        );
    }

    #[test]
    fn test_and() {
        assert_models_eq(
            "a 1 0\nt 2 0\n1 2 -1 0\n 1 2 -2 0\n",
            vec![vec![-1, -2]],
            None,
        );
    }

    #[test]
    fn test_and_or() {
        assert_models_eq(
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n",
            vec![vec![-1, -2], vec![-1, 2], vec![1, -2], vec![1, 2]],
            None,
        );
    }

    #[test]
    fn test_or_and() {
        assert_models_eq(
            "o 1 0\na 2 0\na 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 -2 0\n3 4 1 0\n3 4 2 0\n",
            vec![vec![-1, -2], vec![1, 2]],
            None,
        );
    }
}
