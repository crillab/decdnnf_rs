#![doc = include_str!("../README.md")]

mod algorithms;
pub use algorithms::DecisionDNNFChecker;
pub use algorithms::DirectAccessEngine;
pub use algorithms::ModelCounter;
pub use algorithms::ModelEnumerator;
pub use algorithms::ModelFinder;
pub use algorithms::OrderedDirectAccessEngine;
pub use algorithms::OrphanFinder;

mod core;
pub use core::Assumptions;
pub use core::DecisionDNNF;
pub use core::Edge;
pub use core::EdgeIndex;
pub use core::FreeVariables;
pub use core::Literal;
pub use core::Node;
pub use core::NodeIndex;
pub use core::OrFreeVariables;

mod io;
pub use io::C2dWriter;
pub use io::D4Reader;
pub use io::D4Writer;
