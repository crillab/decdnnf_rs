use crate::{
    core::{EdgeIndex, Literal, Node, NodeIndex},
    DecisionDNNF,
};
use anyhow::{anyhow, Context, Result};
use rustc_hash::FxHashMap;
use std::io::BufWriter;
pub use std::io::Write;

/// A structure used to write a Decision-DNNF in the [c2d](http://reasoning.cs.ucla.edu/c2d/) output format.
pub struct Writer;

impl Writer {
    /// Writes a Decision-DNNF in the c2d format.
    ///
    /// # Errors
    ///
    /// An error is raised when an I/O exception occurs.
    pub fn write<W>(mut writer: W, ddnnf: &DecisionDNNF) -> Result<()>
    where
        W: Write,
    {
        let mut buf = Vec::new();
        let bufwriter = BufWriter::new(&mut buf);
        let mut writer_data = C2DFormatWriterData::new(bufwriter, ddnnf);
        Self::write_from(&mut writer_data, 0.into(), &[])?;
        writeln!(
            writer,
            "nnf {} {} {}",
            writer_data.n_nodes,
            writer_data.n_edges,
            writer_data.ddnnf.n_vars()
        )?;
        std::mem::drop(writer_data);
        write!(writer, "{}", std::str::from_utf8(&buf)?)
            .context("while writing the buffered content")
    }

    fn write_from<W>(
        writer_data: &mut C2DFormatWriterData<W>,
        node_index: NodeIndex,
        propagations: &[Literal],
    ) -> Result<usize>
    where
        W: Write,
    {
        let write_single_child = |writer_data: &mut C2DFormatWriterData<W>,
                                  edge_index: EdgeIndex| {
            let edge = &writer_data.ddnnf.edges()[edge_index];
            let merged_propagations = propagations
                .iter()
                .chain(edge.propagated())
                .copied()
                .collect::<Vec<_>>();
            Self::write_from(writer_data, edge.target(), &merged_propagations)
        };
        match &writer_data.ddnnf.nodes()[node_index] {
            Node::And(v) | Node::Or(v) => {
                if let &[e] = &v[..] {
                    return write_single_child(writer_data, e);
                }
            }
            _ => {}
        };
        let write_propagations = |w_data: &mut C2DFormatWriterData<W>, propagations: &[Literal]| {
            propagations
                .iter()
                .map(|l| w_data.write_literal(*l))
                .collect::<Result<Vec<usize>>>()
        };
        let n = match &writer_data.ddnnf.nodes()[node_index] {
            Node::And(children_nodes) => {
                let mut children_new_indices = children_nodes
                    .iter()
                    .map(|edge_index| {
                        let edge = &writer_data.ddnnf.edges()[*edge_index];
                        Self::write_from(writer_data, edge.target(), edge.propagated())
                    })
                    .collect::<Result<Vec<usize>>>()?;
                let mut propagation_new_indices = write_propagations(writer_data, propagations)?;
                children_new_indices.append(&mut propagation_new_indices);
                writer_data.write_and(children_new_indices)
            }
            Node::Or(children_nodes) => {
                debug_assert_eq!(2, children_nodes.len());
                if let Some(pos) = children_nodes.iter().position(|edge_index| {
                    let edge = &writer_data.ddnnf.edges()[*edge_index];
                    let child = &writer_data.ddnnf.nodes()[edge.target()];
                    matches!(child, Node::False)
                }) {
                    return write_single_child(writer_data, children_nodes[1 - pos]);
                }
                Self::write_or(writer_data, children_nodes, propagations)
            }
            Node::True => {
                if propagations.is_empty() {
                    writer_data.write_true()
                } else if propagations.len() == 1 {
                    writer_data.write_literal(propagations[0])
                } else {
                    let children_indices = write_propagations(writer_data, propagations)?;
                    writer_data.write_and(children_indices)
                }
            }
            Node::False => writer_data.write_false(),
        }?;
        Ok(n)
    }

