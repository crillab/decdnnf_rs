use crate::{core::Literal, DecisionDNNF, Node};
use anyhow::{Context, Result};
use std::io::BufWriter;
pub use std::io::Write;

/// A structure used to write a Decision-DNNF using the [c2d](http://reasoning.cs.ucla.edu/c2d/) output format.
pub struct Writer;

impl Writer {
    /// Writes a Decision-DNNF using the c2d format.
    ///
    /// # Errors
    ///
    /// An error is raised if an I/O exception occurs.
    pub fn write<W>(writer: W, dnnf: &DecisionDNNF) -> Result<()>
    where
        W: Write,
    {
        // TODO: nnf n_nodes n_models n_vars
        let mut bufwriter = BufWriter::new(writer);
        let mut writer_data = C2DFormatWriterData::new(bufwriter, dnnf);
        Self::write_from(&mut writer_data, 0, &[])?;
        Ok(())
    }

    fn write_from<W>(
        writer_data: &mut C2DFormatWriterData<W>,
        node_index: usize,
        propagations: &[Literal],
    ) -> Result<usize>
    where
        W: Write,
    {
        let n = match &writer_data.dnnf.nodes()[node_index] {
            Node::And(children_nodes) => {
                let mut children_new_indices = children_nodes
                    .iter()
                    .map(|edge_index| {
                        let edge = &writer_data.dnnf.edges()[*edge_index];
                        Self::write_from(writer_data, edge.target(), edge.propagated())
                    })
                    .collect::<Result<Vec<usize>>>()?;
                let mut propagation_new_indices = propagations
                    .iter()
                    .map(|l| writer_data.write_literal(*l))
                    .collect::<Result<Vec<usize>>>()?;
                children_new_indices.append(&mut propagation_new_indices);
                writer_data.write_and(&children_new_indices)
            }
            Node::Or(children_nodes) => {
                let children_new_indices = children_nodes
                    .iter()
                    .map(|edge_index| {
                        let edge = &writer_data.dnnf.edges()[*edge_index];
                        Self::write_from(writer_data, edge.target(), edge.propagated())
                    })
                    .collect::<Result<Vec<usize>>>()?;
                let or_node_index = writer_data.write_or(&children_new_indices, 0)?;
                let mut propagation_new_indices = propagations
                    .iter()
                    .map(|l| writer_data.write_literal(*l))
                    .collect::<Result<Vec<usize>>>()?;
                propagation_new_indices.push(or_node_index);
                writer_data.write_and(&propagation_new_indices)
            }
            Node::True => {
                if propagations.is_empty() {
                    writer_data.write_true()
                } else if propagations.len() == 1 {
                    writer_data.write_literal(propagations[0])
                } else {
                    let children_indices = propagations
                        .iter()
                        .map(|l| writer_data.write_literal(*l))
                        .collect::<Result<Vec<usize>>>()?;
                    writer_data.write_and(&children_indices)
                }
            }
            Node::False => writer_data.write_false(),
        }?;
        Ok(n)
    }
}

struct C2DFormatWriterData<'a, W>
where
    W: Write,
{
    writer: BufWriter<W>,
    dnnf: &'a DecisionDNNF,
    next_index: usize,
    true_index: Option<usize>,
    false_index: Option<usize>,
    positive_literal_indices: Vec<Option<usize>>,
    negative_literal_indices: Vec<Option<usize>>,
}

impl<'a, W> C2DFormatWriterData<'a, W>
where
    W: Write,
{
    fn new(writer: BufWriter<W>, dnnf: &'a DecisionDNNF) -> Self {
        Self {
            writer,
            dnnf,
            next_index: 0,
            true_index: None,
            false_index: None,
            positive_literal_indices: vec![None; dnnf.n_vars()],
            negative_literal_indices: vec![None; dnnf.n_vars()],
        }
    }

    fn write_true(&mut self) -> Result<usize> {
        write_opt_bool(&mut self.true_index, &mut self.next_index, &mut || {
            writeln!(self.writer, "a 0")
        })
        .context("while writing a true leaf")
    }

    fn write_false(&mut self) -> Result<usize> {
        write_opt_bool(&mut self.false_index, &mut self.next_index, &mut || {
            writeln!(self.writer, "O 0 0")
        })
        .context("while writing a false leaf")
    }

    fn write_literal(&mut self, l: Literal) -> Result<usize> {
        if l.polarity() {
            write_opt_bool(
                &mut self.positive_literal_indices[l.var_index()],
                &mut self.next_index,
                &mut || writeln!(self.writer, "L {l}"),
            )
            .context("while writing a literal")
        } else {
            write_opt_bool(
                &mut self.negative_literal_indices[l.var_index()],
                &mut self.next_index,
                &mut || writeln!(self.writer, "L {l}"),
            )
            .context("while writing a literal")
        }
    }

    fn write_and(&mut self, node_indices: &[usize]) -> Result<usize> {
        self.next_index += 1;
        write!(self.writer, "A {}", node_indices.len())?;
        for i in node_indices {
            write!(self.writer, " {i}")?;
        }
        writeln!(self.writer)?;
        Ok(self.next_index - 1)
    }

    fn write_or(&mut self, node_indices: &[usize], conflicting_var: usize) -> Result<usize> {
        todo!()
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
