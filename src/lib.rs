//! A library used to handle Decision-DNNF formulas.

mod algorithms;

mod core;
pub use core::DecisionDNNF;
pub use core::Edge;
pub use core::Literal;
pub use core::Node;

mod io;
pub use io::C2dWriter;
pub use io::D4Reader;
