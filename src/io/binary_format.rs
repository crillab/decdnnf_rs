use crate::{
    DecisionDNNF, DecisionDNNFReader, DecisionDNNFWriter, Edge, EdgeIndex, Literal, Node, NodeIndex,
};
use anyhow::{anyhow, Context, Result};
use std::io::{BufReader, BufWriter, Read, Write};

const AND_BYTE: u8 = 0x00;
const OR_BYTE: u8 = 0x01;
const TRUE_BYTE: u8 = 0x02;
const FALSE_BYTE: u8 = 0x03;

/// A structure used to write a Decision-DNNF using a binary format.
#[derive(Default)]
pub struct Writer;

impl<W> DecisionDNNFWriter<W> for Writer
where
    W: Write,
{
    fn write(&self, writer: W, ddnnf: &DecisionDNNF) -> Result<()> {
        let mut writer = BufWriter::new(writer);
        write_usize(&mut writer, ddnnf.n_vars())?;
        write_slice(&mut writer, ddnnf.nodes().as_slice(), |w, n| match n {
            Node::And(items) => {
                w.write_all(&[AND_BYTE])?;
                write_edges(w, items)
            }
            Node::Or(items) => {
                w.write_all(&[OR_BYTE])?;
                write_edges(w, items)
            }
            Node::True => w
                .write_all(&[TRUE_BYTE])
                .context("while writing a True node"),
            Node::False => w
                .write_all(&[FALSE_BYTE])
                .context("while writing a False node"),
        })?;
        write_slice(&mut writer, ddnnf.edges().as_slice(), |w, e| {
            write_usize(w, usize::from(e.target()))?;
            write_slice(w, e.propagated(), |w, p| write_usize(w, p.into_usize()))
        })?;
        Ok(())
    }
}

/// A structure used to read a Decision-DNNF encoded using its writer counterpart.
#[derive(Default)]
pub struct Reader;

impl<R> DecisionDNNFReader<R> for Reader
where
    R: Read,
{
    fn set_do_not_check(&mut self, _do_not_check: bool) {}

    fn read(&self, reader: R) -> Result<DecisionDNNF> {
        let mut reader = BufReader::new(reader);
        let n_vars = read_usize(&mut reader)?;
        let nodes = read_vec(&mut reader, |r| {
            let mut node_bytes = vec![0_u8];
            r.read_exact(&mut node_bytes)?;
            match node_bytes[0] {
                AND_BYTE => {
                    let edges = read_edges(r)?;
                    Ok(Node::And(edges))
                }
                OR_BYTE => {
                    let edges = read_edges(r)?;
                    Ok(Node::Or(edges))
                }
                TRUE_BYTE => Ok(Node::True),
                FALSE_BYTE => Ok(Node::False),
                _ => Err(anyhow!("unknown node code")),
            }
        })?;
        let edges = read_vec(&mut reader, |r| {
            let target = NodeIndex::from(read_usize(r)?);
            let propagated = read_vec(r, |r| Ok(Literal::from_usize(read_usize(r)?)))?;
            Ok(Edge::from_raw_data(target, propagated))
        })?;
        Ok(DecisionDNNF::from_raw_data(n_vars, nodes, edges))
    }
}

fn write_slice<W, T, F>(writer: &mut BufWriter<W>, slice: &[T], item_writer: F) -> Result<()>
where
    W: Write,
    F: Fn(&mut BufWriter<W>, &T) -> Result<()>,
{
    write_usize(writer, slice.len())?;
    for item in slice {
        item_writer(writer, item)?;
    }
    Ok(())
}

fn write_edges<W>(writer: &mut BufWriter<W>, edges: &[EdgeIndex]) -> Result<()>
where
    W: Write,
{
    write_slice(writer, edges, |w, e| write_usize(w, usize::from(*e)))
}

fn write_usize<W>(writer: &mut BufWriter<W>, n: usize) -> Result<()>
where
    W: Write,
{
    writer
        .write_all(&(n as u64).to_be_bytes())
        .context("while writing a number")
}

fn read_vec<R, T, F>(reader: &mut BufReader<R>, item_reader: F) -> Result<Vec<T>>
where
    R: Read,
    F: Fn(&mut BufReader<R>) -> Result<T>,
{
    let len = read_usize(reader)?;
    let mut v = Vec::with_capacity(len);
    for _ in 0..len {
        v.push(item_reader(reader)?);
    }
    Ok(v)
}

fn read_edges<R>(reader: &mut BufReader<R>) -> Result<Vec<EdgeIndex>>
where
    R: Read,
{
    read_vec(reader, |r| Ok(EdgeIndex::from(read_usize(r)?)))
}

fn read_usize<R>(reader: &mut BufReader<R>) -> Result<usize>
where
    R: Read,
{
    let mut buffer = [0_u8; size_of::<u64>()];
    reader
        .read_exact(&mut buffer)
        .context("while reading a number")?;
    #[allow(clippy::cast_possible_truncation)]
    Ok(u64::from_be_bytes(buffer) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::D4Reader;

    fn assert_for_instance(instance_str: &str) {
        let init_formula = D4Reader::default().read(instance_str.as_bytes()).unwrap();
        let mut buf = BufWriter::new(Vec::new());
        Writer.write(&mut buf, &init_formula).unwrap();
        let bytes = buf.into_inner().unwrap();
        let new_formula = Reader.read(bytes.as_slice()).unwrap();
        assert_eq!(init_formula.n_vars(), new_formula.n_vars());
        assert_eq!(init_formula.nodes(), new_formula.nodes());
        assert_eq!(init_formula.edges(), new_formula.edges());
    }

    #[test]
    fn test_trivial() {
        assert_for_instance("t 1 0");
        assert_for_instance("f 1 0");
    }

    #[test]
    fn test_and() {
        assert_for_instance("a 1 0\nt 2 0\n1 2 1 2 0");
    }

    #[test]
    fn test_or() {
        assert_for_instance("o 1 0\nt 2 0\n1 2 -1 0\n1 2 1 2 0\n");
    }
}
