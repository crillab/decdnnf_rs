mod binary_format;
pub use binary_format::Reader as BinaryReader;
pub use binary_format::Writer as BinaryWriter;

mod c2d_format;
pub use c2d_format::Writer as C2dWriter;

mod d4_format;
pub use d4_format::Reader as D4Reader;
pub use d4_format::Writer as D4Writer;

mod smart_reader;
pub use smart_reader::SmartReader;

use crate::DecisionDNNF;
use anyhow::Result;
use std::io::Read;
use std::io::Write;

/// A trait for object that can write [`DecisionDNNF`]
pub trait DecisionDNNFWriter<W: Write> {
    /// Writes a Decision-DNNF.
    ///
    /// # Errors
    ///
    /// An error is raised when an I/O exception occurs.
    fn write(&self, writer: W, ddnnf: &DecisionDNNF) -> Result<()>;
}

/// A trait for object that can read [`DecisionDNNF`]
pub trait DecisionDNNFReader<R: Read> {
    /// Sets whether the reader must activate its checks.
    ///
    /// The checks depend on the underlying reader.
    /// For some readers, the checks can take a lot of time.
    fn set_do_not_check(&mut self, do_not_check: bool);

    /// Reads a [`DecisionDNNF`].
    ///
    /// # Errors
    ///
    /// An error is raised if the Decision-DNNF encoding is incorrect or when an I/O exception occurs.
    fn read(&self, reader: R) -> Result<DecisionDNNF>;
}
