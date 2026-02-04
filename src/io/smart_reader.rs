use anyhow::{anyhow, Context, Result};
use std::io::{BufReader, Read};

use crate::{BinaryReader, D4Reader, DecisionDNNF, DecisionDNNFReader};

/// A [`DecisionDNNFReader`] that tries to load a [`DecisionDNNF`] by trying to read it with other readers.
#[derive(Default)]
pub struct SmartReader {
    do_not_check: bool,
}

macro_rules! try_for {
    ($reader_type:tt, $content:tt) => {
        let reader = $reader_type::default();
        if let Ok(formula) = reader.read($content.as_slice()) {
            return Ok(formula);
        }
    };
}

impl<R> DecisionDNNFReader<R> for SmartReader
where
    R: Read,
{
    fn set_do_not_check(&mut self, do_not_check: bool) {
        self.do_not_check = do_not_check;
    }

    fn read(&self, reader: R) -> Result<DecisionDNNF> {
        let mut content_reader = BufReader::new(reader);
        let mut content = Vec::new();
        content_reader
            .read_to_end(&mut content)
            .context("while reading content")?;
        try_for!(D4Reader, content);
        try_for!(BinaryReader, content);
        Err(anyhow!("cannot read with any input formats"))
    }
}
