use crate::{DecisionDNNF, Edge, Literal, Node, NodeIndex};
use anyhow::{anyhow, Context, Result};
use std::str::FromStr;
use std::{
    cell::RefCell,
    io::{BufRead, BufReader, Read},
    rc::Rc,
    str::SplitWhitespace,
};

/// A structure used to read the output of the d4 compiler.
///
/// The output format of d4 is an extension of the initial format output by c2d.
/// The description of the format is available on the [d4](https://github.com/crillab/d4) repository.
pub struct Reader;

impl Reader {
    /// Reads an instance and returns it.
    ///
    /// It is assumed that the first node is the root, and that the first node index is 1.
    ///
    /// # Errors
    ///
    /// An error is returned if the content of the instance does not follow the d4 format or one of the assumptions described above is not true.
    pub fn read<R>(reader: R) -> Result<DecisionDNNF>
    where
        R: Read,
    {
        let mut reader = BufReader::new(reader);
        let mut buffer = String::new();
        let context = "while parsing a d4 formatted Decision-DNNF";
        let line_index = Rc::new(RefCell::new(0));
        let line_index_context = || format!("while parsing line at index {}", line_index.borrow());
        let mut reader_data = D4FormatReaderData::default();
        loop {
            let line_len = reader
                .read_line(&mut buffer)
                .with_context(line_index_context)
                .context(context)?;
            if line_len == 0 {
                break;
            }
            let mut words = buffer.split_whitespace();
            if let Some(first_word) = words.next() {
                match first_word {
                    "o" | "a" | "t" | "f" => {
                        Self::add_new_node(&mut reader_data, first_word, words)
                            .with_context(line_index_context)
                            .context("while parsing a node")
                            .context(context)?;
                    }
                    w if usize::from_str(w).is_ok() => {
                        Self::add_new_edge(&mut reader_data, first_word, words)
                            .with_context(line_index_context)
                            .context("while parsing an edge")
                            .context(context)?;
                    }
                    _ => {
                        return Err(anyhow!(r#"unexpected first word "{first_word}""#))
                            .with_context(line_index_context)
                            .context(context)
                    }
                }
            }
            buffer.clear();
            *line_index.borrow_mut() += 1;
        }
        reader_data.check_connectivity().context(context)?;
        Ok(DecisionDNNF::from_raw_data(
            reader_data.n_vars,
            reader_data.nodes,
            reader_data.edges,
        ))
    }

    fn add_new_node(
        reader_data: &mut D4FormatReaderData,
        first_word: &str,
        mut words: SplitWhitespace,
    ) -> Result<()> {
        let str_index = words.next().ok_or(anyhow!("missing node index"))?;
        let index = usize::from_str(str_index).context("while parsing the node index")?;
        if words.next() != Some("0") {
            return Err(anyhow!("expected 0 as third word"));
        }
        if words.next().is_some() {
            return Err(anyhow!("unexpected content after 0"));
        }
        reader_data.add_new_node(first_word, index)
    }

    fn add_new_edge(
        reader_data: &mut D4FormatReaderData,
        first_word: &str,
        mut words: SplitWhitespace,
    ) -> Result<()> {
        let source_index = usize::from_str(first_word).context("while parsing the source index")?;
        let str_target_index = words.next().ok_or(anyhow!("missing target index"))?;
        let target_index =
            usize::from_str(str_target_index).context("while parsing the target index")?;
        let mut propagated = Vec::new();
        loop {
            match words.next() {
                Some("0") => break,
                Some(w) if isize::from_str(w).is_ok() => {
                    propagated.push(Literal::from(isize::from_str(w).unwrap()));
                }
                Some(w) => return Err(anyhow!(r#"expected a literal, got "{w}""#)),
                None => return Err(anyhow!("missing final 0")),
            }
        }
        if words.next().is_some() {
            return Err(anyhow!("unexpected content after 0"));
        }
        reader_data.add_new_edge(source_index, target_index, propagated)
    }
}

#[derive(Default)]
struct D4FormatReaderData {
    n_vars: usize,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl D4FormatReaderData {
    fn add_new_node(&mut self, label: &str, index: usize) -> Result<()> {
        let expected_n_nodes = 1 + self.nodes.len();
        if index != expected_n_nodes {
            return Err(anyhow!(
                "wrong node index; expected {expected_n_nodes}, got {index}"
            ));
        }
        self.nodes.push(Node::from_str(label)?);
        Ok(())
    }

    fn add_new_edge(
        &mut self,
        source_index: usize,
        target_index: usize,
        mut propagated: Vec<Literal>,
    ) -> Result<()> {
        propagated.sort_unstable_by_key(Literal::var_index);
        propagated.dedup();
        if source_index > self.nodes.len() {
            return Err(anyhow!(
                "wrong source index; max is {}, got {source_index}",
                self.nodes.len()
            ));
        }
        if target_index > self.nodes.len() {
            return Err(anyhow!(
                "wrong target index; max is {}, got {target_index}",
                self.nodes.len()
            ));
        }
        if source_index == target_index {
            return Err(anyhow!("source and target index must be different"));
        }
        self.n_vars = usize::max(
            self.n_vars,
            propagated
                .iter()
                .map(Literal::var_index)
                .max()
                .map(|i| i + 1)
                .unwrap_or_default(),
        );
        let edge = Edge::from_raw_data((target_index - 1).into(), propagated);
        self.edges.push(edge);
        self.nodes[source_index - 1].add_edge((self.edges.len() - 1).into())?;
        Ok(())
    }

    fn check_connectivity(&self) -> Result<()> {
        let mut seen_once = vec![false; self.nodes.len()];
        let mut seen_on_path = vec![false; self.nodes.len()];
        self.check_connectivity_from(&mut seen_once, &mut seen_on_path, 0.into())?;
        match seen_once.iter().position(|b| !b) {
            Some(i) => Err(anyhow!("no path to the node with index {}", i + 1)),
            None => Ok(()),
        }
    }

    fn check_connectivity_from(
        &self,
        seen_once: &mut [bool],
        seen_on_path: &mut [bool],
        node_index: NodeIndex,
    ) -> Result<()> {
        let add_to_seen_on_path = |i: NodeIndex, sop: &mut [bool]| {
            if sop[usize::from(i)] {
                return Err(anyhow!("cycle detected"));
            }
            sop[usize::from(i)] = true;
            Ok(())
        };
        add_to_seen_on_path(node_index, seen_on_path)?;
        seen_once[usize::from(node_index)] = true;
        match &self.nodes[usize::from(node_index)] {
            Node::And(v) | Node::Or(v) => {
                v.iter().try_for_each(|e| {
                    self.check_connectivity_from(
                        seen_once,
                        seen_on_path,
                        self.edges[usize::from(*e)].target(),
                    )
                })?;
            }
            Node::True | Node::False => {}
        }
        seen_on_path[usize::from(node_index)] = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_error(instance: &str, expected_error: &str) {
        match Reader::read(&mut instance.as_bytes()) {
            Ok(_) => panic!(),
            Err(e) => assert_eq!(expected_error, format!("{}", e.root_cause())),
        }
    }

    #[test]
    fn test_node_unexpected_kind() {
        assert_error("n 1 0\n", r#"unexpected first word "n""#);
    }

    #[test]
    fn test_node_wrong_index() {
        assert_error("a 0 0\n", "wrong node index; expected 1, got 0");
    }

    #[test]
    fn test_node_missing_zero() {
        assert_error("a 1\n", "expected 0 as third word");
    }

    #[test]
    fn test_node_not_a_zero() {
        assert_error("a 1 1\n", "expected 0 as third word");
    }

    #[test]
    fn test_node_content_after_zero() {
        assert_error("a 1 0 0\n", "unexpected content after 0");
    }

    #[test]
    fn test_edge_source_equals_target() {
        assert_error(
            "a 1 0\nt 2 0\nf 3 0\n1 1 0",
            "source and target index must be different",
        );
    }

    #[test]
    fn test_edge_unknown_source() {
        assert_error(
            "a 1 0\nt 2 0\nf 3 0\n4 1 0",
            "wrong source index; max is 3, got 4",
        );
    }

    #[test]
    fn test_edge_target_is_not_a_number() {
        assert_error(
            "a 1 0\nt 2 0\nf 3 0\n1 a 0",
            "invalid digit found in string",
        );
    }

    #[test]
    fn test_edge_unknown_target() {
        assert_error(
            "a 1 0\nt 2 0\nf 3 0\n1 4 0",
            "wrong target index; max is 3, got 4",
        );
    }

    #[test]
    fn test_edge_missing_zero() {
        assert_error("a 1 0\nt 2 0\nf 3 0\n1 2", "missing final 0");
    }

    #[test]
    fn test_edge_content_after_zero() {
        assert_error("a 1 0\nt 2 0\nf 3 0\n1 2 0 0", "unexpected content after 0");
    }

    #[test]
    fn test_edge_literal_is_not_a_number() {
        assert_error(
            "a 1 0\nt 2 0\nf 3 0\n1 2 a 0",
            r#"expected a literal, got "a""#,
        );
    }

    #[test]
    fn test_node_unreachable() {
        assert_error("f 1 0\nt 2 0\n", "no path to the node with index 2");
    }

    #[test]
    fn test_node_cycle() {
        assert_error("a 1 0\na 2 0\n1 2 0\n2 1 0\n", "cycle detected");
    }

    #[test]
    fn test_edge_from_true() {
        assert_error(
            "a 1 0\nt 2 0\n2 1 0\n2 1 0\n",
            "cannot add an edge from a leaf node",
        );
    }

    #[test]
    fn test_edge_from_false() {
        assert_error(
            "a 1 0\nf 2 0\n2 1 0\n2 1 0\n",
            "cannot add an edge from a leaf node",
        );
    }

    #[test]
    fn test_ok() {
        let instance =
            "a 1 0\no 2 0\no 3 0\nt 4 0\n1 2 0\n1 3 0\n2 4 -1 0\n2 4 1 0\n3 4 -2 0\n3 4 2 0\n";
        let ddnnf = Reader::read(&mut instance.as_bytes()).unwrap();
        assert_eq!(2, ddnnf.n_vars());
        assert_eq!(4, ddnnf.nodes().as_slice().len());
        assert_eq!(6, ddnnf.edges().as_slice().len());
    }

    #[test]
    fn test_clause() {
        let instance = r"
        o 1 0
        o 2 0
        t 3 0
        2 3 -1 -2 0
        2 3 1 0
        1 2 0";
        let ddnnf = Reader::read(&mut instance.as_bytes()).unwrap();
        assert_eq!(2, ddnnf.n_vars());
        assert_eq!(3, ddnnf.nodes().as_slice().len());
        assert_eq!(3, ddnnf.edges().as_slice().len());
    }

    #[test]
    fn test_empty_instance() {
        let instance = "t 1 0";
        let ddnnf = Reader::read(&mut instance.as_bytes()).unwrap();
        assert_eq!(0, ddnnf.n_vars());
        assert_eq!(1, ddnnf.nodes().as_slice().len());
        assert_eq!(0, ddnnf.edges().as_slice().len());
    }
}