    fn write_or<W>(
        writer_data: &mut C2DFormatWriterData<W>,
        children_nodes: &[EdgeIndex],
        propagations: &[Literal],
    ) -> Result<usize>
    where
        W: Write,
    {
        let (conflicting_var_index, pos_occurrences, neg_occurrences) =
            Self::split_on_conflicting_variable(writer_data, children_nodes)?;
        let mut write_child = |occ: &[EdgeIndex]| match occ {
            &[e] => {
                let edge = &writer_data.ddnnf.edges()[e];
                Self::write_from(writer_data, edge.target(), edge.propagated())
            }
            _ => Self::write_or(writer_data, occ, &[]),
        };
        let pos_child = write_child(&pos_occurrences)?;
        let neg_child = write_child(&neg_occurrences)?;
        let mut result = writer_data.write_or(conflicting_var_index, neg_child, pos_child)?;
        if !propagations.is_empty() {
            let and_children = propagations
                .iter()
                .map(|p| writer_data.write_literal(*p))
                .chain(std::iter::once(Ok(result)))
                .collect::<Result<Vec<_>>>()?;
            result = writer_data.write_and(and_children)?;
        }
        Ok(result)
    }

    fn split_on_conflicting_variable<W>(
        writer_data: &mut C2DFormatWriterData<W>,
        children_nodes: &[EdgeIndex],
    ) -> Result<(usize, Vec<EdgeIndex>, Vec<EdgeIndex>)>
    where
        W: Write,
    {
        let edges = writer_data.ddnnf.edges();
        let first_index = children_nodes[0];
        for l in edges[first_index].propagated() {
            let (mut pos_occurrences, mut neg_occurrences) = if l.polarity() {
                (vec![first_index], vec![])
            } else {
                (vec![], vec![first_index])
            };
            let mut seen_in_all = true;
            for edge_index in children_nodes.iter().skip(1) {
                let edge = &edges[*edge_index];
                seen_in_all = false;
                for other_l in edge.propagated() {
                    if l.var_index() == other_l.var_index() {
                        seen_in_all = true;
                        if other_l.polarity() {
                            pos_occurrences.push(*edge_index);
                        } else {
                            neg_occurrences.push(*edge_index);
                        }
                    }
                }
                if !seen_in_all {
                    break;
                }
            }
            if seen_in_all && !pos_occurrences.is_empty() && !neg_occurrences.is_empty() {
                return Ok((l.var_index(), pos_occurrences, neg_occurrences));
            }
        }
        Err(anyhow!("cannot convert OR node as a decision node"))
    }
}

struct C2DFormatWriterData<'a, W>
where
    W: Write,
{
    writer: BufWriter<W>,
    ddnnf: &'a DecisionDNNF,
    n_nodes: usize,
    n_edges: usize,
    true_index: Option<usize>,
    false_index: Option<usize>,
    positive_literal_indices: Vec<Option<usize>>,
    negative_literal_indices: Vec<Option<usize>>,
    and_cache: FxHashMap<Vec<usize>, usize>,
    or_cache: FxHashMap<(usize, usize), usize>,
}

impl<'a, W> C2DFormatWriterData<'a, W>
where
    W: Write,
{
    fn new(writer: BufWriter<W>, dnnf: &'a DecisionDNNF) -> Self {
        Self {
            writer,
            ddnnf: dnnf,
            n_nodes: 0,
            n_edges: 0,
            true_index: None,
            false_index: None,
            positive_literal_indices: vec![None; dnnf.n_vars()],
            negative_literal_indices: vec![None; dnnf.n_vars()],
            and_cache: FxHashMap::default(),
            or_cache: FxHashMap::default(),
        }
    }

    fn write_true(&mut self) -> Result<usize> {
        write_opt_bool(&mut self.true_index, &mut self.n_nodes, &mut || {
            writeln!(self.writer, "A 0")
        })
        .context("while writing a true leaf")
    }

    fn write_false(&mut self) -> Result<usize> {
        write_opt_bool(&mut self.false_index, &mut self.n_nodes, &mut || {
            writeln!(self.writer, "O 0 0")
        })
        .context("while writing a false leaf")
    }

    fn write_literal(&mut self, l: Literal) -> Result<usize> {
        if l.polarity() {
            write_opt_bool(
                &mut self.positive_literal_indices[l.var_index()],
                &mut self.n_nodes,
                &mut || writeln!(self.writer, "L {l}"),
            )
            .context("while writing a literal")
        } else {
            write_opt_bool(
                &mut self.negative_literal_indices[l.var_index()],
                &mut self.n_nodes,
                &mut || writeln!(self.writer, "L {l}"),
            )
            .context("while writing a literal")
        }
    }

    fn write_and(&mut self, mut node_indices: Vec<usize>) -> Result<usize> {
        node_indices.sort_unstable();
        if let Some(n) = self.and_cache.get(&node_indices) {
            return Ok(*n);
        }
        self.n_nodes += 1;
        self.n_edges += node_indices.len();
        write!(self.writer, "A {}", node_indices.len())?;
        node_indices
            .iter()
            .try_for_each(|i| write!(self.writer, " {i}"))?;
        writeln!(self.writer)?;
        self.and_cache.insert(node_indices, self.n_nodes - 1);
        Ok(self.n_nodes - 1)
    }

    fn write_or(
        &mut self,
        conflicting_var_index: usize,
        child_index0: usize,
        child_index1: usize,
    ) -> Result<usize> {
        let child_indices = if child_index0 < child_index1 {
            (child_index0, child_index1)
        } else {
            (child_index1, child_index0)
        };
        if let Some(n) = self.or_cache.get(&child_indices) {
            return Ok(*n);
        }
        self.n_nodes += 1;
        self.n_edges += 2;
        writeln!(
            self.writer,
            "O {} 2 {} {}",
            1 + conflicting_var_index,
            child_indices.0,
            child_indices.1,
        )
        .context("while writing a OR node")?;
        self.or_cache.insert(child_indices, self.n_nodes - 1);
        Ok(self.n_nodes - 1)
    }
}

