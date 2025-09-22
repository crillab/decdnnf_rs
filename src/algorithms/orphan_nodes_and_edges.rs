use crate::{
    core::{EdgeIndex, Node, NodeIndex},
    DecisionDNNF,
};
use anyhow::{anyhow, Result};

/// A structure used to detect and remove orphan nodes and edges, i.e. nodes and edges which have no ancestor albeit they are not the root of the [`DecisionDNNF`].
pub struct OrphanFinder {
    orphan_nodes: Vec<NodeIndex>,
    orphan_edges: Vec<EdgeIndex>,
}

impl OrphanFinder {
    /// Search the orphan nodes.
    ///
    /// # Errors
    ///
    /// This function also detects cycles in the graph.
    /// In case one is found, an error is returned.
    pub fn search(ddnnf: &DecisionDNNF) -> Result<Self> {
        let mut node_seen_once = vec![false; ddnnf.n_nodes()];
        let mut node_seen_on_path = vec![false; ddnnf.n_nodes()];
        let mut edge_seen_once = vec![false; ddnnf.n_edges()];
        Self::search_from(
            ddnnf,
            &mut node_seen_once,
            &mut node_seen_on_path,
            &mut edge_seen_once,
            0.into(),
        )?;
        let mut orphan_nodes = Self::seen_to_vec(&node_seen_once);
        orphan_nodes.sort_unstable();
        let mut orphan_edges = Self::seen_to_vec(&edge_seen_once);
        orphan_edges.sort_unstable();
        Ok(Self {
            orphan_nodes,
            orphan_edges,
        })
    }

    fn seen_to_vec<T>(seen: &[bool]) -> Vec<T>
    where
        T: From<usize>,
    {
        seen.iter()
            .enumerate()
            .filter_map(|(i, b)| if *b { None } else { Some(T::from(i)) })
            .collect::<Vec<_>>()
    }

    fn search_from(
        ddnnf: &DecisionDNNF,
        node_seen_once: &mut [bool],
        node_seen_on_path: &mut [bool],
        edge_seen: &mut [bool],
        node_index: NodeIndex,
    ) -> Result<()> {
        if node_seen_on_path[usize::from(node_index)] {
            return Err(anyhow!("cycle detected"));
        }
        if node_seen_once[usize::from(node_index)] {
            return Ok(());
        }
        node_seen_on_path[usize::from(node_index)] = true;
        node_seen_once[usize::from(node_index)] = true;
        match &ddnnf.nodes()[usize::from(node_index)] {
            Node::And(v) | Node::Or(v) => {
                for edge_index in v {
                    edge_seen[usize::from(*edge_index)] = true;
                }
                v.iter().try_for_each(|e| {
                    Self::search_from(
                        ddnnf,
                        node_seen_once,
                        node_seen_on_path,
                        edge_seen,
                        ddnnf.edges()[usize::from(*e)].target(),
                    )
                })?;
            }
            Node::True | Node::False => {}
        }
        node_seen_on_path[usize::from(node_index)] = false;
        Ok(())
    }

    /// Returns a slice to the indices of the orphan nodes.
    #[must_use]
    pub fn orphans_nodes(&self) -> &[NodeIndex] {
        &self.orphan_nodes
    }

    /// Returns a slice to the indices of the orphan nodes.
    #[must_use]
    pub fn orphans_edges(&self) -> &[EdgeIndex] {
        &self.orphan_edges
    }

    /// Replaces the equivalences in the formula.
    pub fn remove_from_formula(&self, ddnnf: &mut DecisionDNNF) {
        if self.orphan_nodes.is_empty() && self.orphan_edges.is_empty() {
            return;
        }
        let new_node_indices = Self::new_indices_with_offset(ddnnf.n_nodes(), &self.orphan_nodes);
        let new_edge_indices = Self::new_indices_with_offset(ddnnf.n_edges(), &self.orphan_edges);
        let mut new_nodes =
            Self::remove_sorted_indices(ddnnf.nodes().as_slice(), self.orphans_nodes());
        std::mem::swap(ddnnf.nodes_mut().as_mut(), &mut new_nodes);
        let mut new_edges =
            Self::remove_sorted_indices(ddnnf.edges().as_slice(), self.orphans_edges());
        std::mem::swap(ddnnf.edges_mut().as_mut(), &mut new_edges);
        for node in ddnnf.nodes_mut().as_mut_slice() {
            match node {
                Node::And(items) | Node::Or(items) => {
                    for edge_index in items {
                        *edge_index = new_edge_indices[usize::from(*edge_index)];
                    }
                }
                Node::True | Node::False => {}
            }
        }
        for edge in ddnnf.edges_mut().as_mut_slice() {
            edge.set_target(new_node_indices[usize::from(edge.target())]);
        }
    }

