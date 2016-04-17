//! Support for navigation of a graph that allows modifications to graph
//! topology and full read-write access to graph data.
//!
//! The data structures in this module require a read-write borrow of an
//! underlying graph. As a result, only one handle may be active at any given
//! time.

pub use super::hidden::mutators::{MutChildList, MutEdge, MutNode, MutParentList};