fn write_opt_bool(
    opt: &mut Option<usize>,
    next_index: &mut usize,
    write_fn: &mut dyn FnMut() -> std::io::Result<()>,
) -> std::io::Result<usize> {
    if let Some(n) = opt {
        return Ok(*n);
    }
    *opt = Some(*next_index);
    *next_index += 1;
    write_fn()?;
    Ok(*next_index - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn assert_translation(init: &str, expected: &str) {
        let ddnnf = D4Reader::default().read(&mut init.as_bytes()).unwrap();
        let mut buffer = Vec::new();
        Writer::write(&mut buffer, &ddnnf).unwrap();
        let actual = String::from_utf8(buffer).unwrap();
        assert_eq!(expected, actual);
    }

    #[test]
    fn test_true() {
        assert_translation("t 1 0\n", "nnf 1 0 0\nA 0\n");
    }

    #[test]
    fn test_false() {
        assert_translation("f 1 0\n", "nnf 1 0 0\nO 0 0\n");
    }

    #[test]
    fn test_lit_with_or() {
        assert_translation("o 1 0\nt 2 0\n1 2 1 0\n", "nnf 1 0 1\nL 1\n");
    }

    #[test]
    fn test_lit_with_and() {
        assert_translation("a 1 0\nt 2 0\n1 2 -1 0\n", "nnf 1 0 1\nL -1\n");
    }

    #[test]
    fn test_and() {
        assert_translation(
            "a 1 0\nt 2 0\n1 2 1 0\n1 2 2 0\n",
            "nnf 3 2 2\nL 1\nL 2\nA 2 0 1\n",
        );
    }

    #[test]
    fn test_or() {
        assert_translation(
            "o 1 0\nt 2 0\n1 2 1 0\n1 2 -1 0\n",
            "nnf 3 2 1\nL 1\nL -1\nO 1 2 0 1\n",
        );
    }

    #[test]
    fn test_caching() {
        assert_translation(
            "o 1 0\no 2 0\nt 3 0\n1 2 -1 2 0\n1 2 1 -3 0\n2 3 -4 5 0\n2 3 4 -5 0",
            "nnf 14 14 5\nL 4\nL -5\nA 2 0 1\nL -4\nL 5\nA 2 3 4\nO 4 2 2 5\nL 1\nL -3\nA 3 6 7 8\nL -1\nL 2\nA 3 6 10 11\nO 1 2 9 12\n",
        );
    }

    #[test]
    fn test_determinism_with_false() {
        assert_translation(
            "o 1 0\nt 2 0\nf 3 0\n1 2 -1 0\n1 3 0\n",
            "nnf 1 0 1\nL -1\n",
        );
        assert_translation(
            "o 1 0\nf 2 0\nt 3 0\n1 2 0\n1 3 -1 0\n",
            "nnf 1 0 1\nL -1\n",
        );
    }
}