    fn remove_sorted_indices<T, U>(items: &[T], to_remove: &[U]) -> Vec<T>
    where
        T: Clone,
        U: Copy,
        usize: From<U>,
    {
        if to_remove.is_empty() {
            return items.to_vec();
        }
        let mut new_vec = Vec::with_capacity(items.len());
        new_vec.append(&mut items[0..usize::from(to_remove[0])].to_vec());
        for (i, n) in to_remove.iter().enumerate() {
            let min_bound = usize::from(*n) + 1;
            if min_bound == items.len() {
                break;
            }
            let max_bound = if i == to_remove.len() - 1 {
                items.len()
            } else {
                usize::from(to_remove[i + 1])
            };
            new_vec.append(&mut items[min_bound..max_bound].to_vec());
        }
        new_vec
    }

    fn new_indices_with_offset<T>(len: usize, orphans: &[T]) -> Vec<T>
    where
        T: Copy + PartialOrd + From<usize> + std::fmt::Debug,
        usize: From<T>,
    {
        let mut new_indices = vec![];
        let mut current_offset = 0;
        for i in 0..len {
            new_indices.push(T::from(i - current_offset));
            if current_offset < orphans.len() && orphans[current_offset] == i.into() {
                current_offset += 1;
            }
        }
        new_indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{core::Edge, Literal};

    #[test]
    fn test_node_unreachable() {
        let ddnnf = DecisionDNNF::from_raw_data(0, vec![Node::False, Node::True], vec![]);
        let finder = OrphanFinder::search(&ddnnf).unwrap();
        assert_eq!(&[NodeIndex::from(1)], finder.orphans_nodes());
    }

    #[test]
    fn test_node_cycle() {
        let ddnnf = DecisionDNNF::from_raw_data(
            0,
            vec![Node::And(vec![0.into()]), Node::And(vec![1.into()])],
            vec![
                Edge::from_raw_data(1.into(), vec![]),
                Edge::from_raw_data(0.into(), vec![]),
            ],
        );
        assert!(OrphanFinder::search(&ddnnf).is_err());
    }

    #[test]
    fn test_remove_node() {
        let mut ddnnf = DecisionDNNF::from_raw_data(
            3,
            vec![
                Node::Or(vec![0.into(), 1.into()]),
                Node::And(vec![2.into()]),
                Node::And(vec![3.into()]),
                Node::True,
            ],
            vec![
                Edge::from_raw_data(1.into(), vec![Literal::from(-1)]),
                Edge::from_raw_data(1.into(), vec![Literal::from(1)]),
                Edge::from_raw_data(3.into(), vec![Literal::from(2), Literal::from(3)]),
                Edge::from_raw_data(3.into(), vec![Literal::from(2), Literal::from(3)]),
            ],
        );
        let finder = OrphanFinder::search(&ddnnf).unwrap();
        assert_eq!(&[NodeIndex::from(2)], finder.orphans_nodes());
        finder.remove_from_formula(&mut ddnnf);
        assert_eq!(
            &[
                Node::Or(vec![0.into(), 1.into()]),
                Node::And(vec![2.into()]),
                Node::True
            ],
            ddnnf.nodes().as_slice()
        );
        assert_eq!(
            &[
                Edge::from_raw_data(1.into(), vec![Literal::from(-1)]),
                Edge::from_raw_data(1.into(), vec![Literal::from(1)]),
                Edge::from_raw_data(2.into(), vec![Literal::from(2), Literal::from(3)]),
            ],
            ddnnf.edges().as_slice(),
        );
    }

    #[test]
    fn test_remove_edge() {
        let mut ddnnf = DecisionDNNF::from_raw_data(
            3,
            vec![
                Node::Or(vec![0.into()]),
                Node::Or(vec![2.into()]),
                Node::True,
            ],
            vec![
                Edge::from_raw_data(1.into(), vec![]),
                Edge::from_raw_data(1.into(), vec![]),
                Edge::from_raw_data(2.into(), vec![Literal::from(1)]),
            ],
        );
        let finder = OrphanFinder::search(&ddnnf).unwrap();
        assert_eq!(&[EdgeIndex::from(1)], finder.orphans_edges());
        finder.remove_from_formula(&mut ddnnf);
        assert_eq!(
            &[
                Node::Or(vec![0.into()]),
                Node::Or(vec![1.into()]),
                Node::True,
            ],
            ddnnf.nodes().as_slice()
        );
        assert_eq!(
            &[
                Edge::from_raw_data(1.into(), vec![]),
                Edge::from_raw_data(2.into(), vec![Literal::from(1)]),
            ],
            ddnnf.edges().as_slice(),
        );
    }
}
